use normal::{
    constants::{
        INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT,
        PERSISTENT_LIFETIME_THRESHOLD,
    },
    error::ErrorCode,
};
use soroban_sdk::{
    contracttype, log, panic_with_error, symbol_short, Address, BytesN, ConversionError, Env,
    String, Symbol, TryFromVal, Val, Vec,
};

pub const ADMIN: Symbol = symbol_short!("ADMIN");

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Config = 1,
    IndexVec = 2,
    Initialized = 3,
}

#[derive(Clone)]
#[contracttype]
pub struct IndexTupleKey {
    pub(crate) symbol: String,
    pub(crate) name: String,
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
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum Operation {
    CreateIndex,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub admin: Address,
    pub index_token_wasm_hash: BytesN<32>,
    /// Tokens allowed to mint index tokens
    pub quote_token_whitelist: Vec<Address>,
    pub paused_operations: Vec<Operation>,
    pub max_manager_fee_bps: i64,
    pub protocol_fee_bps: i64,
    pub default_oracle: Address,
}

/// This struct is used to return a query result with the ...
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexResponse {
    // ...
    pub x: bool,
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
        panic_with_error!(&env, ErrorCode::AdminNotSet)
    });

    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

    admin_addr
}

pub fn get_index_vec(env: &Env) -> Vec<Address> {
    let index_vec = env
        .storage()
        .persistent()
        .get(&DataKey::IndexVec)
        .expect("Index Factory: get_index_vec: Liquidity Pool vector not found");

    env.storage().persistent().extend_ttl(
        &DataKey::IndexVec,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );

    index_vec
}

pub fn save_index_vec(env: &Env, index_info: Vec<Address>) {
    env.storage()
        .persistent()
        .set(&DataKey::IndexVec, &index_info);
    env.storage().persistent().extend_ttl(
        &DataKey::IndexVec,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn save_index_vec_with_tuple_as_key(
    env: &Env,
    tuple_index: (&String, &String),
    index_address: &Address,
) {
    env.storage().persistent().set(
        &(IndexTupleKey {
            symbol: tuple_index.0.clone(),
            name: tuple_index.1.clone(),
        }),
        &index_address,
    );

    env.storage().persistent().extend_ttl(
        &(IndexTupleKey {
            symbol: tuple_index.0.clone(),
            name: tuple_index.1.clone(),
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
