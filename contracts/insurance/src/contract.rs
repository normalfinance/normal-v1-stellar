use soroban_sdk::{
    contract,
    contractimpl,
    contractmeta,
    log,
    panic_with_error,
    Address,
    BytesN,
    Env,
    String,
    Vec,
};

use crate::{
    controller,
    events::{ InsuranceEvents, InsuranceFundEvents },
    insurance_fund::{ self, InsuranceFundTrait },
    math,
    storage::{ get_config, get_insurance_fund, get_stake, utils, Config, Operation },
    token_contract,
};

use normal::{
    constants::{
        INSTANCE_BUMP_AMOUNT,
        INSTANCE_LIFETIME_THRESHOLD,
        THIRTEEN_DAY,
        ONE_MILLION_QUOTE,
    },
    error::{ ErrorCode, NormalResult },
    utils,
    validate,
};

fn check_nonnegative_amount(amount: i128) {
    if amount < 0 {
        panic!("negative amount is not allowed: {}", amount)
    }
}

contractmeta!(key = "Description", val = "Staking vault used to cover protocol debt");

#[contract]
pub struct Insurance;

#[contractimpl]
/// Implementation of the Insurance Fund trait to allow for ...
impl InsuranceFundTrait for Insurance {
    // ################################################################
    //                             ADMIN
    // ################################################################

    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        governor: Address,
        stake_asset: Address,
        token_wasm_hash: BytesN<32>,
        share_token_decimals: u32,
        share_token_name: String,
        share_token_symbol: String
    ) {
        if utils::is_initialized(&env) {
            log!(&env, "Insurance Fund: Initialize: initializing contract twice is not allowed");
            panic_with_error!(&env, ErrorCode::AlreadyInitialized);
        }

        utils::set_initialized(&env);

        // deploy and initialize token contract
        let share_token_address = utils::deploy_token_contract(
            &env,
            token_wasm_hash.clone(),
            &governor,
            env.current_contract_address(),
            share_token_decimals,
            share_token_name,
            share_token_symbol
        );

        let config = Config {
            admin,
            governor,
            stake_asset,
            share_token: share_token_address,
            unstaking_period: THIRTEEN_DAY,
            revenue_settle_period: THIRTEEN_DAY,
            max_insurance: ONE_MILLION_QUOTE,
            paused_operations: Vec::new(&env),
        };
        save_config(&env, config);

        InsuranceFundEvents::initialization(
            &env,
            env.ledger().timestamp(),
            admin,
            governor,
            share_token_address
        );
    }

    // ################################################################
    //                             USER
    // ################################################################

    fn add_stake(env: Env, sender: Address, amount: u64) {
        sender.require_auth();
        check_nonnegative_amount(amount);

        let now = env.ledger().timestamp();
        let config = get_config(&env);
        let insurance_fund = get_insurance_fund(&env);

        validate!(
            !insurance_fund.is_operation_paused(&Operation::Add),
            ErrorCode::InsuranceFundOperationPaused,
            "if staking add disabled"
        )?;

        // TODO: Ensure amount will not put Insurance Fund over max_insurance
        // validate!(
        // 	insurance_fund.max_insurance >,
        // 	ErrorCode::InsuranceFundOperationPaused,
        // 	"if staking add disabled"
        // )?;

        let stake = get_stake(&env, &sender);

        validate!(
            stake.last_withdraw_request_shares == 0 && stake.last_withdraw_request_value == 0,
            ErrorCode::IFWithdrawRequestInProgress,
            "withdraw request in progress"
        )?;

        controller::stake::add_stake(
            &env,
            &mut insurance_fund,
            amount,
            insurance_balance,
            &mut stake,
            now
        );

        token_contract::Client
            ::new(&env, &config.stake_asset)
            .transfer(&sender, &env.current_contract_address(), &amount);
    }

    fn request_remove_stake(env: Env, sender: Address, amount: u64) {
        sender.require_auth();

        let now = env.ledger().timestamp();
        let insurance_fund = get_insurance_fund(&env);

        validate!(
            !insurance_fund.is_operation_paused(&Operation::RequestRemove),
            ErrorCode::InsuranceFundOperationPaused,
            "if staking request remove disabled"
        )?;

        let stake = get_stake(&env, &sender);

        validate!(
            stake.last_withdraw_request_shares == 0,
            ErrorCode::IFWithdrawRequestInProgress,
            "Withdraw request is already in progress"
        )?;

        let n_shares = math::insurance::vault_amount_to_if_shares(
            &env,
            amount,
            insurance_fund.total_shares,
            ctx.accounts.insurance_fund_vault.amount
        )?;

        validate!(n_shares > 0, ErrorCode::IFWithdrawRequestTooSmall, "Requested lp_shares = 0")?;

        let user_if_shares = stake.checked_if_shares(insurance_fund_stake)?;
        validate!(user_if_shares >= n_shares, ErrorCode::InsufficientIFShares, "")?;

        controller::stake::request_remove_stake(
            &env,
            n_shares,
            insurance_vault_amount,
            &mut stake,
            &mut insurance_fund,
            now
        )
    }

    fn cancel_request_remove_stake(env: Env, sender: Address) {
        sender.require_auth();

        let now = env.ledger().timestamp();
        let insurance_fund = get_insurance_fund(&env);

        let stake = get_stake(&env, &sender);
        validate!(
            stake.last_withdraw_request_shares != 0,
            ErrorCode::NoIFWithdrawRequestInProgress,
            "No withdraw request in progress"
        )?;

        controller::stake::cancel_request_remove_stake(
            &env,
            insurance_vault_amount,
            &mut stake,
            &mut insurance_fund,
            now
        )
    }

    fn remove_stake(env: Env, sender: Address) {
        sender.require_auth();

        let now = env.ledger().timestamp();
        let config = get_config(&env);
        let insurance_fund = get_insurance_fund(&env);

        validate!(
            !insurance_fund.is_operation_paused(&Operation::Remove),
            ErrorCode::InsuranceFundOperationPaused,
            "if staking remove disabled"
        )?;

        let stake = get_stake(&env, &sender);

        let amount = controller::stake::remove_stake(
            &env,
            insurance_vault_amount,
            &mut stake,
            &mut insurance_fund,
            now
        );

        token_contract::Client
            ::new(&env, &config.stake_asset)
            .transfer(&env.current_contract_address(), &sender, &amount);
    }

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_config(env: Env) -> Config {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        get_config(&env)
    }

    fn query_insurance_fund(env: Env) -> InsuranceFund {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        get_insurance_fund(&env)
    }

    fn query_stake(env: Env, address: Address) -> Stake {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_stake(&env, &address);
    }
}

