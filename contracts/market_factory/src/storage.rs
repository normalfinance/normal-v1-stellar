use normal::constants::{PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD};
use normal::types::market::MarketFactoryConfig;
use soroban_sdk::{contracttype, Address, ConversionError, Env, String, TryFromVal, Val, Vec};

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
    pub(crate) token_a: Address,
    pub(crate) token_b: Address,
}

impl TryFromVal<Env, DataKey> for Val {
    type Error = ConversionError;

    fn try_from_val(_env: &Env, v: &DataKey) -> Result<Self, Self::Error> {
        Ok((*v as u32).into())
    }
}

pub fn save_config(env: &Env, config: MarketFactoryConfig) {
    env.storage().persistent().set(&DataKey::Config, &config);
    env.storage().persistent().extend_ttl(
        &DataKey::Config,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn get_config(env: &Env) -> MarketFactoryConfig {
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
    tuple_market: (&Address, &Address),
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
