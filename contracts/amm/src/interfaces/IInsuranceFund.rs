use soroban_sdk::{ Address, Env };

// use crate::storage_types::Pair;

pub trait IInsuranceFund {
    fn init(
        e: Env,
        admin: Address,
        max_insurance: u64,
        unstaking_period: i64,
        paused_operations: u32
    );

    fn get_admin(e: Env) -> Address;

    // IF
    fn get_max_insurance(e: Env) -> u64;
    fn set_max_insurance(e: Env, max_insurance: u64);

    fn get_paused_operations(e: Env) -> u32;
    fn set_paused_operations(e: Env, paused_operations: u32);

    fn get_unstaking_period(e: Env) -> i64;
    fn set_unstaking_period(e: Env, if_unstaking_period: i64);

    // Staking
    // fn init_stake(e: Env);
    // fn add_stake(e: Env);
    // fn request_remove_stake(e: Env);
    // fn cancel_request_remove_stake(e: Env);
    // fn remove_stake(e: Env);
}
