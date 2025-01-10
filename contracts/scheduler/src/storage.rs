use normal::{
    ttl::{ PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD },
    types::OrderDirection,
};
use soroban_decimal::Decimal;
use soroban_sdk::{
    contracttype,
    symbol_short,
    Address,
    ConversionError,
    Env,
    Symbol,
    TryFromVal,
    Val,
    Vec,
};

pub const ADMIN: Symbol = symbol_short!("ADMIN");

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Balance(Address, Option<Address>), // Tracks balances: (user, asset). `None` for XLM.
    Config,
    Admin,
    Initialized,
}

impl TryFromVal<Env, DataKey> for Val {
    type Error = ConversionError;

    fn try_from_val(_env: &Env, v: &DataKey) -> Result<Self, Self::Error> {
        Ok((*v as u32).into())
    }
}

// ################################################################

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub admin: Address,
    pub synth_market_factory_address: Address,
    pub index_factory_address: Address,
    pub keeper_accounts: Vec<Address>,
    pub protocol_fee_bps: i64,
    pub keeper_fee_bps: i64,
}

impl Config {
    pub fn protocol_fee_rate(&self) -> Decimal {
        Decimal::bps(self.protocol_fee_bps)
    }

    pub fn keeper_fee_rate(&self) -> Decimal {
        Decimal::bps(self.keeper_fee_bps)
    }
}

const CONFIG: Symbol = symbol_short!("CONFIG");

pub fn save_config(env: &Env, config: Config) {
    env.storage().persistent().set(&DataKey::Config, &config);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Config, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

pub fn get_config(env: &Env) -> Config {
    let config = env.storage().persistent().get(&DataKey::Config).expect("Config not set");

    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Config, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

    config
}

// ################################################################

#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ScheduleType {
    Asset = 0,
    Index = 1,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    /// Address of the asset
    pub address: Option<Address>, // `None` for XLM
    /// The amount of those tokens
    pub amount: u128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub schedule_type: ScheduleType,
    pub target_contract_address: Address,
    pub base_asset_amount_per_interval: u64,
    pub direction: OrderDirection,
    pub active: bool,
    pub interval_seconds: u64,
    pub total_orders: u32,
    pub min_price: Option<u32>,
    pub max_price: Option<u32>,
    pub executed_orders: u32,
    pub total_executed: u64,
    pub total_fees_paid: u64,
    pub last_updated_ts: u64,
    pub last_order_ts: u64,
    /// The timestamp when the schedule was made
    pub schedule_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchedulingInfo {
    /// Vec of schedules sorted by schedule timestamp
    pub schedules: Vec<Schedule>,

    /// Total amount of staked tokens
    pub total_deposits: i128,
    /// Total amount of staked tokens
    pub total_withdrawals: i128,
    // TODO: move to a state-like struct
    // /// List of assets and amounts owed in fees
    // pub protocol_fees_owed: Vec<Asset>,
    // /// Total fees earned by the protocol
    // pub total_fees: u64,
    // /// Last time when fees were collected
    // pub last_fee_collection_time: u64,
}

pub fn get_schedules(env: &Env, key: &Address) -> SchedulingInfo {
    let scheduling_info = match env.storage().persistent().get::<_, SchedulingInfo>(key) {
        Some(stake) => stake,
        None =>
            SchedulingInfo {
                schedules: Vec::new(env),
                total_deposits: 0i128,
                total_withdrawals: 0i128,
            },
    };
    env.storage()
        .persistent()
        .has(&key)
        .then(|| {
            env.storage()
                .persistent()
                .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        });

    scheduling_info
}

pub fn save_schedules(env: &Env, key: &Address, scheduling_info: &SchedulingInfo) {
    env.storage().persistent().set(key, scheduling_info);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

// ################################################################

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeeperInfo {
    /// List of assets and amounts owed in fees
    pub fees_owed: Vec<Asset>,
    /// Total fees earned by the keeper
    pub total_fees: u64,
    /// Last time when keeper collected fees
    pub last_fee_collection_time: u64,
    /// Total number of orders executed
    pub total_orders: u64,
    /// Total amount of executed orders
    pub total_order_amount: u128,
    /// Last time when keeper executed an order
    pub last_order_time: u64,
}

pub fn get_keeper_info(env: &Env, key: &Address) -> KeeperInfo {
    let keeper_info = match env.storage().persistent().get::<_, KeeperInfo>(key) {
        Some(info) => info,
        None =>
            KeeperInfo {
                fees_owed: Vec::new(env),
                total_fees: 0u64,
                last_fee_collection_time: 0u64,
                total_orders: 0u64,
                total_order_amount: 0u128,
                last_order_time: 0u64,
            },
    };
    env.storage()
        .persistent()
        .has(&key)
        .then(|| {
            env.storage()
                .persistent()
                .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        });

    keeper_info
}

pub fn save_keeper_info(env: &Env, key: &Address, keeper_info: &KeeperInfo) {
    env.storage().persistent().set(key, keeper_info);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

// ################################################################

pub mod utils {
    use normal::ttl::{ INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD };
    use soroban_sdk::{ log, panic_with_error };

    use crate::{ errors::ErrorCode, token_contract };

    use super::*;

    pub fn transfer_tokens(
        env: &Env,
        asset: Option<Address>,
        from: &Address,
        to: &Address,
        amount: u128
    ) {
        match asset {
            // Handle XLM
            None => {
                env.pay(&from, &to, amount);
            }
            // Handle tokens
            Some(token_address) => {
                let token_client = token_contract::Client::new(&env, &asset);
                token_client.transfer(&from, &to, &amount);
            }
        }
    }

    pub fn _save_admin(env: &Env, admin_addr: Address) {
        env.storage().instance().set(&ADMIN, &admin_addr);

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
    }

    pub fn _get_admin(env: &Env) -> Address {
        let admin_addr = env
            .storage()
            .instance()
            .get(&ADMIN)
            .unwrap_or_else(|| {
                log!(env, "Factory: Admin not set");
                panic_with_error!(&env, ErrorCode::AdminNotSet)
            });

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        admin_addr
    }

    pub fn is_initialized(e: &Env) -> bool {
        e.storage().persistent().get(&DataKey::Initialized).unwrap_or(false)
    }

    pub fn set_initialized(e: &Env) {
        e.storage().persistent().set(&DataKey::Initialized, &true);
        e.storage()
            .persistent()
            .extend_ttl(
                &DataKey::Initialized,
                PERSISTENT_LIFETIME_THRESHOLD,
                PERSISTENT_BUMP_AMOUNT
            );
    }
}
