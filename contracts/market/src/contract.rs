use normal::{
    constants::{
        DAY_IN_SECONDS,
        INSTANCE_BUMP_AMOUNT,
        INSTANCE_LIFETIME_THRESHOLD,
        LIQUIDATION_FEE_PRECISION,
        SPOT_IMF_PRECISION,
    },
    error::ErrorCode,
    oracle::{ get_oracle_price, OraclePriceData, OracleSource },
    types::SynthTier,
    validate,
    validate_bps,
};
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
    Map,
    String,
    Vec,
};

use crate::{
    controller,
    events::{ MarketEvents, PoolEvents },
    interface::{ market::MarketTrait, pool::PoolTrait },
    math,
    state::{
        liquidity_position::{
            get_liquidity_position_by_ts,
            get_liquidity_position_info,
            LiquidityPositionUpdate,
        },
        market::{ get_market, save_market, Market, MarketOperation, MarketParams, MarketStatus },
        market_position::{ get_market_position, save_market_position },
        pool::Pool,
        reward::{ calculate_collect_reward, RewardInfo },
        tick_array::TickArray,
    },
    storage::utils,
    token_contract,
    utils::{ sparse_swap::SparseSwapTickSequenceBuilder, swap_utils::update_and_swap_amm },
    validation::margin::validate_margin,
};

contractmeta!(
    key = "Description",
    val = "Constant product AMM that maintains a synthetic asset peg"
);

// ################################################################
//                             Market
// ################################################################

#[contract]
pub struct SynthMarket;

