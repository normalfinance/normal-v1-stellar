use soroban_sdk::{
    contract,
    contractimpl,
    contractmeta,
    log,
    panic_with_error,
    vec,
    Address,
    BytesN,
    Env,
    String,
    Vec,
};

use crate::{
    controller,
    events::{ BufferEvents, InsuranceFundEvents },
    insurance_fund::{ self, InsuranceFundTrait },
    interfaces::aqua::LiquidityPoolInterfaceTrait,
    math,
    pool_contract,
    storage::{
        get_config,
        get_insurance_fund,
        get_stake,
        utils,
        Auction,
        AuctionLocation,
        Config,
        Operation,
        Stake,
    },
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
        gov_token: Address,
        stake_asset: Address,
        token_wasm_hash: BytesN<32>,
        share_token_decimals: u32,
        share_token_name: String,
        share_token_symbol: String,
        max_buffer_balance: i128
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

    fn add_if_stake(env: Env, sender: Address, amount: u64) {
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

    fn request_remove_if_stake(env: Env, sender: Address, amount: u64) {
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

    fn cancel_request_remove_if_stake(env: Env, sender: Address) {
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

    fn remove_if_stake(env: Env, sender: Address) {
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

    fn query_if(env: Env) -> InsuranceFund {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        get_insurance_fund(&env)
    }

    fn query_if_stake(env: Env, address: Address) -> Stake {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_stake(&env, &address);
    }
}

#[contractimpl]
/// Implementation of the Buffer trait to allow for ...
impl BufferTrait for Insurance {
    fn update_buffer_max_balance(env: Env, sender: Address, max_balance: i128) {
        sender.require_auth();
        check_nonnegative_amount(max_balance);

        let mut config = get_config(&env);
        config.buffer.max_balance = max_balance;
    }

    fn deposit_into_buffer(env: Env, sender: Address, amount: i128) {
        sender.require_auth();
        check_nonnegative_amount(amount);

        let current_balance = 0;
        // if current_balance + amount > max_balance {
        //     return Err();
        // }

        // ...

        let config = get_config(&env);

        token_contract::Client
            ::new(&env, &config.buffer.gov_token)
            .transfer(&sender, &env.current_contract_address(), &amount);
    }

    fn execute_buffer_buyback(env: Env, sender: Address, amount: i128) {
        sender.require_auth();
        check_nonnegative_amount(amount);

        let config = get_config(&env);

        // Buy <amount> of the gov token from the secondary market
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

        // Burn the tokens (to reduce price)
        token_contract::Client
            ::new(&env, &config.buffer.gov_token)
            .burn(&env.current_contract_address(), &amount);
    }

    fn execute_buffer_auction(env: Env, sender: Address, amount: i128) {
        sender.require_auth();
        check_nonnegative_amount(amount);

        let config = get_config(&env);

        // Mint gov tokens
        token_contract::Client
            ::new(&env, &config.buffer.gov_token)
            .mint(&env.current_contract_address(), &amount);

        let out_amount = match config.buffer.auction_location {
            AuctionLocation::Native => {
                // TODO: run gov token auction
            }
            AuctionLocation::External => {
                // Sell them (https://github.com/AquaToken/soroban-amm/blob/master/liquidity_pool_router/src/contract.rs#L222)
                let x = LiquidityPoolInterfaceTrait::swap(
                    e,
                    user,
                    tokens,
                    token_in,
                    token_out,
                    pool_index,
                    in_amount,
                    out_min
                );

                let out_amt = pool_contract::Client
                    ::new(&env, &config.buffer.gov_token_pool)
                    .swap(
                        &env,
                        &env.current_contract_address(),
                        vec![&config.buffer.gov_token, &config.buffer.quote_token],
                        &config.buffer.gov_token,
                        &config.buffer.quote_token,
                        &config.buffer.pool_index,
                        &amount,
                        0
                    );
                out_amt;
            }
        };
    }

    // ################################################################
    //                             USER
    // ################################################################

    fn bid_buffer_auction(env: Env, user: Address, auction_ts: u64, bid_amount: i128) {
        user.require_auth();
        check_nonnegative_amount(bid_amount);

        let auction = get_auction(&env, auction_ts);

        validate_auction(&env, auction);

        let config = get_config(&env);

        // user sends quote token
        token_contract::Client
            ::new(&env, &config.buffer.quote_token)
            .transfer(&user, &env.current_contract_address(), &bid_amount);

        // buffer sends gov token
        let gov_token_amount = 0; // TODO:
        token_contract::Client
            ::new(&env, &config.buffer.gov_token)
            .transfer(&env.current_contract_address(), &user, &gov_token_amount);

        // update buffer and auction
        auction.available_balance -= gov_token_amount;
    }

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_buffer(env: Env) {}

    fn query_buffer_auctions(env: Env) {}

    fn query_buffe_balance(env: Env) {}
}

fn validate_auction(env: &Env, auction: Auction) {
    // check balance

    // check ts within duration
}
