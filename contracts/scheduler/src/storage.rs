use normal::{
    constants::{PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD},
    types::OrderDirection,
};
use soroban_decimal::Decimal;
use soroban_sdk::{contracttype, Address, ConversionError, Env, Map, TryFromVal, Val, Vec};

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Config = 1,
    Admin = 2,
    Initialized = 3,
}

impl TryFromVal<Env, DataKey> for Val {
    type Error = ConversionError;

    fn try_from_val(_env: &Env, v: &DataKey) -> Result<Self, Self::Error> {
        Ok((*v as u32).into())
    }
}

// ################################################################
//                             Config
// ################################################################

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub synth_market_factory_address: Address,
    pub index_factory_address: Address,
    pub keepers: Vec<Address>,
    pub protocol_fee_bps: i64,
    pub keeper_fee_bps: i64,

    pub protocol_fees_to_collect: Map<Address, i128>,
}

impl Config {
    pub fn protocol_fee_rate(&self) -> Decimal {
        Decimal::bps(self.protocol_fee_bps)
    }

    pub fn keeper_fee_rate(&self) -> Decimal {
        Decimal::bps(self.keeper_fee_bps)
    }
}

pub fn save_config(env: &Env, config: Config) {
    env.storage().persistent().set(&DataKey::Config, &config);
    env.storage().persistent().extend_ttl(
        &DataKey::Config,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn get_config(env: &Env) -> Config {
    let config = env
        .storage()
        .persistent()
        .get(&DataKey::Config)
        .expect("Config not set");

    env.storage().persistent().extend_ttl(
        &DataKey::Config,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );

    config
}

// ################################################################
//                             Schedules
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
    pub address: Address,
    /// The amount of those tokens
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleParams {
    pub schedule_type: ScheduleType,
    pub quote_asset: Address,
    pub target_contract_address: Address,
    pub base_asset_amount_per_interval: u64,
    pub direction: OrderDirection,
    pub interval_seconds: u64,
    pub min_price: Option<u32>,
    pub max_price: Option<u32>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub schedule_type: ScheduleType,
    pub quote_asset: Address,
    pub target_contract_address: Address,
    pub base_asset_amount_per_interval: u64,
    pub direction: OrderDirection,
    pub interval_seconds: u64,
    pub total_orders: u32,
    pub min_price: Option<u32>,
    pub max_price: Option<u32>,
    pub executed_orders: u32,
    pub total_executed: i128,
    pub total_fees_paid: u64,
    pub last_updated_ts: u64,
    pub last_order_ts: u64,
    /// The timestamp when the schedule was made
    pub schedule_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleInfo {
    /// Map of token address to deposit balance
    pub balances: Map<Address, i128>,
    /// Vec of schedules sorted by schedule timestamp
    pub schedules: Vec<Schedule>,
}

pub fn get_schedules(env: &Env, key: &Address) -> ScheduleInfo {
    let schedule_info = match env.storage().persistent().get::<_, ScheduleInfo>(key) {
        Some(stake) => stake,
        None => ScheduleInfo {
            balances: Map::new(env),
            schedules: Vec::new(env),
        },
    };
    env.storage().persistent().has(&key).then(|| {
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );
    });

    schedule_info
}

pub fn save_schedules(env: &Env, key: &Address, schedule_info: &ScheduleInfo) {
    env.storage().persistent().set(key, schedule_info);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

// ################################################################
//                             Keepers
// ################################################################

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeeperInfo {
    /// List of assets and amounts owed in fees
    pub fees_owed: Map<Address, i128>,
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

pub fn get_keeper(env: &Env, key: &Address) -> KeeperInfo {
    let keeper_info = match env.storage().persistent().get::<_, KeeperInfo>(key) {
        Some(info) => info,
        None => KeeperInfo {
            fees_owed: Map::new(env),
            total_fees: 0u64,
            last_fee_collection_time: 0u64,
            total_orders: 0u64,
            total_order_amount: 0u128,
            last_order_time: 0u64,
        },
    };
    env.storage().persistent().has(&key).then(|| {
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );
    });

    keeper_info
}

pub fn save_keeper(env: &Env, key: &Address, keeper_info: &KeeperInfo) {
    env.storage().persistent().set(key, keeper_info);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

// ################################################################

pub mod utils {
    use normal::error::ErrorCode;
    use soroban_sdk::{log, panic_with_error};

    use crate::token_contract;

    use super::*;

    pub fn transfer_token(env: &Env, asset: &Address, from: &Address, to: &Address, amount: i128) {
        let token_client = token_contract::Client::new(env, asset);
        token_client.transfer(from, to, &amount);
    }

    pub fn check_nonnegative_amount(amount: i128) {
        if amount < 0 {
            panic!("negative amount is not allowed: {}", amount)
        }
    }

    pub fn is_admin(env: &Env, sender: Address) {
        let admin = get_admin(env);
        if admin != sender {
            log!(&env, "Scheduler: You are not authorized!");
            panic_with_error!(&env, ErrorCode::NotAuthorized);
        }
    }

    pub fn is_initialized(e: &Env) -> bool {
        e.storage()
            .instance()
            .get(&DataKey::Initialized)
            .unwrap_or(false)
    }

    pub fn set_initialized(e: &Env) {
        e.storage().instance().set(&DataKey::Initialized, &true);
        e.storage()
            .instance()
            .extend_ttl(PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
    }

    pub fn save_admin(e: &Env, address: &Address) {
        e.storage().persistent().set(&DataKey::Admin, address);
        e.storage().persistent().extend_ttl(
            &DataKey::Admin,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );
    }

    pub fn get_admin(e: &Env) -> Address {
        let admin = e.storage().persistent().get(&DataKey::Admin).unwrap();
        e.storage().persistent().extend_ttl(
            &DataKey::Admin,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );

        admin
    }
}
