use soroban_sdk::{assert_with_error, contract, contractimpl, Address, Env, Symbol};

use crate::{
    constants::{LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO, MAX_MARGIN_RATIO, MIN_MARGIN_RATIO},
    errors,
    storage::{get_admin, get_market, get_position, save_market, save_position, DataKey},
    synth_market::SynthMarketTrait,
    token_contract,
};

use normal::oracle::{is_oracle_too_divergent_with_twap_5min, oracle_validity};
use normal::utils::validate;

contractmeta!(
    key = "Description",
    val = "Synthetic asset tracking the value of another cryptocurrency"
);

#[contract]
pub struct SynthMarket;

#[contractimpl]
impl SynthMarketTrait for SynthMarket {
    // ################################################################
    //                             ADMIN
    // ################################################################

    fn initialize(env: Env, sender: Address, params: SynthMarketParams) {
        is_admin(&env, sender);

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

        validate_margin(
            params.margin_ratio_initial,
            params.margin_ratio_maintenance,
            params.liquidator_fee,
        )?;

        let market = SynthMarket::new(params);

        save_market(&env, market);
    }

    fn update_paused_operations(env: Env, admin: Address, paused_operations: Vec<Operation>) {
        is_admin(&env, sender);

        let mut market = get_market(&env);

        e.storage()
            .instance()
            .set(&DataKey::PausedOperations, &paused_operations);

        save_market(&env, market);

        log_all_operations_paused(
            e.storage()
                .instance()
                .get(&DataKey::PausedOperations)
                .unwrap(),
        )
    }

    fn update_amm(env: Env, admin: Address, amm: Address) {
        is_admin(&env, sender);

        let mut market = get_market(&env);

        // Verify oracle is readable
        // let OraclePriceData {
        //     price: _oracle_price,
        //     delay: _oracle_delay,
        //     ..
        // } = get_oracle_price(&oracle_source, &ctx.accounts.oracle, clock.slot)?;

        log!(env, "market.amm: {} -> {}", market.amm, amm);

        market.amm = amm;

        save_market(&env, market);
    }

