use soroban_sdk::{ Address, Env };

use crate::storage_types::{ DataKey, Index };

pub fn set_admin(e: &Env, admin: Address) {
    e.storage().instance().set(&DataKey::Admin, &admin);
}

pub fn get_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).unwrap()
}

pub fn set_protocol_fee(e: &Env, protocol_fee: u64) {
    e.storage().instance().set(&DataKey::ProtocolFee, &protocol_fee);
}

pub fn get_protocol_fee(e: &Env) -> u64 {
    e.storage().instance().get(&DataKey::ProtocolFee).unwrap()
}

pub fn set_default_oracle(e: &Env, default_oracle: Address) {
    e.storage().instance().set(&DataKey::DefaultOracle, &default_oracle);
}

pub fn get_default_oracle(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::DefaultOracle).unwrap()
}

pub fn set_index(e: &Env, id: u64, index: Index) {
    let i = index.clone();

    e.storage().instance().set(&DataKey::Indexes(id), &index);
    e.storage().instance().set(&DataKey::Index(index.index_address), &i);
}

pub fn get_index_by_address(e: &Env, index_address: Address) -> Option<Index> {
    e.storage().instance().get(&DataKey::Index(index_address))
}

pub fn get_index_by_id(e: &Env, id: u64) -> Index {
    e.storage().instance().get(&DataKey::Indexes(id)).unwrap()
}

pub fn get_indexes_length(e: &Env) -> u64 {
    e.storage().instance().get(&DataKey::IndexesLength).unwrap()
}

pub fn increase_indexes_length(e: &Env) {
    let current_len = get_indexes_length(&e);

    let new_len = current_len + 1;

    e.storage().instance().set(&DataKey::IndexesLength, &new_len);
    // TODO: extend_ttl
}
