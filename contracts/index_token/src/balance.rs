use crate::storage_types::{ DataKey, BALANCE_BUMP_AMOUNT, BALANCE_LIFETIME_THRESHOLD };
use soroban_sdk::{ Address, Env };

pub fn read_balance(env: &Env, addr: Address) -> i128 {
    let key = DataKey::Balance(addr);
    if let Some(balance) = env.storage().persistent().get::<DataKey, i128>(&key) {
        env.storage().persistent().extend_ttl(&key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
        balance
    } else {
        0
    }
}

fn write_balance(env: &Env, addr: Address, amount: i128) {
    let key = DataKey::Balance(addr);
    env.storage().persistent().set(&key, &amount);
    env.storage().persistent().extend_ttl(&key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
}

pub fn receive_balance(env: &Env, addr: Address, amount: i128) {
    let balance = read_balance(env, addr.clone());
    write_balance(env, addr, balance + amount);
}

pub fn spend_balance(env: &Env, addr: Address, amount: i128) {
    let balance = read_balance(env, addr.clone());
    if balance < amount {
        panic!("insufficient balance");
    }
    write_balance(env, addr, balance - amount);
}

//  ====

pub fn read_index_contract(env: &Env) -> Address {
    let key = DataKey::IndexContract;
    env.storage().instance().get(&key).unwrap()
}

pub fn write_index_contract(env: &Env, contract: &Address) {
    let key = DataKey::IndexContract;
    env.storage().instance().set(&key, contract);
}

//  ====

pub fn read_last_transfer(env: &Env, addr: Address) -> (u64, i128) {
    let key = DataKey::LastTransfer(addr);
    if let Some(balance) = env.storage().persistent().get::<DataKey, (u64, i128)>(&key) {
        env.storage().persistent().extend_ttl(&key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
        balance
    } else {
        0
    }
}

fn write_last_transfer(env: &Env, addr: Address, amount: i128) {
    let key = DataKey::LastTransfer(addr);
    env.storage().persistent().set(&key, &amount);
    env.storage().persistent().extend_ttl(&key, BALANCE_LIFETIME_THRESHOLD, BALANCE_BUMP_AMOUNT);
}