#[contractimpl]
impl MarketTrait for SynthMarket {
    fn initialize(
        env: Env,
        sender: Address,
        params: MarketParams,
        token_wasm_hash: BytesN<32>,
        synth_token_name: String,
        synth_token_symbol: String
    ) {
        if utils::is_initialized(&env) {
            log!(&env, "Market: Initialize: initializing contract twice is not allowed");
            panic_with_error!(&env, ErrorCode::AlreadyInitialized);
        }

        // Verify oracle is readable
        let (oracle_price, oracle_delay, last_oracle_price_twap) = match params.oracle_source {
            OracleSource::Band => {
                let OraclePriceData {
                    price: oracle_price,
                    delay: oracle_delay,
                    ..
                } = get_band_price(&env, params.oracle)?;
                let last_oracle_price_twap = get_band_twap(&env, params.oracle)?;
                (oracle_price, oracle_delay, last_oracle_price_twap)
            }
            OracleSource::Reflector => {
                let OraclePriceData {
                    price: oracle_price,
                    delay: oracle_delay,
                    ..
                } = get_reflector_price(&env, params.oracle)?;

                (oracle_price, oracle_delay, oracle_price)
            }
            OracleSource::QuoteAsset => {
                log!(env, "Quote asset oracle cant be used for market");
                return Err(ErrorCode::InvalidOracle);
            }
        };

        validate_bps!(
            params.fee_rate,
            params.protocol_fee_rate,
            params.max_allowed_slippage_bps,
            params.max_allowed_variance_bps
        );

        validate_margin(
            &env,
            params.margin_ratio_initial,
            params.margin_ratio_maintenance,
            params.liquidator_fee
        )?;

        // if params.token_a >= params.token_b {
        //     panic!("token_a must be less than token_b");
        // }

        // if !(MIN_SQRT_PRICE_X64..=MAX_SQRT_PRICE_X64).contains(&params.sqrt_price) {
        //     return Err(ErrorCode::SqrtPriceOutOfBounds);
        // }

        // validations...

        utils::set_initialized(&env);

        // deploy and initialize the synth token contract
        let synth_token_address = utils::deploy_synth_token_contract(
            &env,
            token_wasm_hash.clone(),
            &params.token_a,
            env.current_contract_address(),
            params.token_decimals,
            params.token_name,
            params.token_symbol
        );

        // deploy and initialize the liquidity pool token contract
        let lp_token_address = utils::deploy_lp_token_contract(
            &env,
            token_wasm_hash.clone(),
            &params.token_a,
            &params.token_b,
            env.current_contract_address(),
            params.token_decimals,
            "NL",
            "NLP"
        );

        // Initialize core group of TickArrays
        let tick_current_index = math::tick_math::tick_index_from_sqrt_price(
            &params.pool.initial_sqrt_price
        );

        let tick_arrays = Map::new(&env).set(
            tick_current_index,
            TickArray::new(
                &env,
                env.current_contract_address(),
                tick_current_index,
                params.pool.tick_spacing
            )
        );
        let tick_steps: i32 = 10;

        for i in 0..tick_steps {
            let upper_tick_index = params.pool.tick_spacing.safe_mul(i, &env)?;
            let lower_tick_index = params.pool.tick_spacing.safe_mul(-i, &env)?;

            tick_arrays.set(
                upper_tick_index,
                TickArray::new(
                    &env,
                    env.current_contract_address(),
                    upper_tick_index,
                    params.pool.tick_spacing
                )
            );
            tick_arrays.set(
                lower_tick_index,
                TickArray::new(
                    &env,
                    env.current_contract_address(),
                    lower_tick_index,
                    params.pool.tick_spacing
                )
            );
        }

        let now = env.ledger().timestamp();

        let market = Market {
            name: params.name,
            oracle: params.oracle.clone(),
            collateral_token: params.collateral_token.clone(),
            synth_token: synth_token_address,
            amm: Pool {
                token_a: params.pool.token_a.clone(),
                token_b: params.pool.token_b.clone(),
                lp_token: lp_token_address,
                tick_current_index,
                tick_spacing: params.pool.tick_spacing,
                tick_arrays,
                sqrt_price: params.pool.initial_sqrt_price,
                liquidity: 0,
                fee_rate: params.pool.fee_rate,
                protocol_fee_rate: params.pool.protocol_fee_rate,
                protocol_fee_owed_a: 0,
                protocol_fee_owed_b: 0,
                fee_growth_global_a: 0,
                fee_growth_global_b: 0,
                max_allowed_slippage_bps: params.pool.max_allowed_slippage_bps,
                max_allowed_variance_bps: params.pool.max_allowed_variance_bps,
                reward_last_updated_timestamp: now,
                reward_infos: Vec::new(&env),
            },
            decimals: params.token_decimals,
            status: if params.active_status {
                MarketStatus::Active
            } else {
                MarketStatus::Initialized
            },
            synth_tier: params.synth_tier,
            paused_operations: Vec::new(&env),
            collateral_balance: 0,
            debt_balance: 0,
            cumulative_deposit_interest: 0,
            cumulative_lp_interest: 0,
            withdraw_guard_threshold: 0,
            max_token_deposits: 0,
            collateral_token_twap: 0,
            debt_token_twap: 0,
            utilization_twap: 0,
            last_interest_ts: 0,
            last_twap_ts: 0,
            expiry_timestamp: 0,
            expiry_price: 0,
            max_position_size: 0,
            next_deposit_record_id: 0,
            initial_asset_weight: 0,
            maintenance_asset_weight: 0,
            initial_liability_weight: 0,
            maintenance_liability_weight: 0,
            imf_factor: params.imf_factor,
            liquidation_penalty: params.liquidation_penalty,
            liquidator_fee: params.liquidator_fee,
            if_liquidation_fee: params.if_liquidation_fee,
            margin_ratio_initial: params.margin_ratio_initial,
            margin_ratio_maintenance: params.margin_ratio_maintenance,
            debt_ceiling: params.debt_ceiling,
            debt_floor: params.debt_floor,
            oracle_source: params.oracle_source,
            historical_oracle_data: {},
            last_oracle_conf_pct: 0,
            last_oracle_valid: false,
            last_oracle_normalised_price: 0,
            last_oracle_reserve_price_spread_pct: 0,
            oracle_std: 0,
            collateral_loan_balance: 0,
            collateralization_ratio: 0,
            synthetic_tokens_minted: 0,
            collateral_lending_utilization: 0,
            insurance_claim: InsuranceClaim::default(),
            total_gov_token_inflation: 0,
            collateral_action_config: AuctionConfig::default(),
            outstanding_debt: 0,
            protocol_debt: 0,
        };

        save_market(&env, market);

        MarketEvents::initialize_market(&env, market.name, now);
    }

