use soroban_sdk::{ Address, Env };

use crate::storage_types::{ DataKey, Stake };

fn get_token_a(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::TokenA).unwrap()
}
