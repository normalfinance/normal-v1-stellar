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
    events:InsuranceEvents
};

contractmeta!(key = "Description", val = "Staking vault used to cover protocol debt");

#[contract]
pub struct InsuranceFund;

#[contractimpl]
impl IInsuranceFund for InsuranceFund {
    fn init(
        e: Env,
        governor: Address,
        admin: Address,
        max_insurance: u64,
        unstaking_period: i64,
        paused_operations: u32
    ) {
        // todo: already initiazed check
        //
        set_governor(&e, governor);
        set_admin(&e, admin);
        set_max_insurance(&e, max_insurance);
        set_unstaking_period(&e, unstaking_period);
        set_paused_operations(&e, paused_operations);

        InsuranceEvents::insurance_fund_initialization(
            &e,
            index_id,
            from,
            amount,
        );
    }

    fn get_admin(e: Env) -> Address {
        get_admin(&e)
    }

    fn get_max_insurance(e: Env) -> u64 {
        get_max_insurance(&e)
    }

    fn set_max_insurance(e: Env, max_insurance: u64) {
        is_governor(&e);
        is_admin(&e);
        set_max_insurance(&e, max_insurance);
    }

    fn get_unstaking_period(e: Env) -> i64 {
        get_unstaking_period(&e);
    }

    fn set_unstaking_period(e: Env, unstaking_period: i64) {
        is_admin(&e);
        set_unstaking_period(&e, unstaking_period);
    }

    fn get_paused_operations(e: Env) -> Vec<Operation> {
        get_paused_operations(&e);
    }

    fn set_paused_operations(e: Env, paused_operations: Vec<Operation>) {
        is_admin(&e);
        set_paused_operations(&e, paused_operations);
    }

    fn stake(e: Env, to: Address, amount: i128) {
        // Depositor needs to authorize the deposit
        to.require_auth();

        if is_operation_paused(&env, &Operation::Stake) {
            return Err(ErrorCode::OperationPaused);
        }

        let token_client = token::Client::new(&e, &get_token_a(&e));

        oken_a_client.transfer(&to, &e.current_contract_address(), &amount_a);

        // Now calculate how many new pool shares to mint
        //  ...

        mint_shares(&e, to, new_total_shares - total_shares);

        InsuranceEvents::stake(
            &e,
            to,
            '',
            amount,
        );
    }

    fn unstake(e: Env, to: Address, share_amount: i128) -> i128 {
        to.require_auth();

        if is_operation_paused(&env, &Operation::Unstake) {
            return Err(ErrorCode::OperationPaused);
        }

        // First transfer the pool shares that need to be redeemed
        let share_token_client = token::Client::new(&e, &get_token_share(&e));
        share_token_client.transfer(&to, &e.current_contract_address(), &share_amount);

        let balance_a = get_balance_a(&e);
        let balance_shares = get_balance_shares(&e);

        let total_shares = get_total_shares(&e);

        // Now calculate the withdraw amounts
        let out_a = (balance_a * balance_shares) / total_shares;

        burn_shares(&e, balance_shares);
        transfer_a(&e, to.clone(), out_a);
        put_reserve_a(&e, balance_a - out_a);

        InsuranceEvents::unstake(
            &e,
            to,
            '',
            amount,
        );

        out_a;
    }
}
