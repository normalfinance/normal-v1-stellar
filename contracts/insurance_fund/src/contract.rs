use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

use crate::{
    errors,
    interfaces::{ IInsuranceFund::IInsuranceFund },
    storage::{
        get_admin,
        get_max_insurance,
        get_paused_operations,
        get_unstaking_period,
        set_admin,
        set_max_insurance,
        set_unstaking_period,
        set_paused_operations,
    },
    storage_types::{ DataKey },
};

#[contract]
pub struct InsuranceFund;

#[contractimpl]
impl IInsuranceFund for InsuranceFund {
    fn init(
        e: Env,
        admin: Address,
        max_insurance: u64,
        unstaking_period: i64,
        paused_operations: u32
    ) {
        // todo: already initiazed check
        //
        set_admin(&e, admin);
        set_max_insurance(&e, max_insurance);
        set_unstaking_period(&e, unstaking_period);
        set_paused_operations(&e, paused_operations);
    }

    fn get_admin(e: Env) -> Address {
        get_admin(&e)
    }

    fn get_max_insurance(e: Env) -> u64 {
        get_max_insurance(&e)
    }

    fn set_max_insurance(e: Env, max_insurance: u64) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_max_insurance(&e, max_insurance);
    }

    fn get_unstaking_period(e: Env) -> i64 {
        get_unstaking_period(&e)
    }

    fn set_unstaking_period(e: Env, unstaking_period: i64) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_unstaking_period(&e, unstaking_period);
    }

    fn get_paused_operations(e: Env) -> u32 {
        get_paused_operations(&e)
    }

    fn set_paused_operations(e: Env, paused_operations: u32) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_paused_operations(&e, paused_operations);
    }

    // Staking

    // fn add_stake(e: Env, new_paused_operations: u32) {}
}
