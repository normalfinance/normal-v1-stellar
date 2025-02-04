use normal::constants::{PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD};
use normal::oracle::OracleGuardRails;
use soroban_sdk::{
    contracttype, symbol_short, Address, BytesN, ConversionError, Env, Symbol, TryFromVal, Val, Vec,
};

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Config = 1,
    MarketVec = 2,
    Initialized = 3,
}

#[derive(Clone)]
#[contracttype]
pub struct MarketTupleKey {
    pub(crate) token_a: Symbol,
    pub(crate) token_b: Symbol,
}

impl TryFromVal<Env, DataKey> for Val {
    type Error = ConversionError;

    fn try_from_val(_env: &Env, v: &DataKey) -> Result<Self, Self::Error> {
        Ok((*v as u32).into())
    }
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub admin: Address,
    pub governor: Address,
    pub market_wasm_hash: BytesN<32>,
    pub token_wasm_hash: BytesN<32>,
    pub emergency_oracles: Vec<Address>,
    pub oracle_guard_rails: OracleGuardRails,
    // pub lp_token_decimals: u32,
}

/// This struct is used to return a query result with the total amount of LP tokens and assets in a specific pool.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarketResponse {
    /// The asset A in the pool together with asset amounts
    pub asset_a: Asset,
    /// The asset B in the pool together with asset amounts
    pub asset_b: Asset,
    /// The total amount of LP tokens currently issued
    pub asset_lp_share: Asset,
    /// The address of the Stake contract for the liquidity pool
    pub stake_address: Address,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarketInfo {
    pub market_address: Address,
    pub market_response: MarketResponse,
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

pub fn get_market_vec(env: &Env) -> Vec<Address> {
    let market_vec = env
        .storage()
        .persistent()
        .get(&DataKey::MarketVec)
        .expect("Factory: get_market_vec: Market vector not found");

    env.storage().persistent().extend_ttl(
        &DataKey::MarketVec,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );

    market_vec
}

pub fn save_market_vec(env: &Env, market_info: Vec<Address>) {
    env.storage()
        .persistent()
        .set(&DataKey::MarketVec, &market_info);
    env.storage().persistent().extend_ttl(
        &DataKey::MarketVec,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn save_market_vec_with_tuple_as_key(
    env: &Env,
    tuple_market: (&Symbol, &Symbol),
    market_address: &Address,
) {
    env.storage().persistent().set(
        &(MarketTupleKey {
            token_a: tuple_market.0.clone(),
            token_b: tuple_market.1.clone(),
        }),
        &market_address,
    );

    env.storage().persistent().extend_ttl(
        &(MarketTupleKey {
            token_a: tuple_market.0.clone(),
            token_b: tuple_market.1.clone(),
        }),
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

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
