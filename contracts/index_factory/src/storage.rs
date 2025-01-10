use phoenix::ttl::{
    INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT,
    PERSISTENT_LIFETIME_THRESHOLD,
};
use soroban_sdk::{
    contracttype, log, panic_with_error, symbol_short, Address, BytesN, ConversionError, Env,
    Symbol, TryFromVal, Val, Vec,
};

use crate::error::ContractError;

pub const ADMIN: Symbol = symbol_short!("ADMIN");

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Config = 1,
    IndexVec = 2,
    Initialized = 3,
}

// #[derive(Clone)]
// #[contracttype]
// pub struct PairTupleKey {
//     pub(crate) token_a: Address,
//     pub(crate) token_b: Address,
// }

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
    pub index_wasm_hash: BytesN<32>,
    pub index_token_wasm_hash: BytesN<32>,
    pub paused_operations: u8,
    pub max_manager_fee_bps: i64,
    pub protocol_fee_bps: i64,
    pub default_oracle: Address,
}

/// This struct is used to return a query result with the ...
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexResponse {
    // ...
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexInfo {
    pub index_address: Address,
    pub index_response: IndexResponse,
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

pub fn _save_admin(env: &Env, admin_addr: Address) {
    env.storage().instance().set(&ADMIN, &admin_addr);

    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

pub fn _get_admin(env: &Env) -> Address {
    let admin_addr = env.storage().instance().get(&ADMIN).unwrap_or_else(|| {
        log!(env, "Factory: Admin not set");
        panic_with_error!(&env, ContractError::AdminNotSet)
    });

    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

    admin_addr
}

pub fn get_lp_vec(env: &Env) -> Vec<Address> {
    let lp_vec = env
        .storage()
        .persistent()
        .get(&DataKey::LpVec)
        .expect("Factory: get_lp_vec: Liquidity Pool vector not found");

    env.storage().persistent().extend_ttl(
        &DataKey::LpVec,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );

    lp_vec
}

pub fn save_lp_vec(env: &Env, lp_info: Vec<Address>) {
    env.storage().persistent().set(&DataKey::LpVec, &lp_info);
    env.storage().persistent().extend_ttl(
        &DataKey::LpVec,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn save_lp_vec_with_tuple_as_key(
    env: &Env,
    tuple_pool: (&Address, &Address),
    lp_address: &Address,
) {
    env.storage().persistent().set(
        &(PairTupleKey {
            token_a: tuple_pool.0.clone(),
            token_b: tuple_pool.1.clone(),
        }),
        &lp_address,
    );

    env.storage().persistent().extend_ttl(
        &(PairTupleKey {
            token_a: tuple_pool.0.clone(),
            token_b: tuple_pool.1.clone(),
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
