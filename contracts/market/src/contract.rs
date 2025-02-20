use normal::{
    constants::{
        DAY_IN_SECONDS, INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD,
        LIQUIDATION_FEE_PRECISION, SPOT_IMF_PRECISION,
    },
    get_then_update_id,
    math::{casting::Cast, oracle::NormalAction, safe_math::SafeMath},
    oracle::{
        get_band_price, get_oracle_price, HistoricalOracleData, OraclePriceData, OracleSource,
    },
    types::{
        auction::Auction,
        market::{MarketFactoryConfig, MarketInfo, MarketParams, MarketResponse, SynthTier},
    },
    validate, validate_bps,
};
use soroban_sdk::{
    contract, contractimpl, contractmeta, log, panic_with_error, symbol_short, vec, Address,
    BytesN, Env, FromVal, Map, String, Symbol, Vec,
};

use crate::{
    controller,
    errors::Errors,
    events::{MarketEvents, PoolEvents},
    interface::{market::MarketTrait, pool::PoolTrait},
    math::{
        self,
        balance::BalanceType,
        liquidation::is_position_being_liquidated,
        margin::{
            calculate_calculate_max_mintable_amount, calculate_max_withdrawable_amount,
            MarginRequirementType,
        },
    },
    state::{
        liquidity_position::{
            get_liquidity_position_by_ts, get_liquidity_position_info, LiquidityPositionUpdate,
        },
        market::{
            get_market, save_market, Collateral, InsuranceClaim, Market, MarketOperation,
            MarketStatus,
        },
        market_position::{get_market_position, save_market_position},
        pool::Pool,
        reward::{calculate_collect_reward, RewardInfo},
        tick_array::TickArray,
    },
    storage::utils::{self, get_admin, get_factory},
    token_contract,
    utils::{sparse_swap::SparseSwapTickSequenceBuilder, swap_utils::update_and_swap_amm},
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
        params: MarketParams,
        synth_token_addr: Address,
        lp_token_addr: Address,
        factory_addr: Address,
        insurance_addr: Address,
    ) {
        if utils::is_initialized(&env) {
            log!(
                &env,
                "Market: Initialize: initializing contract twice is not allowed"
            );
            panic_with_error!(&env, Errors::AlreadyInitialized);
        }

        let now = env.ledger().timestamp();

        // Verify oracle is readable
        let (oracle_price, oracle_delay) = match params.oracle_source {
            OracleSource::Band => {
                let OraclePriceData {
                    price: oracle_price,
                    delay: oracle_delay,
                    ..
                } = get_band_price(&env, &params.oracle, (), now);
                // let last_oracle_price_twap = get_band_twap(&env, params.oracle)?;
                (oracle_price, oracle_delay)
            }
        };

        validate_margin(
            &env,
            params.margin_ratio_initial,
            params.margin_ratio_maintenance,
            params.liquidator_fee,
        );

        utils::set_initialized(&env);
        utils::save_factory(&env, &factory_addr);

        // Initialize core group of TickArrays
        let tick_current_index =
            math::tick_math::tick_index_from_sqrt_price(&params.pool.initial_sqrt_price);

        let mut tick_arrays: Map<i32, TickArray> = Map::new(&env);
        tick_arrays.set(
            tick_current_index,
            TickArray::new(
                &env,
                env.current_contract_address(),
                tick_current_index,
                params.pool.tick_spacing,
            ),
        );
        let tick_steps: i32 = 10;

        for i in 0..tick_steps {
            let upper_tick_index = params.pool.tick_spacing.safe_mul(i, &env);
            let lower_tick_index = params.pool.tick_spacing.safe_mul(-i, &env);

            tick_arrays.set(
                upper_tick_index,
                TickArray::new(
                    &env,
                    env.current_contract_address(),
                    upper_tick_index,
                    params.pool.tick_spacing,
                ),
            );
            tick_arrays.set(
                lower_tick_index,
                TickArray::new(
                    &env,
                    env.current_contract_address(),
                    lower_tick_index,
                    params.pool.tick_spacing,
                ),
            );
        }

        let market = Market {
            name: params.name,
            synth_token: synth_token_address,
            collateral: Collaterat {
                token: params.quote_token.clone(),
                oracle: params.oracle.clone(),
                oracle_source: params.oracle_source,
                balance: 0,
                token_twap: 0,
                margin_ratio_initial: params.margin_ratio_initial,
                margin_ratio_maintenance: params.margin_ratio_maintenance,
                loan_balance: 0,
                c_ratio: 0,
                lending_utilization: 0,
                auction_config: Auction::default(),
            },
            amm: Pool {
                token_a: synth_token_address,
                token_b: params.quote_token,
                lp_token: lp_token_address,
                tick_current_index,
                tick_spacing: params.pool.tick_spacing,
                tick_arrays,

                // Oracle
                oracle: params.pool.oracle.clone(),
                oracle_source: params.pool.oracle_source,
                historical_oracle_data: HistoricalOracleData::default(),
                last_oracle_conf_pct: 0,
                last_oracle_valid: false,
                last_oracle_normalised_price: 0,
                last_oracle_price_spread_pct: 0,
                oracle_std: 0,
                last_price_twap_5min: 0,
                last_price_twap_ts: 0,
                last_update_slot: now,

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
            synth_tier: params.tier,
            paused_operations: Vec::new(&env),

            debt_balance: 0,
            cumulative_deposit_interest: 0,
            cumulative_lp_interest: 0,
            withdraw_guard_threshold: 0,
            max_token_deposits: 0,

            debt_token_twap: 0,
            utilization_twap: 0,
            last_interest_ts: 0,
            last_twap_ts: 0,
            expiry_ts: 0,
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

            debt_ceiling: params.debt_ceiling,
            debt_floor: params.debt_floor,

            synthetic_tokens_minted: 0,

            // Insurance
            insurance: insurance_addr,
            insurance_claim: InsuranceClaim::default(),
            total_gov_token_inflation: 0,

            outstanding_debt: 0,
            protocol_debt: 0,

            lp_ts: 0,
            last_lp_rebalance_ts: 0,
        };

        save_market(&env, market);

        MarketEvents::initialize_market(&env, market.name, now);
    }

    fn initialize_shutdown(env: Env, sender: Address, expiry_ts: u64) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);
        log!(&env, "updating market {} expiry", market.name);

        market.paused_operations = vec![
            &env,
            MarketOperation::Deposit,
            MarketOperation::Withdraw,
            MarketOperation::Lend,
            MarketOperation::Delete,
        ];

        MarketOperation::log_all_operations_paused(&env, market.paused_operations);

        // TODO: freeze collateral prices

        // vault owners can withraw any excess collateral if their debt obligations are met

        validate!(
            &env,
            env.ledger().timestamp() < expiry_ts,
            Errors::DefaultError,
            "Market expiry ts must later than current clock timestamp"
        );

        log!(
            &env,
            "market.status {} -> {}",
            market.status,
            MarketStatus::ReduceOnly
        );
        log!(
            &env,
            "market.expiry_ts {} -> {}",
            market.expiry_ts,
            expiry_ts
        );

        // automatically enter reduce only
        market.status = MarketStatus::ReduceOnly;
        market.expiry_ts = expiry_ts;

        save_market(&env, market)
    }

    fn update_paused_operations(
        env: Env,
        sender: Address,
        paused_operations: Vec<MarketOperation>,
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
        debt_ceiling: Option<u128>,
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

    fn extend_expiry_ts(env: Env, sender: Address, expiry_ts: u64) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);

        log!(&env, "updating market {} expiry", market.name);

        // TODO: validate already in reduceonly mode / shutdown
        let current_ts = env.ledger().timestamp();
        validate!(
            &env,
            current_ts < expiry_ts,
            Errors::DefaultError,
            "Market expiry ts must later than current clock ts"
        );

        validate!(
            &env,
            current_ts < expiry_ts,
            Errors::DefaultError,
            "Market expiry ts must later than current clock ts"
        );

        log!(
            &env,
            "market.expiry_ts {} -> {}",
            market.expiry_ts,
            expiry_ts
        );

        market.expiry_ts = expiry_ts;

        save_market(&env, market)
    }

    fn update_margin_config(
        env: Env,
        sender: Address,
        margin_ratio_initial: u32,
        margin_ratio_maintenance: u32,
        imf_factor: Option<u32>,
    ) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);

        log!(&env, "updating market {} margin ratio", market.name);

        validate_margin(
            &env,
            margin_ratio_initial,
            margin_ratio_maintenance,
            market.liquidator_fee,
        );

        log!(
            &env,
            "market.margin_ratio_initial: {} -> {}",
            market.collateral.margin_ratio_initial,
            margin_ratio_initial
        );

        log!(
            &env,
            "market.margin_ratio_maintenance: {} -> {}",
            market.collateral.margin_ratio_maintenance,
            margin_ratio_maintenance
        );

        market.collateral.margin_ratio_initial = margin_ratio_initial;
        market.collateral.margin_ratio_maintenance = margin_ratio_maintenance;

        if let Some(imf_factor) = imf_factor {
            validate!(
                &env,
                imf_factor <= SPOT_IMF_PRECISION,
                Errors::DefaultError,
                "invalid imf factor"
            );

            log!(
                &env,
                "market.imf_factor: {} -> {}",
                market.imf_factor,
                imf_factor
            );

            market.imf_factor = imf_factor;
        }

        save_market(&env, market)
    }

    fn update_liquidation_config(
        env: Env,
        sender: Address,
        liquidator_fee: u32,
        if_liquidation_fee: u32,
        liquidation_penalty: Option<u32>,
    ) {
        utils::is_admin(&env, &sender, true);

        let mut market = get_market(&env);

        log!(&env, "updating market {} liquidation fee", market.name);

        validate!(
            &env,
            liquidator_fee.safe_add(if_liquidation_fee, &env) < LIQUIDATION_FEE_PRECISION,
            Errors::DefaultError,
            "Total liquidation fee must be less than 100%"
        );

        validate!(
            &env,
            if_liquidation_fee < LIQUIDATION_FEE_PRECISION,
            Errors::DefaultError,
            "If liquidation fee must be less than 100%"
        );

        validate_margin(
            &env,
            market.collateral.margin_ratio_initial,
            market.collateral.margin_ratio_maintenance,
            liquidator_fee,
        );

        log!(
            &env,
            "market.liquidator_fee: {} -> {}",
            market.liquidator_fee,
            liquidator_fee
        );

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
            Errors::DefaultError,
            "must set settlement/delist through another instruction"
        );

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
        log!(
            &env,
            "market.tier: {} -> {}",
            market.synthetic.tier,
            synth_tier
        );
        market.synthetic.tier = synth_tier;

        save_market(&env, market)
    }

    // ################################################################
    //                          Super Keeper
    // ################################################################

    fn update_collateral_oracle(
        env: Env,
        super_keeper: Address,
        oracle: Address,
        oracle_source: OracleSource,
    ) {
        super_keeper.require_auth();
        utils::validate_super_keeper(&env, &super_keeper);

        let mut market = &mut get_market(&env);
        log!(&env, "market {}", market.name);

        let now = env.ledger().timestamp();

        // Verify oracle is readable
        // let OraclePriceData {
        //     price: _oracle_price,
        //     delay: _oracle_delay,
        //     ..
        // } = get_oracle_price(&oracle_source, &ctx.accounts.oracle, now)?;

        log!(
            &env,
            "market.collateral.oracle: {:?} -> {:?}",
            market.collateral.oracle,
            oracle
        );
        log!(
            &env,
            "market.collateral.oracle_source: {:?} -> {:?}",
            market.collateral.oracle_source,
            oracle_source
        );

        market.collateral.oracle = oracle;
        market.collateral.oracle_source = oracle_source;
    }

    fn update_collateral_oracle_freeze(env: Env, keeper: Address, frozen: bool) {
        keeper.require_auth();
        utils::validate_super_keeper(&env, &super_keeper);

        let mut market = &mut get_market(&env);
        log!(&env, "market {}", market.name);

        log!(
            &env,
            "market.collateral.oracle_frozen: {:?} -> {:?}",
            market.collateral.oracle_frozen,
            frozen
        );

        market.collateral.oracle_frozen = frozen;
        // TODO: do we need to freeze price?
    }

    // ################################################################
    //                             Keeper
    // ################################################################

    fn settle_revenue(env: Env, keeper: Address) {
        keeper.require_auth();

        /**
         * Revenue is
         */
        let market = &mut get_market(&env);

        // validate!(
        //     insurance_fund.revenue_settle_period > 0,
        //     Errors::RevenueSettingsCannotSettleToIF,
        //     "invalid revenue_settle_period settings on market"
        // );

        let market_vault_amount = 0;
        // let insurance_vault_amount = 0;

        let now = env.ledger().timestamp();

        // env.invoke_contract(&market.insurnace, func, args);

        // let time_until_next_update = math::helpers::on_the_hour_update(
        //     now,
        //     &market.insurnace.last_revenue_settle_ts,
        //     &market.insurnace.revenue_settle_period
        // );

        // validate!(
        //     time_until_next_update == 0,
        //     Errors::RevenueSettingsCannotSettleToIF,
        //     "Must wait {} seconds until next available settlement time",
        //     time_until_next_update
        // );

        // // uses proportion of revenue pool allocated to insurance fund
        // let token_amount = controller::insurance::settle_revenue_to_insurance_fund(
        //     spot_vault_amount,
        //     insurance_vault_amount,
        //     market,
        //     now,
        //     true
        // );

        // insurance_fund.last_revenue_settle_ts = now;

        // token_contract::Client
        //     ::new(&env, address)
        //     .transfer(&env.current_contract_address(), &insurance_fund, token_amount);

        // math::spot_withdraw::validate_spot_market_vault_amount(
        //     spot_market,
        //     ctx.accounts.spot_market_vault.amount
        // );
    }

    fn liquidate_position(
        env: Env,
        liquidator: Address,
        user: Address,
        max_base_asset_amount: u64,
        limit_price: Option<u64>,
    ) {
        liquidator.require_auth();

        if user == liquidator {
            panic_with_error!(env, Errors::UserCantLiquidateThemself);
        }

        let now = env.ledger().timestamp();
        let mut market = &mut get_market(&env);

        controller::liquidation::liquidate_position(
            &env,
            &market,
            max_base_asset_amount,
            limit_price,
            &user,
            &liquidator,
            now,
        );
    }

    fn resolve_position_bankruptcy(env: Env, sender: Address) {
        sender.require_auth();

        // ..

        controller::liquidation::resolve_position_bankruptcy(&env);
    }

    // ################################################################
    //                             User
    // ################################################################

    fn deposit_collateral(env: Env, sender: Address, amount: i128, reduce_only: bool) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        // Validate position
        let mut position = &mut get_market_position(&env, &sender);

        validate!(&env, !position.is_bankrupt(), Errors::PositionBankrupt);

        // Validate market
        let mut market = get_market(&env);

        let now = env.ledger().timestamp();
        let collateral_oracle_price_data = &get_oracle_price(
            &env,
            &market.collateral.oracle_source,
            &market.collateral.oracle,
            (market.collateral.symbol, symbol_short!("USD")),
            now,
        )
        .clone();
        let debt_oracle_price_data = &get_oracle_price(
            &env,
            &market.amm.oracle_source,
            &market.amm.oracle,
            (market.synthetic.symbol, symbol_short!("USD")),
            now,
        )
        .clone();

        validate!(
            &env,
            !matches!(market.status, MarketStatus::Initialized),
            Errors::MarketBeingInitialized,
            "Market is being initialized"
        );

        validate!(
            &env,
            !market.is_operation_paused(MarketOperation::Deposit),
            Errors::MarketOperationPaused,
            "Market collateral deposits paused"
        );

        controller::balance::update_market_twap_stats(
            &env,
            &mut market,
            collateral_oracle_price_data,
            debt_oracle_price_data,
            now,
        );

        let is_borrow_before = position.is_borrow();

        let force_reduce_only = market.is_reduce_only();

        // if reduce only, have to compare ix amount to current borrow amount
        let amount =
            if (force_reduce_only || reduce_only) && position.balance_type == BalanceType::Borrow {
                position
                    .get_token_amount(&env, &market)
                    .cast::<u64>(&env)
                    .min(amount)
            } else {
                amount
            };

        position.increment_total_deposits(
            &env,
            amount,
            collateral_oracle_price_data.price,
            market.get_precision().cast(&env),
        );

        let total_deposits_after = position.total_deposits;
        let total_withdraws_after = position.total_withdraws;

        // Update the position
        controller::market_position::update_balances_and_cumulative_deposits(
            &env,
            amount as u128,
            &BalanceType::Deposit,
            &mut market,
            &mut position,
            false,
            None,
        );

        let token_amount = position.get_token_amount(&env, &market);
        if token_amount == 0 {
            validate!(
                &env,
                position.scaled_balance == 0,
                Errors::InvalidPosition,
                "deposit left user with invalid position. scaled balance = {} token amount = {}",
                position.scaled_balance,
                token_amount
            );
        }

        if position.is_being_liquidated() {
            // try to update liquidation status if position is was already being liq'd
            let is_being_liquidated = is_position_being_liquidated(
                &env,
                &position,
                state.liquidation_margin_buffer_ratio,
            );

            if !is_being_liquidated {
                position.exit_liquidation();
            }
        }

        position.update_last_active_ts(now);

        // Deposit the collateral token
        token_contract::Client::new(&env, &market.collateral.token).transfer(
            &sender,
            &env.current_contract_address(),
            &amount,
        );

        let deposit_record_id = get_then_update_id!(market, next_deposit_record_id);
        // let oracle_price = oracle_price_data.price;

        MarketEvents::collateral_deposit(
            &env,
            market.name,
            sender,
            market.collateral.token,
            amount,
        );

        market.validate_max_token_deposits_and_borrows(&env, false);
    }

    fn withdraw_collateral(env: Env, sender: Address, amount: i128, reduce_only: bool) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        let mut position = get_market_position(&env, &sender);

        validate!(&env, !position.is_bankrupt(), Errors::PositionBankrupt);

        let mut market = get_market(&env);

        validate!(
            &env,
            !market.is_operation_paused(MarketOperation::Withdraw),
            Errors::MarketOperationPaused,
            "Market collateral withdrawals paused"
        );

        let now = env.ledger().timestamp();
        let collateral_oracle_price_data = &get_oracle_price(
            &env,
            &market.collateral.oracle_source,
            &market.collateral.oracle,
            (market.collateral.symbol, symbol_short!("USD")),
            now,
        )
        .clone();
        let debt_oracle_price_data = &get_oracle_price(
            &env,
            &market.amm.oracle_source,
            &market.amm.oracle,
            (market.synthetic.symbol, symbol_short!("USD")),
            now,
        )
        .clone();

        let market_is_reduce_only = {
            controller::balance::update_market_twap_stats(
                &env,
                &mut market,
                collateral_oracle_price_data,
                debt_oracle_price_data,
                now,
            );

            market.is_reduce_only()
        };

        let amount = {
            let reduce_only = reduce_only || market_is_reduce_only;

            let mut amount = if reduce_only {
                validate!(
                    &env,
                    position.balance_type == BalanceType::Deposit,
                    Errors::ReduceOnlyWithdrawIncreasedRisk
                );

                let max_withdrawable_amount =
                    calculate_max_withdrawable_amount(&env, market, &position);

                let existing_deposit_amount =
                    position.get_token_amount(&env, &market).cast::<u64>(&env);

                amount
                    .min(max_withdrawable_amount)
                    .min(existing_deposit_amount)
            } else {
                amount
            };

            position.increment_total_withdraws(
                &env,
                amount,
                collateral_oracle_price_data.price,
                market.get_precision().cast(&env),
            );

            // prevents withdraw when limits hit
            controller::market_position::update_balances_and_cumulative_deposits_with_limits(
                &env,
                amount as u128,
                &BalanceType::Borrow,
                &mut market,
                &mut position,
            );

            amount
        };

        position.meets_withdraw_margin_requirement(
            &env,
            MarginRequirementType::Initial,
            amount as u128,
            now,
        );

        // validate_spot_margin_trading(user, &perp_market_map, &spot_market_map, &mut oracle_map)?;

        if position.is_being_liquidated() {
            position.exit_liquidation();
        }

        position.update_last_active_ts(now);

        // ....

        MarketEvents::collateral_withdrawal(
            &env,
            market.name,
            sender,
            market.collateral.token,
            amount,
        );
    }

    fn borrow_and_increase_liquidity(env: Env, sender: Address, amount: i128) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        let mut market = get_market(&env);

        validate!(
            &env,
            !matches!(market.status, MarketStatus::Initialized),
            Errors::MarketBeingInitialized,
            "Market is being initialized"
        );

        validate!(
            &env,
            !market.is_operation_paused(MarketOperation::Borrow),
            Errors::MarketOperationPaused,
            "Market debt borrowing paused"
        );

        let mut position = &mut get_market_position(&env, &sender);

        validate!(
            &env,
            !position.is_being_liquidated(),
            Errors::PositionBankrupt
        );

        // validate debt ceiling floor

        let max_mintable_amount = calculate_calculate_max_mintable_amount(&env, market, &position);

        validate!(
            &env,
            amount <= max_mintable_amount,
            Errors::InsufficientFunds,
            "Cannot mint that much"
        );

        validate!(
            &env,
            position.debt_balance.safe_add(amount, &env) <= market.synthetic.max_position_size,
            Errors::MaxPositionSize,
            "Max debt position size reached"
        );

        // update posiiton and market

        let now = env.ledger().timestamp();
        position.update_last_active_ts(now);

        token_contract::Client::new(&env, &market.synthetic.token)
            .mint(&env.current_contract_address(), &amount);

        MarketEvents::mint_synthetic(&env);

        controller::market_position::provide_liquidity(&env, &market, &position);

        MarketEvents::mint_synthetic(&env);

        //     let liquidity_position = get_liquidity_position_by_ts(&env, market.liquidity_position_ts);

        //     // // Update LP tick indexes if out of range
        //     // if liquidity_position.is_out_of_bounds() {
        //     //     let (new_tick_lower_index, new_tick_upper_index) = utils::find_new_tick_indexes();

        //     //     let modify_lp_response: ModifyLiquidityPositionResponse = env.invoke_contract(
        //     //         &config.amm_contract_address,
        //     //         &Symbol::new(&env, "modify_position"),
        //     //         vec![
        //     //             &env,
        //     //             sender.into_val(&env),
        //     //             position_timestamp: market.liquidity_position_ts,
        //     //             update: PositionUpdate {
        //     //                 tick_lower_index: new_tick_lower_index,
        //     //                 tick_upper_index: new_tick_upper_index,
        //     //             }
        //     //         ]
        //     //     );
        //     // }
    }

    fn remove_liquidity_and_repay(env: Env, sender: Address, amount: i128) {}

    // ################################################################
    //                             Queries
    // ################################################################

    fn query_market(env: Env) -> Market {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        get_market(&env)
    }

    fn query_synth_token_address(env: Env) -> Address {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        get_market(&env).synthetic.token
    }

    fn query_lp_contract_address(env: Env) -> Address {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        get_market(&env).amm.lp_token
    }

    fn query_market_info(env: Env) -> MarketResponse {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let market = get_market(&env);

        MarketResponse { name: market.name }
    }

    fn query_market_info_for_factory(env: Env) -> MarketInfo {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let market = get_market(&env);
        let market_response = MarketResponse { name: market.name };

        MarketInfo {
            market_address: env.current_contract_address(),
            market_response,
        }
    }

    // fn migrate_admin_key(env: Env) -> Result<(), ErrorCode> {
    //     let admin = get_admin(&env);
    //     env.storage().instance().set(&ADMIN, &admin);
    //     Ok(())
    // }
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
        max_allowed_variance_bps: Option<i64>,
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
        emissions_per_second_x64: u128,
    ) {
        sender.require_auth();

        let market = &mut get_market(&env);
        let now = env.ledger().timestamp();

        if market.amm.get_reward_by_token(reward_token) {
            panic_with_error!(env, Errors::AdminNotSet);
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
        token_contract::Client::new(&env, &reward_token).transfer(
            &sender,
            &env.current_contract_address(),
            &initial_balance,
        );
    }

    fn set_reward_emissions(
        env: Env,
        sender: Address,
        reward_token: Address,
        emissions_per_second_x64: u128,
    ) {
        sender.require_auth();

        let market = get_market(&env);
        let (reward, reward_index) = market.amm.get_reward_by_token(reward_token)?;

        let emissions_per_day =
            math::bit_math::checked_mul_shift_right(DAY_IN_SECONDS, emissions_per_second_x64);
        if reward.current_balance < emissions_per_day {
            panic_with_error!(&env, Errors::RewardVaultAmountInsufficient);
        }

        let timestamp = env.ledger().timestamp();
        let next_reward_infos = controller::pool::next_amm_reward_infos(&market.amm, timestamp)?;

        market.amm.update_rewards(next_reward_infos, timestamp);
        market.amm.reward_infos.set(
            reward_index,
            RewardInfo {
                emissions_per_second_x64,
                ..reward
            },
        )
    }

    fn set_reward_authority(
        env: Env,
        sender: Address,
        reward_token: Address,
        new_reward_authority: Address,
    ) {
        sender.require_auth();

        let mut market = get_market(&env);

        market
            .amm
            .update_reward_authority(reward_token, new_reward_authority);
    }

    fn reset_oracle_twap(env: Env, sender: Address) {
        utils::is_admin(&env, &sender, true);

        // admin failsafe to reset amm oracle_twap to the mark_twap

        let market = &mut get_market(&env);

        log!(&env, "resetting amm oracle twap for market {}", market.name);
        log!(
            &env,
            "market.amm.historical_oracle_data.last_oracle_price_twap: {:?} -> {:?}",
            market.amm.historical_oracle_data.last_oracle_price_twap,
            market.amm.last_price_twap.cast::<i64>(&env)
        );

        log!(
            &env,
            "market.historical_oracle_data.last_oracle_price_twap_ts: {:?} -> {:?}",
            market.amm.historical_oracle_data.last_oracle_price_twap_ts,
            market.amm.last_price_twap_ts
        );

        market.amm.historical_oracle_data.last_oracle_price_twap =
            market.amm.last_price_twap.cast::<i64>(&env);

        market.amm.historical_oracle_data.last_oracle_price_twap_ts = market.amm.last_price_twap_ts;
    }

    fn update_oracle_twap(env: Env, keeper: Address) {
        keeper.require_auth();

        // allow update to amm's oracle twap iff price gap is reduced and thus more tame funding
        // otherwise if oracle error or funding flip: set oracle twap to mark twap (0 gap)

        let now = env.ledger().timestamp();

        let mut market = &mut get_market(&env);
        log!(&env, "updating amm oracle twap for market {}", market.name);
        // let price_oracle = &ctx.accounts.oracle;
        let oracle_twap = market.amm.get_oracle_twap(&market.oracle, now);

        if let Some(oracle_twap) = oracle_twap {
            let oracle_mark_gap_before = market.amm.last_price_twap.cast::<i64>(&env).safe_sub(
                market.amm.historical_oracle_data.last_oracle_price_twap,
                &env,
            );

            let oracle_mark_gap_after = market
                .amm
                .last_price_twap
                .cast::<i64>(&env)
                .safe_sub(oracle_twap, &env);

            if (oracle_mark_gap_after > 0 && oracle_mark_gap_before < 0)
                || (oracle_mark_gap_after < 0 && oracle_mark_gap_before > 0)
            {
                log!(
                    &env,
                    "market.amm.historical_oracle_data.last_oracle_price_twap {} -> {}",
                    market.amm.historical_oracle_data.last_oracle_price_twap,
                    market.amm.last_price_twap.cast::<i64>(&env)
                );
                log!(
                    &env,
                    "market.amm.historical_oracle_data.last_oracle_price_twap_ts {} -> {}",
                    market.amm.historical_oracle_data.last_oracle_price_twap_ts,
                    now
                );
                market.amm.historical_oracle_data.last_oracle_price_twap =
                    market.amm.last_price_twap.cast::<i64>(&env);
                market.amm.historical_oracle_data.last_oracle_price_twap_ts = now;
            } else if oracle_mark_gap_after.unsigned_abs() <= oracle_mark_gap_before.unsigned_abs()
            {
                log!(
                    &env,
                    "market.amm.historical_oracle_data.last_oracle_price_twap {} -> {}",
                    market.amm.historical_oracle_data.last_oracle_price_twap,
                    oracle_twap
                );
                log!(
                    &env,
                    "market.amm.historical_oracle_data.last_oracle_price_twap_ts {} -> {}",
                    market.amm.historical_oracle_data.last_oracle_price_twap_ts,
                    now
                );
                market.amm.historical_oracle_data.last_oracle_price_twap = oracle_twap;
                market.amm.historical_oracle_data.last_oracle_price_twap_ts = now;
            } else {
                panic_with_error!(env, Errors::PriceBandsBreached);
            }
        } else {
            panic_with_error!(env, Errors::InvalidOracle);
        }
    }

    // ################################################################
    //                          Super Keeper
    // ################################################################

    fn update_oracle(env: Env, sender: Address, oracle: Address, oracle_source: OracleSource) {
        sender.require_auth();
        utils::validate_super_keeper(&env, &sender);

        let mut market = &mut get_market(&env);
        log!(&env, "market {}", market.name);

        let now = env.ledger().timestamp();

        // Verify oracle is readable
        // let OraclePriceData {
        //     price: _oracle_price,
        //     delay: _oracle_delay,
        //     ..
        // } = get_oracle_price(&oracle_source, &ctx.accounts.oracle, now)?;

        log!(&env, "market.oracle: {:?} -> {:?}", market.amm.oracle, oracle);
        log!(
            &env,
            "market.oracle_source: {:?} -> {:?}",
            market.amm.oracle_source,
            oracle_source
        );

        market.amm.oracle = oracle;
        market.amm.oracle_source = oracle_source;
    }

    fn update_oracle_freeze(env: Env, sender: Address, frozen: bool) {
        sender.require_auth();
        utils::validate_super_keeper(&env, &sender);

        let mut market = &mut get_market(&env);
        log!(&env, "market {}", market.name);

        log!(
            &env,
            "market.oracle_frozen: {:?} -> {:?}",
            market.oracle_frozen,
            frozen
        );

        market.oracle_frozen = frozen;
        // TODO: do we need to freeze price?
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
        update: LiquidityPositionUpdate,
    ) {
        sender.require_auth();

        let mut position = get_liquidity_position_by_ts(&env, &sender, position_ts);

        position.update(&update);
    }

    fn close_position(env: Env, sender: Address, position_ts: u64) {
        sender.require_auth();

        let mut position = get_liquidity_position_by_ts(&env, &sender, position_ts);

        if !position.is_position_empty() {
            panic_with_error!(&env, ContractError::ClosePositionNotEmpty);
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
        tick_array_upper_index: i32,
    ) {
        sender.require_auth();

        if liquidity_amount == 0 {
            panic_with_error!(&env, ContractError::LiquidityZero);
        }

        let market = get_market(&env);
        let position = get_liquidity_position_by_ts(&env, &sender, position_ts);

        let tick_array_lower = match market.amm.tick_arrays.get(tick_array_lower_index) {
            Some(ta) => ta,
            None => {
                panic_with_error!(&env, Errors::AdminNotSet);
            }
        };
        let tick_array_upper = match market.amm.tick_arrays.get(tick_array_upper_index) {
            Some(ta) => ta,
            None => {
                panic_with_error!(&env, Errors::AdminNotSet);
            }
        };

        let liquidity_delta =
            math::liquidity_math::convert_to_liquidity_delta(&env, liquidity_amount, true);
        let timestamp = env.ledger().timestamp();

        let update = controller::liquidity::calculate_modify_liquidity(
            &env,
            &market.amm,
            &position,
            &tick_array_lower,
            &tick_array_upper,
            liquidity_delta,
            timestamp,
        );

        controller::liquidity::sync_modify_liquidity_values(
            &mut market.amm,
            &mut position,
            &mut tick_array_lower,
            &mut tick_array_upper,
            update,
            timestamp,
        );

        let (delta_a, delta_b) = controller::liquidity::calculate_liquidity_token_deltas(
            &env,
            market.amm.tick_current_index,
            market.amm.sqrt_price,
            &position,
            liquidity_delta,
        );

        if delta_a > token_max_a || delta_b > token_max_b {
            panic_with_error!(&env, ContractError::TokenMaxExceeded);
        }

        token_contract::Client::new(&env, &market.amm.token_a).transfer(
            &sender,
            &env.current_contract_address(),
            &delta_a,
        );

        token_contract::Client::new(&env, &market.amm.token_b).transfer(
            &sender,
            &env.current_contract_address(),
            &delta_b,
        );

        // utils::mint_shares(&env, &market.amm.lp_token, &env.current_contract_address(), 1);

        PoolEvents::increase_liquidity(
            &env,
            &sender,
            market.amm.token_a,
            market.amm.token_b,
            delta_a,
            delta_b,
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
        tick_array_upper_index: i32,
    ) {
        sender.require_auth();

        if liquidity_amount == 0 {
            panic_with_error!(&env, Errors::LiquidityZero);
        }
        let liquidity_delta =
            math::liquidity_math::convert_to_liquidity_delta(liquidity_amount, true);
        let timestamp = env.ledger().timestamp();

        let market = get_market(&env);
        let position = get_liquidity_position_by_ts(&env, &sender, position_ts);

        let tick_array_lower = match market.amm.tick_arrays.get(tick_array_lower_index) {
            Some(ta) => ta,
            None => {
                panic_with_error!(&env, Errors::AdminNotSet);
            }
        };
        let tick_array_upper = match market.amm.tick_arrays.get(tick_array_upper_index) {
            Some(ta) => ta,
            None => {
                panic_with_error!(&env, Errors::AdminNotSet);
            }
        };

        let update = controller::liquidity::calculate_modify_liquidity(
            &env,
            &market.amm,
            &position,
            &tick_array_lower,
            &tick_array_upper,
            liquidity_delta,
            timestamp,
        );

        controller::liquidity::sync_modify_liquidity_values(
            &mut market.amm,
            &mut position,
            &mut tick_array_lower,
            &mut tick_array_upper,
            update,
            timestamp,
        );

        let (delta_a, delta_b) = controller::liquidity::calculate_liquidity_token_deltas(
            &env,
            market.amm.tick_current_index,
            market.amm.sqrt_price,
            &position,
            liquidity_delta,
        );

        if delta_a < token_max_a || delta_b < token_max_b {
            panic_with_error!(env, Errors::TokenMinSubceeded);
        }

        token_contract::Client::new(&env, &market.amm.token_a).transfer(
            &env.current_contract_address(),
            &sender,
            &delta_a,
        );

        token_contract::Client::new(&env, &market.amm.token_b).transfer(
            &env.current_contract_address(),
            &sender,
            &delta_b,
        );

        PoolEvents::remove_liquidity(
            &env,
            &sender,
            market.amm.token_a,
            market.amm.token_b,
            delta_a,
            delta_b,
        );
    }

    fn swap(
        env: Env,
        sender: Address,
        amount: i128,
        other_amount_threshold: u64,
        sqrt_price_limit: u128,
        amount_specified_is_input: bool,
        a_to_b: bool, // Zero for one,
        tick_array_indexes: Vec<i32>,
    ) {
        sender.require_auth();
        utils::check_nonnegative_amount(amount);

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let timestamp = env.ledger().timestamp();
        let market = get_market(&env);

        controller::pool::update_pool_price(&env, &market.amm);

        let tick_arrays = tick_array_indexes
            .into_iter()
            .map(|index| {
                let tick_array = match market.amm.tick_arrays.get(index) {
                    Some(ta) => ta,
                    None => {
                        panic_with_error!(&env, Errors::AdminNotSet);
                    }
                };
                tick_array
            })
            .collect();

        let builder =
            SparseSwapTickSequenceBuilder::try_from(&env, &market.amm, a_to_b, tick_arrays, None);
        let mut swap_tick_sequence = builder.build(&env);

        let swap_update = controller::swap::swap(
            &env,
            &market.amm,
            &mut swap_tick_sequence,
            amount,
            sqrt_price_limit,
            amount_specified_is_input,
            a_to_b,
            timestamp,
        );

        if amount_specified_is_input {
            if (a_to_b && other_amount_threshold > swap_update.amount_b)
                || (!a_to_b && other_amount_threshold > swap_update.amount_a)
            {
                panic_with_error!(&env, Errors::AmountOutBelowMinimum);
            }
        } else if (a_to_b && other_amount_threshold < swap_update.amount_a)
            || (!a_to_b && other_amount_threshold < swap_update.amount_b)
        {
            panic_with_error!(&env, Errors::AmountInAboveMaximum);
        }

        update_and_swap_amm(&env, &market.amm, sender, swap_update, a_to_b, timestamp);

        PoolEvents::swap(
            &env,
            sender,
            market.amm.token_a,
            market.amm.token_b,
            amount,
            0,
            0,
        );
    }

    fn collect_fees(env: Env, sender: Address, position_ts: u64) {
        sender.require_auth();

        let market = get_market(&env);
        let mut position = get_liquidity_position_by_ts(&env, &sender, position_ts);

        // Store the fees owed to use as transfer amounts.
        let fee_owed_a = position.fee_owed_a;
        let fee_owed_b = position.fee_owed_b;

        position.reset_fees_owed();

        token_contract::Client::new(&env, &market.amm.token_a).transfer(
            &env.current_contract_address(),
            &sender,
            &fee_owed_a,
        );

        token_contract::Client::new(&env, &market.amm.token_b).transfer(
            &env.current_contract_address(),
            &sender,
            &fee_owed_b,
        );

        PoolEvents::collect_fees(&env, sender, fee_owed_a, fee_owed_b);
    }

    fn collect_reward(env: Env, sender: Address, reward_token: Address, position_ts: u64) {
        sender.require_auth();

        let market = get_market(&env);
        let position = &mut get_liquidity_position_by_ts(&env, &sender, position_ts);

        let (reward, reward_index) = market.amm.get_reward_by_token(reward_token);
        let (transfer_amount, updated_amount_owed) =
            calculate_collect_reward(position.reward_infos[index], reward.current_balance);

        position.update_reward_owed(index, updated_amount_owed);

        token_contract::Client::new(&env, &reward_token).transfer(
            &env.current_contract_address(),
            &sender,
            &transfer_amount,
        );

        // TODO: decrement the market's available reward balance
        reward.current_balance -= transfer_amount;
    }

    // ################################################################
    //                             Queries
    // ################################################################
}

#[contractimpl]
impl SynthMarket {
    #[allow(dead_code)]
    pub fn update(env: Env, new_wasm_hash: BytesN<32>) {
        let admin = get_admin(&env);
        admin.require_auth();

        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
}

// TODO: do we need to update the AMM?
pub fn update_amm_and_check_validity(
    market: &mut Market,
    oracle_price_data: &OraclePriceData,
    now: u64,
    action: Option<NormalAction>,
) {
    // _update_amm(market, oracle_price_data, state, now, clock_slot)?;

    // 1 hour EMA
    // let risk_ema_price = market.amm.historical_oracle_data.last_oracle_price_twap;

    // let oracle_validity = oracle_validity(
    //     market.name,
    //     risk_ema_price,
    //     oracle_price_data,
    //     oracle_guard_rails().validity, // import from Oracle module
    //     market.get_max_confidence_interval_multiplier()?,
    //     false
    // );

    // validate!(
    //     is_oracle_valid_for_action(oracle_validity, action)?,
    //     Errors::InvalidOracle,
    //     "Invalid Oracle ({:?} vs ema={:?}) for perp market index={} and action={:?}",
    //     oracle_price_data,
    //     risk_ema_price,
    //     market.name,
    //     action
    // );
}
