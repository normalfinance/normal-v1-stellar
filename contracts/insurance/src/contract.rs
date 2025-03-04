use soroban_sdk::{
    contract, contractimpl, contractmeta, log, panic_with_error, Address, BytesN, Env, String,
    Symbol, Vec,
};

use crate::{
    buffer::BufferTrait,
    controller,
    errors::Errors,
    events::InsuranceFundEvents,
    insurance_fund::InsuranceFundTrait,
    math,
    storage::{
        get_buffer, get_insurance_fund, get_stake, save_buffer, save_insurance_fund,
        utils::{self, get_admin},
        Buffer, InsuranceFund, InsuranceFundOperation, Stake,
    },
    token_contract,
};

use normal::{
    constants::{
        INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD, ONE_MILLION_QUOTE, THIRTEEN_DAY,
    },
    validate,
};

contractmeta!(
    key = "Description",
    val = "Staking vault used to cover protocol debt"
);

#[contract]
pub struct Insurance;

#[contractimpl]
/// Implementation of the Insurance Fund trait to allow for ...
impl InsuranceFundTrait for Insurance {
    // ################################################################
    //                             Admin
    // ################################################################

    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        governor_contract: Address,
        deposit_token: Address,
        token_wasm_hash: BytesN<32>,
        stake_token_decimals: u32,
        stake_token_name: String,
        stake_token_symbol: String,
        max_buffer_balance: i128,
    ) {
        if utils::is_initialized(&env) {
            log!(
                &env,
                "Insurance: Initialize: initializing contract twice is not allowed"
            );
            panic_with_error!(&env, Errors::AlreadyInitialized);
        }

        utils::set_initialized(&env);

        // deploy and initialize token contract
        let stake_token_address = utils::deploy_token_contract(
            &env,
            token_wasm_hash.clone(),
            &governor_contract,
            env.current_contract_address(),
            stake_token_decimals,
            stake_token_name,
            stake_token_symbol,
        );

        utils::save_admin(&env, &admin);
        utils::save_governor(&env, &governor_contract);

        let insurance_fund = InsuranceFund {
            deposit_token,
            stake_token: stake_token_address.clone(),
            unstaking_period: THIRTEEN_DAY,
            revenue_settle_period: THIRTEEN_DAY,
            max_insurance: ONE_MILLION_QUOTE,
            paused_operations: Vec::new(&env),
            total_shares: 0,
            user_shares: 0,
            shares_base: 0,
            last_revenue_settle_ts: 0,
            total_factor: 0,
            user_factor: 0,
        };
        save_insurance_fund(&env, insurance_fund);

        let governance_token: Address = env.invoke_contract(
            &governor_contract,
            &Symbol::new(&env, "query_governance_token_address"),
            Vec::new(&env),
        );

        let buffer = Buffer {
            governance_token: governance_token.clone(),
            governance_token_pool: governance_token.clone(), // TODO:
            quote_token: governance_token.clone(),           // TODO:
            auctions: Vec::new(&env),
            min_auction_duration: 3600,
            max_balance: max_buffer_balance,
            total_burns: 0,
            total_mints: 0,
        };
        save_buffer(&env, buffer);

        InsuranceFundEvents::initialize_if(
            &env,
            env.ledger().timestamp(),
            admin,
            governor_contract,
            stake_token_address,
        );
    }

    // ################################################################
    //                             USER
    // ################################################################

    fn add_if_stake(env: Env, sender: Address, amount: i128) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        let now = env.ledger().timestamp();
        let mut insurance_fund = get_insurance_fund(&env);

        validate!(
            &env,
            !insurance_fund.is_operation_paused(&InsuranceFundOperation::Add),
            Errors::InsuranceFundOperationPaused,
            "if staking add disabled"
        );

        // TODO: Ensure amount will not put Insurance Fund over max_insurance
        // validate!(
        // 	insurance_fund.max_insurance >,
        // 	Errors::InsuranceFundOperationPaused,
        // 	"if staking add disabled"
        // );

        let mut stake = get_stake(&env, &sender);

        validate!(
            &env,
            stake.last_withdraw_request_shares == 0 && stake.last_withdraw_request_value == 0,
            Errors::IFWithdrawRequestInProgress,
            "withdraw request in progress"
        );

        let insurance_vault_amount =
            token_contract::Client::new(&env, &insurance_fund.deposit_token)
                .balance(&env.current_contract_address());

        controller::stake::add_stake(
            &env,
            &sender,
            amount,
            insurance_vault_amount,
            &mut stake,
            &mut insurance_fund,
            now,
        );

        token_contract::Client::new(&env, &insurance_fund.deposit_token).transfer(
            &sender,
            &env.current_contract_address(),
            &amount,
        );
    }

    fn request_remove_if_stake(env: Env, sender: Address, amount: i128) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        let now = env.ledger().timestamp();
        let mut insurance_fund = get_insurance_fund(&env);

        validate!(
            &env,
            !insurance_fund.is_operation_paused(&InsuranceFundOperation::RequestRemove),
            Errors::InsuranceFundOperationPaused,
            "if staking request remove disabled"
        );

        let mut stake = get_stake(&env, &sender);

        validate!(
            &env,
            stake.last_withdraw_request_shares == 0,
            Errors::IFWithdrawRequestInProgress,
            "Withdraw request is already in progress"
        );

        let insurance_vault_amount =
            token_contract::Client::new(&env, &insurance_fund.deposit_token)
                .balance(&env.current_contract_address());

        let n_shares = math::insurance::vault_amount_to_if_shares(
            &env,
            amount,
            insurance_fund.total_shares,
            insurance_vault_amount,
        );

        validate!(
            &env,
            n_shares > 0,
            Errors::IFWithdrawRequestTooSmall,
            "Requested lp_shares = 0"
        );

        let user_if_shares = stake.checked_if_shares(&env, &insurance_fund);
        validate!(
            &env,
            user_if_shares >= n_shares,
            Errors::InsufficientIFShares,
            ""
        );

        controller::stake::request_remove_stake(
            &env,
            &sender,
            n_shares,
            insurance_vault_amount,
            &mut stake,
            &mut insurance_fund,
            now,
        );
    }

    fn cancel_request_remove_if_stake(env: Env, sender: Address) {
        sender.require_auth();

        let now = env.ledger().timestamp();
        let mut insurance_fund = get_insurance_fund(&env);

        let mut stake = get_stake(&env, &sender);
        validate!(
            &env,
            stake.last_withdraw_request_shares != 0,
            Errors::NoIFWithdrawRequestInProgress,
            "No withdraw request in progress"
        );

        let insurance_vault_amount =
            token_contract::Client::new(&env, &insurance_fund.deposit_token)
                .balance(&env.current_contract_address());

        controller::stake::cancel_request_remove_stake(
            &env,
            &sender,
            insurance_vault_amount,
            &mut insurance_fund,
            &mut stake,
            now,
        );
    }

    fn remove_if_stake(env: Env, sender: Address) {
        sender.require_auth();

        let now = env.ledger().timestamp();
        let mut insurance_fund = get_insurance_fund(&env);

        validate!(
            &env,
            !insurance_fund.is_operation_paused(&InsuranceFundOperation::Remove),
            Errors::InsuranceFundOperationPaused,
            "if staking remove disabled"
        );

        let mut stake = get_stake(&env, &sender);

        let insurance_vault_amount =
            token_contract::Client::new(&env, &insurance_fund.deposit_token)
                .balance(&env.current_contract_address());

        let amount = controller::stake::remove_stake(
            &env,
            &sender,
            insurance_vault_amount,
            &mut stake,
            &mut insurance_fund,
            now,
        );

        token_contract::Client::new(&env, &insurance_fund.deposit_token).transfer(
            &env.current_contract_address(),
            &sender,
            &amount,
        );
    }

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_insurance_fund(env: Env) -> InsuranceFund {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        get_insurance_fund(&env)
    }

    fn query_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_admin(&env)
    }

    fn query_if_stake(env: Env, address: Address) -> Stake {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_stake(&env, &address)
    }
}

