use soroban_sdk::{ Address, Env };

use crate::storage_types::{ DataKey, Stake };

// Admin

pub fn set_admin(e: &Env, admin: Address) {
    e.storage().instance().set(&DataKey::Admin, &admin);
}

pub fn get_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).unwrap()
}

// Stake

// pub fn get_stake_by_address(e: &Env, authority: Address) -> Option<Stake> {
//     e.storage().instance().get(&DataKey::Stake(authority))
// }

// Max Insurance
pub fn set_max_insurance(e: &Env, max_insurance: u64) {
    e.storage().instance().set(&DataKey::MaxInsurance, &max_insurance);
}

pub fn get_max_insurance(e: &Env) -> u64 {
    e.storage().instance().get(&DataKey::MaxInsurance).unwrap()
}

// Unstaking period

pub fn set_unstaking_period(e: &Env, unstaking_period: i64) {
    e.storage().instance().set(&DataKey::UnstakingPeriod, &unstaking_period);
}

pub fn get_unstaking_period(e: &Env) -> i64 {
    e.storage().instance().get(&DataKey::UnstakingPeriod).unwrap()
}

// Paused operations

pub fn set_paused_operations(e: &Env, paused_operations: u32) {
    e.storage().instance().set(&DataKey::PausedOperations, &paused_operations);
}

pub fn get_paused_operations(e: &Env) -> u32 {
    e.storage().instance().get(&DataKey::PausedOperations).unwrap()
}