    fn update_debt_limit(
        env: Env,
        admin: Address,
        debt_floor: Option<u32>,
        debt_ceiling: Option<u128>,
    ) {
        is_admin(&env, sender);

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

    fn extend_expiry_ts(env: Env, admin: Address, expiry_ts: i64) {
        is_admin(&env, sender);

        let mut market = get_market(&env);

        log!(env, "updating market {} expiry", market.name);

        // TODO: validate already in reduceonly mode / shutdown
        let current_ts = env.ledger().timestamp();
        validate!(
            current_ts < expiry_ts,
            ErrorCode::DefaultError,
            "Market expiry ts must later than current clock ts"
        )?;

        validate!(
            current_ts < expiry_ts,
            ErrorCode::DefaultError,
            "Market expiry ts must later than current clock ts"
        )?;

        msg!("market.expiry_ts {} -> {}", market.expiry_ts, expiry_ts);

        market.expiry_ts = expiry_ts;

        save_market(&env, market)
    }

    fn update_margin_config(
        env: Env,
        admin: Address,
        imf_factor: Option<u32>,
        margin_ratio_initial: Option<u32>,
        margin_ratio_maintenance: Option<u32>,
    ) {
        is_admin(&env, sender);

        let mut market = get_market(&env);

        log!(env, "updating market {} margin ratio", market.name);

        validate_margin(
            margin_ratio_initial,
            margin_ratio_maintenance,
            market.liquidator_fee,
        )?;

        log!(
            env,
            "market.margin_ratio_initial: {} -> {}",
            market.margin_ratio_initial,
            margin_ratio_initial
        );

        log!(
            env,
            "market.margin_ratio_maintenance: {} -> {}",
            market.margin_ratio_maintenance,
            margin_ratio_maintenance
        );

        market.margin_ratio_initial = margin_ratio_initial;
        market.margin_ratio_maintenance = margin_ratio_maintenance;

        if let Some(imf_factor) = imf_factor {
            validate!(
                imf_factor <= SPOT_IMF_PRECISION,
                ErrorCode::DefaultError,
                "invalid imf factor"
            )?;

            msg!("market.imf_factor: {} -> {}", market.imf_factor, imf_factor);

            market.imf_factor = imf_factor;
        }

        save_market(&env, market)
    }

    fn update_liquidation_config(
        env: Env,
        admin: Address,
        liquidation_fee: u32,
        if_liquidation_fee: u32,
        liquidation_penalty: Option<u32>,
    ) {
        is_admin(&env, sender);

        let mut market = get_market(&env);

        log!(env, "updating market {} liquidation fee", market.name);

        validate!(
            liquidator_fee.safe_add(if_liquidation_fee)? < LIQUIDATION_FEE_PRECISION,
            ErrorCode::DefaultError,
            "Total liquidation fee must be less than 100%"
        )?;

        validate!(
            if_liquidation_fee < LIQUIDATION_FEE_PRECISION,
            ErrorCode::DefaultError,
            "If liquidation fee must be less than 100%"
        )?;

        validate_margin(
            market.margin_ratio_initial,
            market.margin_ratio_maintenance,
            liquidator_fee,
        )?;

        log!(
            env,
            "market.liquidator_fee: {} -> {}",
            market.liquidator_fee,
            liquidator_fee
        );

        log!(
            env,
            "market.if_liquidation_fee: {} -> {}",
            market.if_liquidation_fee,
            if_liquidation_fee
        );

        market.liquidator_fee = liquidator_fee;
        market.if_liquidation_fee = if_liquidation_fee;

        if let Some(liquidation_penalty) = liquidation_penalty {
            log!(env, "updating market {} liquidation penalty", market.name);

            // TODO: do we need validation?

            log!(
                env,
                "market.liquidation_penalty: {} -> {}",
                market.liquidation_penalty,
                liquidation_penalty
            );

            market.liquidation_penalty = liquidation_penalty;
        }

        save_market(&env, market)
    }

    fn update_name(env: Env, admin: Address, name: String) {
        is_admin(&env, sender);

        let mut market = get_market(&env);

        log!(env, "market.name: {} -> {}", market.name, name);
        market.name = name;

        save_market(&env, market)
    }

    fn update_status(env: Env, admin: Address, status: MarketStatus) {
        is_admin(&env, sender);

        validate!(
            !matches!(status, MarketStatus::Delisted | MarketStatus::Settlement),
            ErrorCode::DefaultError,
            "must set settlement/delist through another instruction"
        )?;

        let mut market = get_market(&env);

        log!(env, "market {}", market.name);
        log!(env, "market.status: {} -> {}", market.status, status);
        market.status = status;

        save_market(&env, market)
    }

    fn update_synth_tier(env: Env, admin: Address, synth_tier: SynthTier) {
        is_admin(&env, sender);

        let mut market = get_market(&env);

        log!(env, "market {}", market.name);
        log!(
            env,
            "market.synth_tier: {} -> {}",
            market.synth_tier,
            synth_tier
        );
        market.synth_tier = synth_tier;

        save_market(&env, market)
    }

    // ################################################################

    fn freeze_oracle(env: Env, sender: Address) {
        sender.require_auth();

        is_emergency_oracle(&env, sender);
    }

    fn initialize_shutdown(env: Env, keeper: Address, expiry_ts: i64) {
        is_admin(&env, sender);

        let mut market = get_market(&env);
        log!(env, "updating market {} expiry", market.name);

        // Pause vault Create, Deposit, Lend, and Delete
        market.paused_operations = EMERGENCY_SHUTDOWN_PAUSED_OPERATIONS;

        Operation::log_all_operations_paused(market.paused_operations);

        // TODO: freeze collateral prices

        // vault owners can withraw any excess collateral if their debt obligations are met

        validate!(
            env.ledger().timestamp < expiry_ts,
            ErrorCode::DefaultError,
            "Market expiry ts must later than current clock timestamp"
        )?;

        log!(
            env,
            "market.status {} -> {}",
            market.status,
            MarketStatus::ReduceOnly
        );
        log!(
            env,
            "market.expiry_ts {} -> {}",
            market.expiry_ts,
            expiry_ts
        );

        // automatically enter reduce only
        market.status = MarketStatus::ReduceOnly;
        market.expiry_ts = expiry_ts;

        save_market(&env, market)
    }

    // ################################################################
    //                             KEEPER
    // ################################################################

    fn liquidate_position(
        e: Env,
        liquidator: Address,
        user: Address,
        liquidator_max_base_asset_amount: u64,
        limit_price: Option<u64>,
    ) {
        liquidator.require_auth();

        if user == liquidator {
            return Err(ErrorCode::UserCantLiquidateThemself);
        }

        // TODO: do we define these per market or at the factory level?
        let liquidation_margin_buffer_ratio = state.liquidation_margin_buffer_ratio;
        let initial_pct_to_liquidate = state.initial_pct_to_liquidate as u128;
        let liquidation_duration = state.liquidation_duration as u128;

        let mut position = get_position(&env, &user);

        validate!(
            !position.is_bankrupt(),
            ErrorCode::UserBankrupt,
            "user bankrupt"
        )?;

        validate!(
            !market.is_operation_paused(SynthOperation::Liquidation),
            ErrorCode::InvalidLiquidation,
            "Liquidation operation is paused for market {}",
            market_index
        )?;

        let margin_calculation =
            calculate_margin_requirement_and_total_collateral_and_liability_info(
                user,
                MarginContext::liquidation(liquidation_margin_buffer_ratio)
                    .track_market_margin_requirement(MarketIdentifier::perp(market_index))?,
            )?;

        if !position.is_being_liquidated() && margin_calculation.meets_margin_requirement() {
            msg!("margin calculation: {:?}", margin_calculation);
            return Err(ErrorCode::SufficientCollateral);
        } else if position.is_being_liquidated() && margin_calculation.can_exit_liquidation()? {
            position.exit_liquidation();
            return Ok(());
        }

        let liquidation_id = position.enter_liquidation()?;
        let mut margin_freed = 0_u64;

        validate!(
           position.is_open_position()
            ErrorCode::PositionDoesntHaveOpenPositionOrOrders
        )?;

        // ...

        let oracle_price_data = get_oralce_price_data(&market.amm.oracle)?;

        update_amm_and_check_validity(
            &mut market,
            oracle_price_data,
            state,
            now,
            Some(DriftAction::Liquidate),
        )?;

        let oracle_price = if market.status == MarketStatus::Settlement {
            market.expiry_price
        } else {
            oracle_price_data.price
        };

        let oracle_price_too_divergent = is_oracle_too_divergent_with_twap_5min(
            oracle_price,
            perp_market_map
                .get_ref(&market_index)?
                .amm
                .historical_oracle_data
                .last_oracle_price_twap_5min,
            state
                .oracle_guard_rails
                .max_oracle_twap_5min_percent_divergence()
                .cast()?,
        )?;

        validate!(!oracle_price_too_divergent, ErrorCode::PriceBandsBreached)?;

        let user_base_asset_amount = position.cumulative_deposits.unsigned_abs();

        let margin_ratio = SynthMarket::get_margin_ratio(
            user_base_asset_amount.cast()?,
            MarginRequirementType::Maintenance,
        )?;

        let margin_ratio_with_buffer = margin_ratio.safe_add(liquidation_margin_buffer_ratio)?;

        let margin_shortage = margin_calculation.margin_shortage()?;

        // ...

        let quote_oracle_price = get_oracle_price_data(&market.quote_oracle)?.price;
        let liquidator_fee = market.liquidator_fee;
        let if_liquidation_fee = calculate_if_fee(
            margin_calculation.tracked_market_margin_shortage(margin_shortage)?,
            user_base_asset_amount,
            margin_ratio_with_buffer,
            liquidator_fee,
            oracle_price,
            quote_oracle_price,
            market.if_liquidation_fee,
        )?;
        let base_asset_amount_to_cover_margin_shortage = standardize_base_asset_amount_ceil(
            calculate_base_asset_amount_to_cover_margin_shortage(
                margin_shortage,
                margin_ratio_with_buffer,
                liquidator_fee,
                if_liquidation_fee,
                oracle_price,
                quote_oracle_price,
            )?,
            market.amm.order_step_size, // TODO: is this the tick spacing?
        )?;

        // ...
    }

    fn resolve_position_bankruptcy(e: Env, sender: Address) {
        sender.require_auth();
    }

    // ################################################################
    //                             USER
    // ################################################################

    fn deposit_collateral(env: Env, user: Address, amount: i128) {
        user.require_auth();

        if amount <= 0 {
            return Err(ErrorCode::InsufficientDeposit);
        }

        let mut position = get_position(&env, &user);

        validate!(!position.is_bankrupt(), ErrorCode::UserBankrupt)?;

        let mut market = get_market(&env);
        // let oracle_price_data = &oracle_map.get_price_data(&synth_market.oracle)?.clone();

        validate!(
            !matches!(market.status, MarketStatus::Initialized),
            ErrorCode::MarketBeingInitialized,
            "Market is being initialized"
        )?;

        let force_reduce_only = market.is_reduce_only();

        utils::update_market_cumulative_interest();

        // ...

        // Deposit the token amount from the user into the market
        let collateral_token_client = token_contract::Client::new(&env, &market.collateral_token);
        collateral_token_client.transfer(&user, &env.current_contract_address(), &amount);

        // TODO: update the user's position
        // ...
        save_position(&env, &user, &position);

        // TODO: update the market's cumulative properties
        utils::update_position_and_market_cumulative_deposits(&env, &position);

        // ...

        SynthMarketEvents::collateral_deposit(&env);
    }

    fn transfer_collateral(env: Env, from_user: Address, to_user: Address, amount: u64) {
        from_user.require_auth();

        validate!(
            !to_user.is_bankrupt(),
            ErrorCode::UserBankrupt,
            "to_user bankrupt"
        )?;
        validate!(
            !from_user.is_bankrupt(),
            ErrorCode::UserBankrupt,
            "from_user bankrupt"
        )?;

        validate!(
            from_user_key != to_user_key,
            ErrorCode::CantTransferBetweenSameUserAccount,
            "cant transfer between the same user account"
        )?;

        // let oracle_price_data = oracle_map.get_price_data(&synth_market.oracle)?;
        // controller::synth_balance::update_synth_market_cumulative_interest(
        // 	synth_market,
        // 	Some(oracle_price_data),
        // 	clock.unix_timestamp
        // )?;

        // ...

        to_user.increment_total_deposits(
            amount,
            oracle_price,
            spot_market.get_precision().cast()?,
        )?;

        let total_deposits_after = to_user.total_deposits;
        let total_withdraws_after = to_user.total_withdraws;

        // ...

        SynthMarketEvents::collateral_transfer();
    }

    fn withdraw_collateral(env: Env, user: Address, amount: i128, reduce_only: bool) {
        user.require_auth();

        if amount <= 0 {
            return Err(ErrorCode::InsufficientDeposit);
        }

        let mut position = get_position(&env, &user);

        validate!(!position.is_bankrupt(), ErrorCode::UserBankrupt)?;

        let mut market = get_market(&env);
        // ...

        SynthMarketEvents::collateral_withdrawal(&env);
    }

    fn borrow_synthetic_and_provide_liquidity(env: Env, user: Address, amount: i128) {
        user.require_auth();

        let mut market = get_market(&env);

        // Compute amount to mint

        let mint_amount = 0;

        if mint_amount >= max_amount_user_can_mint {
            return Err(ErrorCode::InsufficientFunds);
        }

        // Mint tokens
        let synth_token_client = token_contract::Client::new(&env, &market.token);
        synth_token_client.mint(&env.current_contract_address(), &mint_amount);

        // Update market numbers
        market.debt_balance += 0;
        market.synthetic_tokens_minted += mint_amount;
        market.outstanding_debt += 0;

        // Fetch the protocol LP
        let liquidity_position = get_lp(&env, market.liquidity_position_ts);

        // Update LP tick indexes if out of range
        if liquidity_position.is_out_of_bounds() {
            let (new_tick_lower_index, new_tick_upper_index) = utils::find_new_tick_indexes();

            let modify_lp_response: ModifyLiquidityPositionResponse = env.invoke_contract(
                &config.amm_contract_address,
                &Symbol::new(&env, "modify_position"),
                vec![
                    &env,
                    sender.into_val(&env),
                    position_timestamp: market.liquidity_position_ts,
                    update: PositionUpdate {
                        tick_lower_index: new_tick_lower_index,
                        tick_upper_index: new_tick_upper_index,
                    }
                ],
            );
        }

        // Add new liquidity to the AMM
        let increase_liquidity_response: IncreaseLiquidityResponse = env.invoke_contract(
            &config.amm_contract_address,
            &Symbol::new(&env, "increase_liquidity"),
            vec![
                &env,
                sender.into_val(&env),
                position_timestamp: market.liquidity_position_ts,
                liquidity_amount: amount,
                token_max_a: 0,
                token_max_b: 0
            ],
        );

        // Update market liquidity provisioning properties
        // market.debt_balance += 0;

        SynthMarketEvents::mint_synthetic();
        SynthMarketEvents::provide_liquidity();
    }
}

pub fn validate_margin(
    margin_ratio_initial: u32,
    margin_ratio_maintenance: u32,
    liquidation_fee: u32,
) {
    if !(MIN_MARGIN_RATIO..=MAX_MARGIN_RATIO).contains(&margin_ratio_initial) {
        return Err(ErrorCode::InvalidMarginRatio);
    }

    if margin_ratio_initial <= margin_ratio_maintenance {
        return Err(ErrorCode::InvalidMarginRatio);
    }

    if !(MIN_MARGIN_RATIO..=MAX_MARGIN_RATIO).contains(&margin_ratio_maintenance) {
        return Err(ErrorCode::InvalidMarginRatio);
    }

    validate!(
        margin_ratio_maintenance * LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO > liquidation_fee,
        ErrorCode::InvalidMarginRatio,
        "margin_ratio_maintenance must be greater than liquidation fee"
    )?;
}

// TODO: do we need to update the AMM?
pub fn update_amm_and_check_validity(
    market: &mut PerpMarket,
    oracle_price_data: &OraclePriceData,
    state: &State,
    now: i64,
    action: Option<DriftAction>,
) -> DriftResult {
    // _update_amm(market, oracle_price_data, state, now, clock_slot)?;

    // 1 hour EMA
    let risk_ema_price = market.amm.historical_oracle_data.last_oracle_price_twap;

    let oracle_validity = oracle_validity(
        market.name,
        risk_ema_price,
        oracle_price_data,
        oracle_guard_rails().validity, // import from Oracle module
        market.get_max_confidence_interval_multiplier()?,
        false,
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
