use normal::{
    constants::{PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD},
    oracle::OracleSource,
    types::IndexAsset,
};
use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, Symbol, Vec};

pub const XLM: Symbol = symbol_short!("XLM");
pub const USD: Symbol = symbol_short!("USD");

#[derive(Clone)]
#[contracttype]
pub struct AllowanceDataKey {
    pub from: Address,
    pub spender: Address,
}

#[contracttype]
pub struct AllowanceValue {
    pub amount: i128,
    pub expiration_ledger: u32,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Index,
    Factory,
    Allowance(AllowanceDataKey),
    Balance(Address),
    State(Address),
    Admin,
    Initialized,
}

// ################################################################
//                             Index
// ################################################################

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum IndexOperation {
    Mint,
    Redeem,
    Rebalance,
    Update,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Index {
    /// Token used to mint/redeem
    pub quote_token: Address,
    /// Oracle for fetching the quote asset price
    pub oracle: Address,
    /// Oracle type
    pub oracle_source: OracleSource,
    /// Private indexes are mutable and can only be minted by the admin and whitelist
    /// Pubilic indexes are immutabel and can be minted by anyone
    pub is_public: bool,
    /// List of operations temporarily paused
    pub paused_operations: Vec<IndexOperation>,
    /// A custom annual fee set by the admin
    pub manager_fee_bps: i64,
    /// List of accounts explicitly allowed to mint the index
    pub whitelist: Vec<Address>,
    /// List of accounts blocked from minting the index
    pub blacklist: Vec<Address>,
    /// The Net Asset Value (NAV) at the inception of the index - what the creator deposits (e.g. $1,000)
    pub base_nav: i128,
    /// The price assigned to the index at inception (e.g. $100)
    pub initial_price: i128,
    pub component_balances: Map<Address, i128>, // Token address > balance
    /// The ts when the component balances were last updated
    pub component_balance_update_ts: u64,
    pub component_assets: Vec<IndexAsset>,
    /// Minimum amount of time that must pass before the index can be rebalanced again
    pub rebalance_threshold: u64,
    /// The ts when the index was last rebalanced
    pub rebalance_ts: u64,
    /// The ts when the index was last updated (any property)
    pub last_updated_ts: u64,
    /// Metrics
    pub total_fees: i128,
    pub total_mints: i128,
    pub total_redemptions: i128,
}

impl Index {
    pub fn can_invest(&self, account: Address) -> bool {
        self.whitelist.contains(&account)
    }

    pub fn can_rebalance(&self, now: u64) -> bool {
        self.time_since_last_rebalance(now) >= self.rebalance_threshold
    }

    pub fn time_since_last_rebalance(&self, now: u64) -> u64 {
        now - self.rebalance_ts
    }
}

pub fn save_index(env: &Env, index: Index) {
    env.storage().persistent().set(&DataKey::Index, &index);
    env.storage().persistent().extend_ttl(
        &DataKey::Index,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn get_index(env: &Env) -> Index {
    let index = env
        .storage()
        .persistent()
        .get(&DataKey::Index)
        .expect("Config not set");

    env.storage().persistent().extend_ttl(
        &DataKey::Index,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );

    index
}

// ################################################################
//                         Last Transfer
// ################################################################

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LastTransfer {
    pub balance: i128,
    pub ts: u64,
}

pub fn get_last_transfer(env: &Env, key: &Address) -> LastTransfer {
    let last_transfer = match env.storage().persistent().get::<_, LastTransfer>(key) {
        Some(transfer) => transfer,
        None => LastTransfer {
            balance: 0i128,
            ts: 0u64, // current_time
        },
    };
    env.storage().persistent().has(&key).then(|| {
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );
    });

    last_transfer
}

pub fn save_last_transfer(env: &Env, key: &Address, last_transfer: &LastTransfer) {
    env.storage().persistent().set(key, last_transfer);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

// ################################################################

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Swap {
    pub ask_asset: Address,
    pub offer_asset: Address,
    pub ask_asset_min_amount: Option<i128>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferWithFees {
    pub protocol_fee_amount: i128,
    pub manager_fee_amount: i128,
    pub total_fees: i128,
    pub net_amount: i128,
}

// ################################################################

pub mod utils {
    use normal::error::ErrorCode;
    use soroban_sdk::{log, panic_with_error};

    use crate::token_contract;

    use super::*;

    pub fn get_token_balance(env: &Env, token: &Address, account: &Address) -> i128 {
        token_contract::Client::new(env, token).balance(account)
    }

    pub fn transfer_token(env: &Env, token: &Address, from: &Address, to: &Address, amount: i128) {
        let token_client = token_contract::Client::new(env, token);
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
            log!(&env, "Index Token: You are not authorized!");
            panic_with_error!(&env, ErrorCode::NotAuthorized);
        }
    }

    pub fn is_governor(_env: &Env, _sender: Address) {
        // let factory_client = index_factory_contract::Client::new(&env, &read_factory(&env));
        // let config = factory_client.query_config();

        // if config.governor != sender {
        //     log!(&env, "Index Token: You are not authorized!");
        //     panic_with_error!(&env, ErrorCode::NotAuthorized);
        // }
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

    pub fn save_factory(e: &Env, factory: &Address) {
        e.storage().persistent().set(&DataKey::Factory, factory);
        e.storage().persistent().extend_ttl(
            &DataKey::Factory,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );
    }

    pub fn get_factory(e: &Env) -> Address {
        let factory = e.storage().persistent().get(&DataKey::Factory).unwrap();
        e.storage().persistent().extend_ttl(
            &DataKey::Factory,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );

        factory
    }
}