    fn initialize_shutdown(env: Env, sender: Address, expiry_ts: u64) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);
        log!(&env, "updating market {} expiry", market.name);

        // Pause vault Create, Deposit, Lend, and Delete
        market.paused_operations = vec![&env, MarketOperation::Deposit];

        // MarketOperation::log_all_operations_paused(market.paused_operations);

        // TODO: freeze collateral prices

        // vault owners can withraw any excess collateral if their debt obligations are met

        validate!(
            &env,
            env.ledger().timestamp() < expiry_ts,
            ErrorCode::DefaultError,
            "Market expiry ts must later than current clock timestamp"
        )?;

        log!(&env, "market.status {} -> {}", market.status, MarketStatus::ReduceOnly);
        log!(&env, "market.expiry_ts {} -> {}", market.expiry_ts, expiry_ts);

        // automatically enter reduce only
        market.status = MarketStatus::ReduceOnly;
        market.expiry_ts = expiry_ts;

        save_market(&env, market)
    }

    // fn delete(env: Env, sender: Address) {
    //     utils::is_admin(&env, &sender, true);
    // }

    fn update_paused_operations(
        env: Env,
        sender: Address,
        paused_operations: Vec<MarketOperation>
    ) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);
        market.paused_operations = paused_operations;

        save_market(&env, market);
    }

    fn update_debt_limit(
        env: Env,
        sender: Address,
        debt_floor: Option<u32>,
        debt_ceiling: Option<u128>
    ) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);

        // TODO: validation

        if let Some(debt_floor) = debt_floor {
            market.debt_floor = debt_floor;
        }
        if let Some(debt_ceiling) = debt_ceiling {
            market.debt_ceiling = debt_ceiling;
        }

        save_market(&env, market)
    }

    fn extend_expiry_ts(env: Env, sender: Address, expiry_ts: i64) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);

        log!(&env, "updating market {} expiry", market.name);

        // TODO: validate already in reduceonly mode / shutdown
        let current_ts = env.ledger().timestamp();
        validate!(
            &env,
            current_ts < expiry_ts,
            ErrorCode::DefaultError,
            "Market expiry ts must later than current clock ts"
        )?;

        validate!(
            &env,
            current_ts < expiry_ts,
            ErrorCode::DefaultError,
            "Market expiry ts must later than current clock ts"
        )?;

        log!(&env, "market.expiry_ts {} -> {}", market.expiry_ts, expiry_ts);

        market.expiry_ts = expiry_ts;

        save_market(&env, market)
    }

    fn update_margin_config(
        env: Env,
        sender: Address,
        imf_factor: Option<u32>,
        margin_ratio_initial: Option<u32>,
        margin_ratio_maintenance: Option<u32>
    ) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);

        log!(&env, "updating market {} margin ratio", market.name);

        validate_margin(
            &env,
            margin_ratio_initial,
            margin_ratio_maintenance,
            market.liquidator_fee
        )?;

        log!(
            &env,
            "market.margin_ratio_initial: {} -> {}",
            market.margin_ratio_initial,
            margin_ratio_initial
        );

        log!(
            &env,
            "market.margin_ratio_maintenance: {} -> {}",
            market.margin_ratio_maintenance,
            margin_ratio_maintenance
        );

        market.margin_ratio_initial = margin_ratio_initial;
        market.margin_ratio_maintenance = margin_ratio_maintenance;

        if let Some(imf_factor) = imf_factor {
            validate!(
                &env,
                imf_factor <= SPOT_IMF_PRECISION,
                ErrorCode::DefaultError,
                "invalid imf factor"
            )?;

            log!(&env, "market.imf_factor: {} -> {}", market.imf_factor, imf_factor);

            market.imf_factor = imf_factor;
        }

        save_market(&env, market)
    }

    fn update_liquidation_config(
        env: Env,
        sender: Address,
        liquidator_fee: u32,
        if_liquidation_fee: u32,
        liquidation_penalty: Option<u32>
    ) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);

        log!(&env, "updating market {} liquidation fee", market.name);

        validate!(
            &env,
            liquidator_fee.safe_add(if_liquidation_fee)? < LIQUIDATION_FEE_PRECISION,
            ErrorCode::DefaultError,
            "Total liquidation fee must be less than 100%"
        )?;

        validate!(
            &env,
            if_liquidation_fee < LIQUIDATION_FEE_PRECISION,
            ErrorCode::DefaultError,
            "If liquidation fee must be less than 100%"
        )?;

        validate_margin(
            &env,
            market.margin_ratio_initial,
            market.margin_ratio_maintenance,
            liquidator_fee
        )?;

        log!(&env, "market.liquidator_fee: {} -> {}", market.liquidator_fee, liquidator_fee);

        log!(
            &env,
            "market.if_liquidation_fee: {} -> {}",
            market.if_liquidation_fee,
            if_liquidation_fee
        );

        market.liquidator_fee = liquidator_fee;
        market.if_liquidation_fee = if_liquidation_fee;

        if let Some(liquidation_penalty) = liquidation_penalty {
            log!(&env, "updating market {} liquidation penalty", market.name);

            // TODO: do we need validation?

            log!(
                &env,
                "market.liquidation_penalty: {} -> {}",
                market.liquidation_penalty,
                liquidation_penalty
            );

            market.liquidation_penalty = liquidation_penalty;
        }

        save_market(&env, market)
    }

    fn update_name(env: Env, sender: Address, name: String) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);

        log!(&env, "market.name: {} -> {}", market.name, name);
        market.name = name;

        save_market(&env, market)
    }

    fn update_status(env: Env, sender: Address, status: MarketStatus) {
        utils::is_admin(&env, &sender, true);

        validate!(
            &env,
            !matches!(status, MarketStatus::Delisted | MarketStatus::Settlement),
            ErrorCode::DefaultError,
            "must set settlement/delist through another instruction"
        )?;

        let mut market = get_market(&env);

        log!(&env, "market {}", market.name);
        log!(&env, "market.status: {} -> {}", market.status, status);
        market.status = status;

        save_market(&env, market)
    }

    fn update_synth_tier(env: Env, sender: Address, synth_tier: SynthTier) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);

        log!(&env, "market {}", market.name);
        log!(&env, "market.synth_tier: {} -> {}", market.synth_tier, synth_tier);
        market.synth_tier = synth_tier;

        save_market(&env, market)
    }

    fn reset_oracle_twap(env: Env, sender: Address) {
        utils::is_admin(&env, &sender, true);

        // admin failsafe to reset amm oracle_twap to the mark_twap

        let market = &mut get_market(&env);

        log!(&env, "resetting amm oracle twap for market {}", market.name);
        log!(
            &env,
            "market.historical_oracle_data.last_oracle_price_twap: {:?} -> {:?}",
            market.historical_oracle_data.last_oracle_price_twap,
            market.last_mark_price_twap.cast::<i64>()?
        );

        log!(
            &env,
            "market.historical_oracle_data.last_oracle_price_twap_ts: {:?} -> {:?}",
            market.historical_oracle_data.last_oracle_price_twap_ts,
            market.last_mark_price_twap_ts
        );

        market.historical_oracle_data.last_oracle_price_twap =
            market.last_mark_price_twap.cast::<i64>()?;

        market.historical_oracle_data.last_oracle_price_twap_ts = market.last_mark_price_twap_ts;
    }

    // ################################################################
    //                             Keeper
    // ################################################################

    fn update_oracle_twap(env: Env, keeper: Address) {
        keeper.require_auth();

        // allow update to amm's oracle twap iff price gap is reduced and thus more tame funding
        // otherwise if oracle error or funding flip: set oracle twap to mark twap (0 gap)

        let now = env.ledger().timestamp();

        let mut market = &mut get_market(&env);
        log!(&env, "updating amm oracle twap for market {}", market.name);
        // let price_oracle = &ctx.accounts.oracle;
        let oracle_twap = pool.get_oracle_twap(&market.oracle, now)?;

        if let Some(oracle_twap) = oracle_twap {
            let oracle_mark_gap_before = pool.last_mark_price_twap
                .cast::<i64>()?
                .safe_sub(market.historical_oracle_data.last_oracle_price_twap)?;

            let oracle_mark_gap_after = pool.last_mark_price_twap
                .cast::<i64>()?
                .safe_sub(oracle_twap)?;

            if
                (oracle_mark_gap_after > 0 && oracle_mark_gap_before < 0) ||
                (oracle_mark_gap_after < 0 && oracle_mark_gap_before > 0)
            {
                log!(
                    &env,
                    "pool.historical_oracle_data.last_oracle_price_twap {} -> {}",
                    market.historical_oracle_data.last_oracle_price_twap,
                    pool.last_mark_price_twap.cast::<i64>()?
                );
                log!(
                    &env,
                    "pool.historical_oracle_data.last_oracle_price_twap_ts {} -> {}",
                    market.historical_oracle_data.last_oracle_price_twap_ts,
                    now
                );
                market.historical_oracle_data.last_oracle_price_twap =
                    pool.last_mark_price_twap.cast::<i64>()?;
                market.historical_oracle_data.last_oracle_price_twap_ts = now;
            } else if oracle_mark_gap_after.unsigned_abs() <= oracle_mark_gap_before.unsigned_abs() {
                log!(
                    &env,
                    "pool.historical_oracle_data.last_oracle_price_twap {} -> {}",
                    market.historical_oracle_data.last_oracle_price_twap,
                    oracle_twap
                );
                log!(
                    &env,
                    "pool.historical_oracle_data.last_oracle_price_twap_ts {} -> {}",
                    market.historical_oracle_data.last_oracle_price_twap_ts,
                    now
                );
                market.historical_oracle_data.last_oracle_price_twap = oracle_twap;
                market.historical_oracle_data.last_oracle_price_twap_ts = now;
            } else {
                return Err(ErrorCode::PriceBandsBreached);
            }
        } else {
            return Err(ErrorCode::InvalidOracle);
        }
    }

    fn update_oracle(env: Env, keeper: Address, oracle: Address, oracle_source: OracleSource) {
        keeper.require_auth();

        let mut market = &mut get_market(&env);
        log!(&env, "market {}", market.name);

        let now = env.ledger().timestamp();

        // OracleMap::validate_oracle_account_info(&ctx.accounts.oracle)?;

        // Verify oracle is readable
        // let OraclePriceData {
        //     price: _oracle_price,
        //     delay: _oracle_delay,
        //     ..
        // } = get_oracle_price(&oracle_source, &ctx.accounts.oracle, now)?;

        log!(&env, "market.oracle: {:?} -> {:?}", market.oracle, oracle);

        log!(&env, "market.oracle_source: {:?} -> {:?}", market.oracle_source, oracle_source);

        market.oracle = oracle;
        market.oracle_source = oracle_source;
    }

    fn freeze_oracle(env: Env, emergency_oracle: Address) {
        emergency_oracle.require_auth();

        // is_emergency_oracle(&env, sender);
    }

    // fn liquidate_position(
    //     env: Env,
    //     liquidator: Address,
    //     user: Address,
    //     liquidator_max_base_asset_amount: u64,
    //     limit_price: Option<u64>
    // ) {
    //     liquidator.require_auth();

    //     if user == liquidator {
    //         return Err(ErrorCode::UserCantLiquidateThemself);
    //     }

    //     controller::liquidation::liquidate_position(&env);
    // }

    // fn resolve_position_bankruptcy(e: Env, sender: Address) {
    //     sender.require_auth();

    //     // ..

    //     controller::liquidation::resolve_position_bankruptcy(&env);
    // }

    // ################################################################
    //                             User
    // ################################################################

    fn deposit_collateral(env: Env, sender: Address, amount: i128) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        let mut position = get_market_position(&env, &sender);

        validate!(&env, !position.is_bankrupt(), ErrorCode::UserBankrupt)?;

        let mut market = get_market(&env);
        // let oracle_price_data = &oracle_map.get_price_data(&synth_market.oracle)?.clone();

        validate!(
            &env,
            !matches!(market.status, MarketStatus::Initialized),
            ErrorCode::MarketBeingInitialized,
            "Market is being initialized"
        )?;

        let force_reduce_only = market.is_reduce_only();

        utils::update_market_cumulative_interest();

        // ...

        // Deposit the token amount from the user into the market
        token_contract::Client
            ::new(&env, &market.collateral_token)
            .transfer(&sender, &env.current_contract_address(), &amount);

        // TODO: update the user's position
        // ...
        save_market_position(&env, &sender, &position);

        // TODO: update the market's cumulative properties
        utils::update_position_and_market_cumulative_deposits(&env, &position);

        // ...

        MarketEvents::collateral_deposit(
            &env,
            market.name,
            sender,
            market.collateral_token,
            amount
        );
    }

    fn withdraw_collateral(env: Env, sender: Address, amount: i128, reduce_only: bool) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        let mut position = get_market_position(&env, &sender);

        validate!(&env, !position.is_bankrupt(), ErrorCode::UserBankrupt)?;

        let mut market = get_market(&env);

        // ...

        MarketEvents::collateral_withdrawal(
            &env,
            market.name,
            sender,
            market.collateral_token,
            amount
        );
    }

    fn borrow_synth(env: Env, sender: Address, amount: i128) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        let mut market = get_market(&env);

        // Compute amount to mint

        let mint_amount = 0;

        if mint_amount >= max_amount_user_can_mint {
            return Err(ErrorCode::InsufficientFunds);
        }

        // Mint synth tokens
        token_contract::Client
            ::new(&env, &market.synth_token)
            .mint(&env.current_contract_address(), &mint_amount);

        MarketEvents::mint_synthetic(&env);

        // Update market numbers
        market.debt_balance += 0;
        market.synthetic_tokens_minted += mint_amount;
        market.outstanding_debt += 0;

        // Fetch the protocol LP
        let liquidity_position = get_lp(&env, market.liquidity_position_ts);

        // // Update LP tick indexes if out of range
        // if liquidity_position.is_out_of_bounds() {
        //     let (new_tick_lower_index, new_tick_upper_index) = utils::find_new_tick_indexes();

        //     let modify_lp_response: ModifyLiquidityPositionResponse = env.invoke_contract(
        //         &config.amm_contract_address,
        //         &Symbol::new(&env, "modify_position"),
        //         vec![
        //             &env,
        //             sender.into_val(&env),
        //             position_timestamp: market.liquidity_position_ts,
        //             update: PositionUpdate {
        //                 tick_lower_index: new_tick_lower_index,
        //                 tick_upper_index: new_tick_upper_index,
        //             }
        //         ]
        //     );
        // }

        // Add new liquidity to the AMM
        SynthMarket::increase_liquidity(
            &env,
            sender.into_val(&env),
            market.liquidity_position_ts,
            amount,
            0,
            0
        );

        // Update market liquidity provisioning properties
        market.debt_balance += 0;

        PoolEvents::provide_liquidity(&env);
    }
}

