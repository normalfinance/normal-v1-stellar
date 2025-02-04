use normal::constants::{PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD, PRICE_PRECISION};
use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, ConversionError, Env, TryFromVal, Val};

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Market = 1,
    Admin = 2,
    Initialized = 3,
}

impl TryFromVal<Env, DataKey> for Val {
    type Error = ConversionError;

    fn try_from_val(_env: &Env, v: &DataKey) -> Result<Self, Self::Error> {
        Ok((*v as u32).into())
    }
}

pub mod utils {
    use normal::error::ErrorCode;
    use soroban_sdk::{log, panic_with_error, String};

    use crate::token_contract;

    use super::*;

    pub fn check_nonnegative_amount(amount: i128) {
        if amount < 0 {
            panic!("negative amount is not allowed: {}", amount)
        }
    }

    pub fn is_admin(env: &Env, sender: &Address, with_auth: bool) {
        let admin = get_admin(env);
        if admin != *sender {
            log!(&env, "Index Token: You are not authorized!");
            panic_with_error!(&env, ErrorCode::NotAuthorized);
        }
        if with_auth {
            sender.require_auth();
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

    #[allow(clippy::too_many_arguments)]
    pub fn deploy_synth_token_contract(
        env: &Env,
        token_wasm_hash: BytesN<32>,
        token_a: &Address,
        admin: Address,
        decimals: u32,
        name: String,
        symbol: String,
    ) -> Address {
        let mut salt = Bytes::new(env);
        salt.append(&token_a.clone().to_xdr(env));
        let salt = env.crypto().sha256(&salt);
        env.deployer()
            .with_current_contract(salt)
            .deploy_v2(token_wasm_hash, (admin, decimals, name, symbol))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn deploy_lp_token_contract(
        env: &Env,
        token_wasm_hash: BytesN<32>,
        token_a: &Address,
        token_b: &Address,
        admin: Address,
        decimals: u32,
        name: String,
        symbol: String,
    ) -> Address {
        let mut salt = Bytes::new(env);
        salt.append(&token_a.clone().to_xdr(env));
        salt.append(&token_b.clone().to_xdr(env));
        let salt = env.crypto().sha256(&salt);
        env.deployer()
            .with_current_contract(salt)
            .deploy_v2(token_wasm_hash, (admin, decimals, name, symbol))
    }

    pub fn get_balance(env: &Env, contract: &Address) -> i128 {
        token_contract::Client::new(env, contract).balance(&e.current_contract_address())
    }

    pub fn is_initialized(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Initialized)
            .unwrap_or(false)
    }

    pub fn set_initialized(env: &Env) {
        env.storage().persistent().set(&DataKey::Initialized, &true);
        env.storage().persistent().extend_ttl(
            &DataKey::Initialized,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );
    }

    pub fn save_admin(env: &Env, address: &Address) {
        env.storage().persistent().set(&DataKey::Admin, address);
        env.storage().persistent().extend_ttl(
            &DataKey::Admin,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );
    }

    pub fn get_admin(env: &Env) -> Address {
        let admin = env.storage().persistent().get(&DataKey::Admin).unwrap();
        env.storage().persistent().extend_ttl(
            &DataKey::Admin,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );

        admin
    }
}
