use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

use crate::{
    errors,
    interfaces::{ IInsuranceFund::IInsuranceFund },
    storage::{ get_admin },
    storage_types::{ DataKey },
};

#[contract]
pub struct Index;

#[contractimpl]
impl Index {
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

    // fn get_admin(e: Env) -> Address {
    //     get_admin(&e)
    // }

    // Getters

    // fn get_max_insurance(e: Env) -> u64 {
    //     get_max_insurance(&e)
    // }

    // fn get_unstaking_period(e: Env) -> i64 {
    //     get_unstaking_period(&e)
    // }

    // Setters

    fn update_index_expense_ratio(e: Env, max_insurance: u64) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_max_insurance(&e, max_insurance);
    }

    fn update_index_expiry(e: Env, max_insurance: u64) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_max_insurance(&e, max_insurance);
    }

    fn update_index_paused_operations(e: Env, max_insurance: u64) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_max_insurance(&e, max_insurance);
    }

    fn update_index_visibility(e: Env, max_insurance: u64) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_max_insurance(&e, max_insurance);
    }

    fn update_index_whitelist(e: Env, whitelist: Vec<Address>) {
        // let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        whitelist_authority.require_auth();
        set_max_insurance(&e, max_insurance);
    }

    fn update_index_blacklist(e: Env, max_insurance: u64) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_max_insurance(&e, max_insurance);
    }

    // User

    fn mint_index_tokens(e: Env, amount: u64) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_max_insurance(&e, max_insurance);
    }

    fn redeem_index_tokens(e: Env, amount: u64) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_max_insurance(&e, max_insurance);
    }

    fn rebalance_index(e: Env) {
        // let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        index.rebalance_authority.require_auth();
        // set_max_insurance(&e, max_insurance);
    }
}
