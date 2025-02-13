use normal::constants::{PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD, PRICE_PRECISION};
use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, ConversionError, Env, TryFromVal, Val};

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Market = 1,
    FactoryAddr = 2,
    Admin = 3,
    Initialized = 4,
}

impl TryFromVal<Env, DataKey> for Val {
    type Error = ConversionError;

    fn try_from_val(_env: &Env, v: &DataKey) -> Result<Self, Self::Error> {
        Ok((*v as u32).into())
    }
}

pub mod utils {
    use normal::{error::ErrorCode, types::market::MarketFactoryConfig};
    use soroban_sdk::{log, panic_with_error, String, Symbol, Vec};

    use crate::{errors::Errors, token_contract};

    use super::*;

    pub fn check_nonnegative_amount(amount: i128) {
        if amount < 0 {
            panic!("negative amount is not allowed: {}", amount)
        }
    }

    pub fn is_admin(env: &Env, sender: &Address, with_auth: bool) {
        let admin = get_admin(env);
        if admin != *sender {
            log!(&env, "Market: You are not authorized!");
            panic_with_error!(&env, ErrorCode::NotAuthorized);
        }
        if with_auth {
            sender.require_auth();
        }
    }

    pub fn validate_governor(env: &Env, sender: &Address) {
        let factory_config: MarketFactoryConfig = env.invoke_contract(
            &get_factory(&env),
            &Symbol::new(&env, "query_config"),
            Vec::new(&env),
        );
        if factory_config.governor != *sender {
            log!(&env, "Market: You are not authorized!");
            panic_with_error!(&env, ErrorCode::NotAuthorized);
        }
    }

    pub fn validate_super_keeper(env: &Env, address: &Address) {
        let factory_config: MarketFactoryConfig = env.invoke_contract(
            &get_factory(env),
            &Symbol::new(&env, "query_config"),
            Vec::new(env),
        );

        if !factory_config.super_keepers.contains(*address) {
            log!(env, "Market: You are not authorized!");
            panic_with_error!(env, Errors::NotAuthorized);
        }
    }

    pub fn get_balance(env: &Env, contract: &Address) -> i128 {
        token_contract::Client::new(env, contract).balance(&env.current_contract_address())
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

    pub fn save_factory(env: &Env, address: &Address) {
        env.storage()
            .persistent()
            .set(&DataKey::FactoryAddr, address);
        env.storage().persistent().extend_ttl(
            &DataKey::FactoryAddr,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );
    }

    pub fn get_factory(env: &Env) -> Address {
        let factory = env
            .storage()
            .persistent()
            .get(&DataKey::FactoryAddr)
            .unwrap();
        env.storage().persistent().extend_ttl(
            &DataKey::FactoryAddr,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );

        factory
    }
}