#[contractimpl]
/// Implementation of the Buffer trait to allow for ...
impl BufferTrait for Insurance {
    fn update_buffer_max_balance(env: Env, sender: Address, max_balance: i128) {
        sender.require_auth();
        utils::check_nonnegative_amount(max_balance);

        let mut buffer = get_buffer(&env);
        buffer.max_balance = max_balance;
    }

    fn deposit_into_buffer(env: Env, sender: Address, amount: i128) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        let _current_balance = 0;

        // if current_balance + amount > max_balance {
        //     return Err();
        // }

        // ...

        let buffer = get_buffer(&env);

        token_contract::Client::new(&env, &buffer.governance_token).transfer(
            &sender,
            &env.current_contract_address(),
            &amount,
        );
    }

    fn execute_buffer_buyback(env: Env, sender: Address, amount: i128) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        let buffer = get_buffer(&env);

        // Buy <amount> of the gov token from the secondary market
        // let swap_response: SwapResponse = env.invoke_contract(
        //     &norm_lp_contract_address,
        //     &Symbol::new(&env, "swap"),
        //     vec![
        //         &env,
        //         sender.into_val(&env),
        //         0, // _amount0_out: i128,
        //         0, // _amount1_out: i128,
        //         &env.current_contract_address(), // _to: Address,
        //         [] // _data: Bytes
        //     ]
        // );

        // Burn the tokens (to reduce price)
        token_contract::Client::new(&env, &buffer.governance_token)
            .burn(&env.current_contract_address(), &amount);
    }

    fn execute_buffer_auction(env: Env, sender: Address, amount: i128) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        let buffer = get_buffer(&env);

        // Mint gov tokens
        token_contract::Client::new(&env, &buffer.governance_token)
            .mint(&env.current_contract_address(), &amount);

        // let out_amount = match buffer.auction_location {
        //     AuctionLocation::Native => {
        //         // TODO: run gov token auction
        //     }
        //     AuctionLocation::External => {
        //         // Sell them (https://github.com/AquaToken/soroban-amm/blob/master/liquidity_pool_router/src/contract.rs#L222)
        //         let x = LiquidityPoolInterfaceTrait::swap(
        //             e,
        //             user,
        //             tokens,
        //             token_in,
        //             token_out,
        //             pool_index,
        //             in_amount,
        //             out_min
        //         );

        //         let out_amt = pool_contract::Client
        //             ::new(&env, &config.buffer.gov_token_pool)
        //             .swap(
        //                 &env,
        //                 &env.current_contract_address(),
        //                 vec![&config.buffer.gov_token, &config.buffer.quote_token],
        //                 &config.buffer.gov_token,
        //                 &config.buffer.quote_token,
        //                 &config.buffer.pool_index,
        //                 &amount,
        //                 0
        //             );
        //         out_amt;
        //     }
        // };
    }

    // ################################################################
    //                             USER
    // ################################################################

    // fn bid_buffer_auction(env: Env, user: Address, auction_ts: u64, bid_amount: i128) {
    //     user.require_auth();
    //     check_nonnegative_amount(bid_amount);

    //     let auction = get_auction(&env, auction_ts);

    //     validate_auction(&env, auction);

    //     let config = get_config(&env);

    //     // user sends quote token
    //     token_contract::Client::new(&env, &config.buffer.quote_token).transfer(
    //         &user,
    //         &env.current_contract_address(),
    //         &bid_amount,
    //     );

    //     // buffer sends gov token
    //     let gov_token_amount = 0; // TODO:
    //     token_contract::Client::new(&env, &config.buffer.gov_token).transfer(
    //         &env.current_contract_address(),
    //         &user,
    //         &gov_token_amount,
    //     );

    //     // update buffer and auction
    //     auction.available_balance -= gov_token_amount;
    // }

    // ################################################################
    //                             QUERIES
    // ################################################################

    // fn query_buffer(env: Env) {}

    // fn query_buffer_auctions(env: Env) {}

    // fn query_buffe_balance(env: Env) {}
}

// fn validate_auction(env: &Env, auction: Auction) {
//     // check balance

//     // check ts within duration
// }
