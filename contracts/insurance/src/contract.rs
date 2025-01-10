use soroban_sdk::{assert_with_error, contract, contractimpl, Address, Env};

use crate::{
    errors,
    events::InsuranceEvents,
    insurance_fund::InsuranceFundTrait,
    storage::{get_config, set_config, DataKey},
};

use normal::utils::validate;

contractmeta!(
    key = "Description",
    val = "Staking vault used to cover protocol debt"
);

#[contract]
pub struct Insurance;

#[contractimpl]
/// Implementation of the Insurance Fund trait to allow for ...
impl InsuranceFundTrait for Insurance {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        share_token: Address,
        governor: Address,
        admin: Address,
        max_insurance: u64,
        unstaking_period: i64,
        paused_operations: u32,
        min_reward: i128,
    ) {
        if is_initialized(&env) {
            log!(
                &env,
                "Insurance Fund: Initialize: initializing contract twice is not allowed"
            );
            panic_with_error!(&env, ContractError::AlreadyInitialized);
        }

        set_initialized(&env);

        let config = Config {
            lp_token,
            min_bond,
            min_reward,
            manager,
            owner,
            max_complexity,
        };
        save_config(&env, config);

        InsuranceEvents::insurance_fund_initialization(&e, index_id, from, amount);
    }

    fn stake(env: Env, sender: Address, amount: i128) {
        sender.require_auth();

        if amount <= 0 {
            return Err(ErrorCode::InsufficientDeposit);
        }

        validate!(
            !insurance_fund.is_operation_paused(InsuranceFundOperation::Add),
            ErrorCode::InsuranceFundOperationPaused,
            "if staking add disabled"
        )?;
        // if is_operation_paused(&env, &Operation::Stake) {
        //     return Err(ErrorCode::OperationPaused);
        // }

        // TODO: Ensure amount will not put Insurance Fund over max_insurance
        // validate!(
        // 	insurance_fund.max_insurance >,
        // 	ErrorCode::InsuranceFundOperationPaused,
        // 	"if staking add disabled"
        // )?;

        validate!(
            insurance_fund_stake.last_withdraw_request_shares == 0
                && insurance_fund_stake.last_withdraw_request_value == 0,
            ErrorCode::IFWithdrawRequestInProgress,
            "withdraw request in progress"
        )?;

        utils::add_stake(
            amount,
            ctx.accounts.insurance_fund_vault.amount,
            insurance_fund_stake,
            insurance_fund,
            clock.unix_timestamp,
        )?;

        controller::token::receive(
            &ctx.accounts.token_program,
            &ctx.accounts.user_token_account,
            &ctx.accounts.insurance_fund_vault,
            &ctx.accounts.authority,
            amount,
            &mint,
        )?;
    }

    fn unstake(env: Env, sender: Address, amount: i128) -> i128 {
        sender.require_auth();

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

        // InsuranceEvents::unstake(
        //     &e,
        //     to,
        //     '',
        //     amount,
        // );

        out_a;
    }

    fn withdraw_rewards(env: Env, sender: Address) {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        env.events().publish(("withdraw_rewards", "user"), &sender);

        let mut stakes = get_stakes(&env, &sender);

        for asset in get_distributions(&env) {
            let pending_reward = calculate_pending_rewards(&env, &asset, &stakes);
            env.events()
                .publish(("withdraw_rewards", "reward_token"), &asset);

            token_contract::Client::new(&env, &asset).transfer(
                &env.current_contract_address(),
                &sender,
                &pending_reward,
            );
        }
        stakes.last_reward_time = env.ledger().timestamp();
        save_stakes(&env, &sender, &stakes);
    }
}

#[contractimpl]
/// Implementation of the Buffer trait to allow for ...
impl BufferTrait for Insurance {
    fn initialize(
        env: Env,
        norm_token_contract_address: Address,
        lp_contract_address: Address,
        max_balance: i128,
    ) {
    }

    fn deposit(env: Env, amount: i128) {
        if amount <= 0 {
            return Err();
        }

        let current_balance = 0;
        if current_balance + amount > max_balance {
            return Err();
        }
    }

    fn buy_back_and_burn(env: Env, sender: Address, amount: i128) {
        if amount <= 0 {
            return Err();
        }

        // Buy NORM
        let swap_response: SwapResponse = env.invoke_contract(
            &norm_lp_contract_address,
            &Symbol::new(&env, "swap"),
            vec![
                &env,
                sender.into_val(&env),
                0,                               // _amount0_out: i128,
                0,                               // _amount1_out: i128,
                &env.current_contract_address(), // _to: Address,
                [],                              // _data: Bytes
            ],
        );

        // Burn it
        env.invoke_contract(
            &norm_token_contract_address,
            &Symbol::new(&env, "burn"),
            (env.current_contract_address(), amount),
        );

        // Update things
    }

    fn mint_and_sell(env: Env, sender: Address, amount: i128, to: Address) {
        if amount <= 0 {
            return Err();
        }

        // Mint NORM tokens
        env.invoke_contract(
            &norm_token_contract_address,
            &Symbol::new(&env, "mint"),
            (env.current_contract_address(), amount),
        );

        // Sell them
        let swap_response: SwapResponse = env.invoke_contract(
            &norm_lp_contract_address,
            &Symbol::new(&env, "swap"),
            vec![
                &env,
                sender.into_val(&env),
                0,                               // _amount0_out: i128,
                0,                               // _amount1_out: i128,
                &env.current_contract_address(), // _to: Address,
                [],                              // _data: Bytes
            ],
        );

        // Transfer proceeds to recipient
    }
}