// ################################################################
//                             Pool
// ################################################################

#[contractimpl]
impl PoolTrait for SynthMarket {
    fn initialize_tick_array(env: Env, sender: Address, start_tick_index: i32) {
        sender.require_auth();

        let mut market = get_market(&env);

        market.amm.initiliaze_tick_array(&env, start_tick_index);
    }

    #[allow(clippy::too_many_arguments)]
    fn update_pool(
        env: Env,
        sender: Address,
        fee_rate: Option<i64>,
        protocol_fee_rate: Option<i64>,
        max_allowed_slippage_bps: Option<i64>,
        max_allowed_variance_bps: Option<i64>
    ) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);

        if let Some(fee_rate) = fee_rate {
            validate_bps!(fee_rate);
            market.amm.fee_rate = fee_rate;
        }
        if let Some(protocol_fee_rate) = protocol_fee_rate {
            validate_bps!(protocol_fee_rate);
            market.amm.protocol_fee_rate = protocol_fee_rate;
        }

        if let Some(max_allowed_slippage_bps) = max_allowed_slippage_bps {
            validate_bps!(max_allowed_slippage_bps);
            market.amm.max_allowed_slippage_bps = max_allowed_slippage_bps;
        }

        if let Some(max_allowed_variance_bps) = max_allowed_variance_bps {
            validate_bps!(max_allowed_variance_bps);
            market.amm.max_allowed_variance_bps = max_allowed_variance_bps;
        }

        save_market(&env, market);
    }

    fn initialize_reward(
        env: Env,
        sender: Address,
        reward_token: Address,
        initial_balance: i128,
        emissions_per_second_x64: u128
    ) {
        sender.require_auth();

        let market = &mut get_market(&env);
        let now = env.ledger().timestamp();

        if market.amm.get_reward_by_token(reward_token) {
            return Err(ErrorCode::AdminNotSet);
        }

        let reward = RewardInfo {
            token: reward_token,
            authority: sender,
            initial_balance,
            current_balance: initial_balance,
            emissions_per_second_x64,
            growth_global_x64: 0,
        };

        market.amm.reward_infos.append(reward);

        // Deposit initial reward token balance
        token_contract::Client
            ::new(&env, &reward_token)
            .transfer(&sender, &env.current_contract_address(), &initial_balance);
    }

    fn set_reward_emissions(
        env: Env,
        sender: Address,
        reward_token: Address,
        emissions_per_second_x64: u128
    ) {
        sender.require_auth();

        let market = get_market(&env);
        let (reward, reward_index) = market.amm.get_reward_by_token(reward_token)?;

        let emissions_per_day = math::bit_math::checked_mul_shift_right(
            DAY_IN_SECONDS,
            emissions_per_second_x64
        )?;
        if reward.current_balance < emissions_per_day {
            return Err(ErrorCode::RewardVaultAmountInsufficient);
        }

        let timestamp = env.ledger().timestamp();
        let next_reward_infos = controller::pool::next_amm_reward_infos(&market.amm, timestamp)?;

        market.amm.update_rewards(next_reward_infos, timestamp);
        market.amm.reward_infos.set(reward_index, RewardInfo {
            emissions_per_second_x64,
            ..reward
        })
    }

    fn set_reward_authority(
        env: Env,
        sender: Address,
        reward_token: Address,
        new_reward_authority: Address
    ) {
        sender.require_auth();

        let mut market = get_market(&env);

        market.amm.update_reward_authority(reward_token, new_reward_authority);
    }

    // ################################################################
    //                             Keeper
    // ################################################################

    fn collect_protocol_fees(env: Env, sender: Address) {
        sender.require_auth();

        let mut market = get_market(&env);

        // TODO: distribute revenue to the appropriate locations instead of the sender
        token_contract::Client
            ::new(&env, &market.amm.token_a)
            .transfer(&env.current_contract_address(), &sender, market.amm.protocol_fee_owed_a);
        token_contract::Client
            ::new(&env, &market.amm.token_b)
            .transfer(&env.current_contract_address(), &sender, market.amm.protocol_fee_owed_b);

        market.amm.reset_protocol_fees_owed();
    }

    // ################################################################
    //                             User
    // ################################################################

    fn create_position(env: Env, sender: Address, tick_lower_index: i32, tick_upper_index: i32) {
        sender.require_auth();

        let mut market = get_market(&env);
        let mut position = get_liquidity_position_info(&env, &sender);
        // posi
        position.open_position(&env, &market.amm, tick_lower_index, tick_upper_index)?;
    }

    fn modify_position(
        env: Env,
        sender: Address,
        position_ts: u64,
        update: LiquidityPositionUpdate
    ) {
        sender.require_auth();

        let mut position = get_liquidity_position_by_ts(&env, &sender, position_ts)?;

        position.update(&update);
    }

    fn close_position(env: Env, sender: Address, position_ts: u64) {
        sender.require_auth();

        let mut position = get_liquidity_position_by_ts(&env, &sender, position_ts)?;

        if !position.is_position_empty() {
            return Err(ErrorCode::ClosePositionNotEmpty);
        }

        let market = get_market(&env);

        token_contract::Client::new(&env, &market.amm.lp_token).burn(&sender, 1);
    }

    fn increase_liquidity(
        env: Env,
        sender: Address,
        position_ts: u64,
        liquidity_amount: u128,
        token_max_a: u64,
        token_max_b: u64,
        tick_array_lower_index: i32,
        tick_array_upper_index: i32
    ) {
        sender.require_auth();

        if liquidity_amount == 0 {
            return Err(ErrorCode::LiquidityZero);
        }

        let market = get_market(&env);
        let position = get_liquidity_position_by_ts(&env, &sender, position_ts)?;

        let tick_array_lower = match market.amm.tick_arrays.get(tick_array_lower_index) {
            Some(ta) => ta,
            None => {
                return Err(ErrorCode::AdminNotSet);
            }
        };
        let tick_array_upper = match market.amm.tick_arrays.get(tick_array_upper_index) {
            Some(ta) => ta,
            None => {
                return Err(ErrorCode::AdminNotSet);
            }
        };

        let liquidity_delta = math::liquidity_math::convert_to_liquidity_delta(
            liquidity_amount,
            true
        )?;
        let timestamp = env.ledger().timestamp();

        let update = controller::liquidity::calculate_modify_liquidity(
            &env,
            &market.amm,
            &position,
            &tick_array_lower,
            &tick_array_upper,
            liquidity_delta,
            timestamp
        )?;

        controller::liquidity::sync_modify_liquidity_values(
            &mut market.amm,
            &mut position,
            &mut tick_array_lower,
            &mut tick_array_upper,
            update,
            timestamp
        )?;

        let (delta_a, delta_b) = controller::liquidity::calculate_liquidity_token_deltas(
            market.amm.tick_current_index,
            market.amm.sqrt_price,
            &position,
            liquidity_delta
        )?;

        if delta_a > token_max_a || delta_b > token_max_b {
            return Err(ErrorCode::TokenMaxExceeded);
        }

        token_contract::Client
            ::new(&env, &market.amm.token_a)
            .transfer(&sender, &env.current_contract_address(), &delta_a);

        token_contract::Client
            ::new(&env, &market.amm.token_b)
            .transfer(&sender, &env.current_contract_address(), &delta_b);

        // utils::mint_shares(&env, &market.amm.lp_token, &env.current_contract_address(), 1);

        PoolEvents::increase_liquidity(
            &env,
            &sender,
            market.amm.token_a,
            market.amm.token_b,
            delta_a,
            delta_b
        );
    }

    fn decrease_liquidity(
        env: Env,
        sender: Address,
        position_ts: u64,
        liquidity_amount: u128,
        token_max_a: u64,
        token_max_b: u64,
        tick_array_lower_index: i32,
        tick_array_upper_index: i32
    ) {
        sender.require_auth();

        if liquidity_amount == 0 {
            return Err(ErrorCode::LiquidityZero);
        }
        let liquidity_delta = math::liquidity_math::convert_to_liquidity_delta(
            liquidity_amount,
            true
        )?;
        let timestamp = env.ledger().timestamp();

        let market = get_market(&env);
        let position = get_liquidity_position_by_ts(&env, &sender, position_ts)?;

        let tick_array_lower = match market.amm.tick_arrays.get(tick_array_lower_index) {
            Some(ta) => ta,
            None => {
                return Err(ErrorCode::AdminNotSet);
            }
        };
        let tick_array_upper = match market.amm.tick_arrays.get(tick_array_upper_index) {
            Some(ta) => ta,
            None => {
                return Err(ErrorCode::AdminNotSet);
            }
        };

        let update = controller::liquidity::calculate_modify_liquidity(
            &env,
            &market.amm,
            &position,
            &tick_array_lower,
            &tick_array_upper,
            liquidity_delta,
            timestamp
        )?;

        controller::liquidity::sync_modify_liquidity_values(
            &mut market.amm,
            &mut position,
            &mut tick_array_lower,
            &mut tick_array_upper,
            update,
            timestamp
        )?;

        let (delta_a, delta_b) = controller::liquidity::calculate_liquidity_token_deltas(
            market.amm.tick_current_index,
            market.amm.sqrt_price,
            &position,
            liquidity_delta
        )?;

        if delta_a < token_max_a || delta_b < token_max_b {
            return Err(ErrorCode::TokenMinSubceeded);
        }

        token_contract::Client
            ::new(&env, &market.amm.token_a)
            .transfer(&env.current_contract_address(), &sender, &delta_a);

        token_contract::Client
            ::new(&env, &market.amm.token_b)
            .transfer(&env.current_contract_address(), &sender, &delta_b);

        PoolEvents::remove_liquidity(
            &env,
            &sender,
            market.amm.token_a,
            market.amm.token_b,
            delta_a,
            delta_b
        );
    }

    fn swap(
        env: Env,
        sender: Address,
        amount: u64,
        other_amount_threshold: u64,
        sqrt_price_limit: u128,
        amount_specified_is_input: bool,
        a_to_b: bool, // Zero for one,
        tick_array_indexes: Vec<i32>
    ) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let timestamp = env.ledger().timestamp();
        let market = get_market(&env);

        controller::pool::update_pool_price(&env, &market.amm);

        let tick_arrays = tick_array_indexes
            .into_iter()
            .map(|index| {
                let tick_array = match market.amm.tick_arrays.get(index) {
                    Some(ta) => ta,
                    None => {
                        return Err(ErrorCode::AdminNotSet);
                    }
                };
                tick_array
            })
            .collect();

        let builder = SparseSwapTickSequenceBuilder::try_from(
            &env,
            &market.amm,
            a_to_b,
            tick_arrays,
            None
        )?;
        let mut swap_tick_sequence = builder.build(&env)?;

        let swap_update = controller::swap::swap(
            &env,
            &market.amm,
            &mut swap_tick_sequence,
            amount,
            sqrt_price_limit,
            amount_specified_is_input,
            a_to_b,
            timestamp
        )?;

        if amount_specified_is_input {
            if
                (a_to_b && other_amount_threshold > swap_update.amount_quote) ||
                (!a_to_b && other_amount_threshold > swap_update.amount_synthetic)
            {
                return Err(ErrorCode::AmountOutBelowMinimum);
            }
        } else if
            (a_to_b && other_amount_threshold < swap_update.amount_synthetic) ||
            (!a_to_b && other_amount_threshold < swap_update.amount_quote)
        {
            return Err(ErrorCode::AmountInAboveMaximum);
        }

        update_and_swap_amm(&env, &market.amm, sender, swap_update, a_to_b, timestamp);

        PoolEvents::swap(&env, sender, market.amm.token_a, market.amm.token_b, amount, 0, 0);
    }

    fn collect_fees(env: Env, sender: Address, position_ts: u64) {
        sender.require_auth();

        let market = get_market(&env);
        let mut position = get_liquidity_position_by_ts(&env, &sender, position_ts);

        // Store the fees owed to use as transfer amounts.
        let fee_owed_a = position.fee_owed_a;
        let fee_owed_b = position.fee_owed_b;

        position.reset_fees_owed();

        token_contract::Client
            ::new(&env, &market.amm.token_a)
            .transfer(&env.current_contract_address(), &sender, &fee_owed_a);

        token_contract::Client
            ::new(&env, &market.amm.token_b)
            .transfer(&env.current_contract_address(), &sender, &fee_owed_b);

        PoolEvents::collect_fees(&env, sender, fee_owed_a, fee_owed_b);
    }

    fn collect_reward(env: Env, sender: Address, reward_token: Address, position_ts: u64) {
        sender.require_auth();

        let market = get_market(&env);
        let position = &mut get_liquidity_position_by_ts(&env, &sender, position_ts)?;

        let (reward, reward_index) = market.amm.get_reward_by_token(reward_token)?;
        let (transfer_amount, updated_amount_owed) = calculate_collect_reward(
            position.reward_infos[index],
            reward.current_balance
        );

        position.update_reward_owed(index, updated_amount_owed);

        token_contract::Client
            ::new(&env, &reward_token)
            .transfer(&env.current_contract_address(), &sender, &transfer_amount);

        // TODO: decrement the market's available reward balance
        reward.current_balance -= transfer_amount;
    }

    // ################################################################
    //                             Queries
    // ################################################################

    // fn query_lp_token_address(env: Env) -> Address {
    //     env.storage()
    //         .instance()
    //         .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

    //     get_pool(&env).lp_token
    // }
}

// TODO: do we need to update the AMM?
pub fn update_amm_and_check_validity(
    market: &mut PerpMarket,
    oracle_price_data: &OraclePriceData,
    state: &State,
    now: i64,
    action: Option<DriftAction>
) -> NormalResult {
    // _update_amm(market, oracle_price_data, state, now, clock_slot)?;

    // 1 hour EMA
    let risk_ema_price = market.amm.historical_oracle_data.last_oracle_price_twap;

    let oracle_validity = oracle_validity(
        market.name,
        risk_ema_price,
        oracle_price_data,
        oracle_guard_rails().validity, // import from Oracle module
        market.get_max_confidence_interval_multiplier()?,
        false
    )?;

    validate!(
        is_oracle_valid_for_action(oracle_validity, action)?,
        ErrorCode::InvalidOracle,
        "Invalid Oracle ({:?} vs ema={:?}) for perp market index={} and action={:?}",
        oracle_price_data,
        risk_ema_price,
        market.name,
        action
    )?;

    Ok(())
}