#[contractimpl]
/// Implementation of the Buffer trait to allow for ...
impl BufferTrait for Insurance {
    fn initialize(
        env: Env,
        norm_token_contract_address: Address,
        lp_contract_address: Address,
        max_balance: i128
    ) {}

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
                0, // _amount0_out: i128,
                0, // _amount1_out: i128,
                &env.current_contract_address(), // _to: Address,
                [] // _data: Bytes
            ]
        );

        // Burn it
        env.invoke_contract(&norm_token_contract_address, &Symbol::new(&env, "burn"), (
            env.current_contract_address(),
            amount,
        ));

        // Update things
    }

    fn mint_and_sell(env: Env, sender: Address, amount: i128, to: Address) {
        if amount <= 0 {
            return Err();
        }

        // Mint NORM tokens
        env.invoke_contract(&norm_token_contract_address, &Symbol::new(&env, "mint"), (
            env.current_contract_address(),
            amount,
        ));

        // Sell them
        let swap_response: SwapResponse = env.invoke_contract(
            &norm_lp_contract_address,
            &Symbol::new(&env, "swap"),
            vec![
                &env,
                sender.into_val(&env),
                0, // _amount0_out: i128,
                0, // _amount1_out: i128,
                &env.current_contract_address(), // _to: Address,
                [] // _data: Bytes
            ]
        );

        // Transfer proceeds to recipient
    }
}
