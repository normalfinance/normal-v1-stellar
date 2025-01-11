use crate::storage_types::{ DataKey, BALANCE_BUMP_AMOUNT, BALANCE_LIFETIME_THRESHOLD };
use soroban_sdk::{ Address, Env };

pub fn read_balance(e: &Env, addr: Address) -> i128 {
    let key = DataKey::Balance(addr);
    if let Some(balance) = e.storage().persistent().get::<DataKey, i128>(&key) {
        e.storage().persistent().extend_ttl(&key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
        balance
    } else {
        0
    }
}

fn write_balance(e: &Env, addr: Address, amount: i128) {
    let key = DataKey::Balance(addr);
    e.storage().persistent().set(&key, &amount);
    e.storage().persistent().extend_ttl(&key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
}

pub fn receive_balance(e: &Env, addr: Address, amount: i128) {
    let balance = read_balance(e, addr.clone());
    write_balance(e, addr, balance + amount);
}

pub fn spend_balance(e: &Env, addr: Address, amount: i128) {
    let balance = read_balance(e, addr.clone());
    if balance < amount {
        panic!("insufficient balance");
    }
    write_balance(e, addr, balance - amount);
}

//  ====

pub fn read_index_contract(e: &Env) -> Address {
    let key = DataKey::IndexContract;
    e.storage().instance().get(&key).unwrap()
}

pub fn write_index_contract(e: &Env, contract: &Address) {
    let key = DataKey::IndexContract;
    e.storage().instance().set(&key, contract);
}

//  ====

pub fn read_last_transfer(e: &Env, addr: Address) -> (u64, i128) {
    let key = DataKey::LastTransfer(addr);
    if let Some(balance) = e.storage().persistent().get::<DataKey, (u64, i128)>(&key) {
        e.storage().persistent().extend_ttl(&key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
        balance
    } else {
        0
    }
}

fn write_last_transfer(e: &Env, addr: Address, amount: i128) {
    let key = DataKey::LastTransfer(addr);
    e.storage().persistent().set(&key, &amount);
    e.storage().persistent().extend_ttl(&key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
}