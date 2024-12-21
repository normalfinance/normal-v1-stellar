use soroban_sdk::{ Address, Env };

use crate::storage_types::{ DataKey, Stake };

// Governor

pub fn is_governor(e: &Env) {
    if e.invoker() != get_governor(e) {
        return Err(ErrorCode:OnlyGovernor)
    }
    // TODO: do we need to auth the governor?
    // governor.require_auth();
}

pub fn set_governor(e: &Env, governor: Address) {
    e.storage().instance().set(&DataKey::Governor, &governor);
}

pub fn get_governor(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Governor).unwrap()
}

// Admin

pub fn set_admin(e: &Env, admin: Address) {
    e.storage().instance().set(&DataKey::Admin, &admin);
}

pub fn get_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).unwrap()
}

pub fn is_admin(e: &Env) {
    let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
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

pub fn set_paused_operations(e: &Env, paused_operations: Vec<Operation>) {
    e.storage().instance().set(&DataKey::PausedOperations, &paused_operations);
}

pub fn get_paused_operations(e: &Env) -> Vec<Operation> {
    e.storage()
        .get::<Vec<PausedOperation>>(&DataKey::PausedOperations)
        .unwrap_or_else(|| Vec::new(env));
}

pub fn is_operation_paused(e: &Env, operation: &Operation) -> bool {
    let paused_operations = get_paused_operations(e);
    paused_operations.contains(operation)
}
