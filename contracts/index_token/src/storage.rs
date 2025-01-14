use normal::{
    constants::{PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD},
    oracle::OracleSource,
    types::IndexAsset,
};
use soroban_sdk::{contracttype, Address, Env, Map, Vec};

// ################################################################

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
    LastTransfer(Address),
    Admin,
    Initialized,
}

// ################################################################
//                             INDEX
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
    pub quote_oracle: Address,
    /// Oracle type
    pub quote_oracle_source: OracleSource,
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
    pub base_nav: i64,
    /// The price assigned to the index at inception (e.g. $100)
    pub initial_price: i32,
    ///
    pub component_balances: Map<Address, i128>, // Token address > balance
    /// The ts when the component balances were last updated
    pub component_balance_update_ts: u64,
    ///
    pub component_assets: Vec<IndexAsset>,
    /// Minimum amount of time that must pass before the index can be rebalanced again
    pub rebalance_threshold: i64,
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
    pub fn can_invest(&self, env: &Env, account: Address) -> bool {
        self.whitelist.contains(&account)
    }

    pub fn can_rebalance(&self, now: u64) -> bool {
        self.time_since_last_rebalance(now) >= self.rebalance_threshold
    }

    pub fn time_since_last_rebalance(&self, now: u64) -> u64 {
        now - self.rebalance_ts
    }
}

pub fn get_index(env: &Env) -> Index {
    let key = DataKey::Index;
    let index = env
        .storage()
        .persistent()
        .get(&key)
        .expect("Index: Index not set");
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );

    index
}

pub fn save_index(env: &Env, index: Index) {
    let key = DataKey::Index;
    env.storage().persistent().set(&key, &index);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

// ################################################################

pub fn read_factory(env: &Env) -> Address {
    let key = DataKey::Factory;
    env.storage().instance().get(&key).unwrap()
}

pub fn write_factory(env: &Env, id: &Address) {
    let key = DataKey::Factory;
    env.storage().instance().set(&key, id);
}

// ################################################################

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LastTransfer {
    pub ts: u64,
    pub balance: i128,
}

pub fn get_last_transfer(env: &Env, key: &Address) -> LastTransfer {
    let last_transfer = match env.storage().persistent().get::<_, LastTransfer>(key) {
        Some(transfer) => transfer,
        None => LastTransfer {
            ts: 0u64, // current_time
            balance: 0i128,
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

pub fn is_initialized(e: &Env) -> bool {
    e.storage()
        .persistent()
        .get(&DataKey::Initialized)
        .unwrap_or(false)
}

pub fn set_initialized(e: &Env) {
    e.storage().persistent().set(&DataKey::Initialized, &true);

    e.storage().persistent().extend_ttl(
        &DataKey::Initialized,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn read_administrator(env: &Env) -> Address {
    let key = DataKey::Admin;
    env.storage().instance().get(&key).unwrap()
}

pub fn write_administrator(env: &Env, id: &Address) {
    let key = DataKey::Admin;
    env.storage().instance().set(&key, id);
}
