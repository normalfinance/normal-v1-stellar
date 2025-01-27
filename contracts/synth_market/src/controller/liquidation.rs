// #[cfg(test)]
// mod tests;

use normal::{
    error::{ErrorCode, NormalResult},
    validate,
};
use soroban_sdk::{Address, Env};

use crate::storage::Position;

pub fn liquidate_position(
    env: &Env,
    // market_index: u16,
    liquidator_max_base_asset_amount: u64,
    limit_price: Option<u64>,
    user: &Address,
    liquidator: &Address,
    now: i64,
) -> NormalResult {
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

    let margin_calculation = calculate_margin_requirement_and_total_collateral_and_liability_info(
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

    Ok(())
}

pub fn resolve_position_bankruptcy(
    // market_index: u16,
    user: &Address,
    liquidator: &Address,
    now: i64,
    insurance_fund_vault_balance: u64,
) -> NormalResult<u64> {
    if !user.is_bankrupt() && is_user_bankrupt(user) {
        user.enter_bankruptcy();
    }

    validate!(
        user.is_bankrupt(),
        ErrorCode::UserNotBankrupt,
        "user not bankrupt"
    )?;

    validate!(
        !liquidator.is_being_liquidated(),
        ErrorCode::UserIsBeingLiquidated,
        "liquidator being liquidated"
    )?;

    validate!(
        !liquidator.is_bankrupt(),
        ErrorCode::UserBankrupt,
        "liquidator bankrupt"
    )?;

    let market = perp_market_map.get_ref(&market_index)?;

    validate!(
        !market.is_operation_paused(PerpOperation::Liquidation),
        ErrorCode::InvalidLiquidation,
        "Liquidation operation is paused for market {}",
        market_index
    )?;

    drop(market);

    user.get_perp_position(market_index).map_err(|e| {
        msg!(
            "User does not have a position for perp market {}",
            market_index
        );
        e
    })?;

    let loss = user
        .get_perp_position(market_index)?
        .quote_asset_amount
        .cast::<i128>()?;

    validate!(
        loss < 0,
        ErrorCode::InvalidPerpPositionToLiquidate,
        "user must have negative pnl"
    )?;

    let MarginCalculation {
        margin_requirement,
        total_collateral,
        ..
    } = calculate_margin_requirement_and_total_collateral_and_liability_info(
        user,
        perp_market_map,
        spot_market_map,
        oracle_map,
        MarginContext::standard(MarginRequirementType::Maintenance),
    )?;

    // spot market's insurance fund draw attempt here (before social loss)
    // subtract 1 from available insurance_fund_vault_balance so deposits in insurance vault always remains >= 1

    let if_payment = {
        let mut perp_market = perp_market_map.get_ref_mut(&market_index)?;
        let max_insurance_withdraw = perp_market
            .insurance_claim
            .quote_max_insurance
            .safe_sub(perp_market.insurance_claim.quote_settled_insurance)?
            .cast::<u128>()?;

        let if_payment = loss
            .unsigned_abs()
            .min(insurance_fund_vault_balance.saturating_sub(1).cast()?)
            .min(max_insurance_withdraw);

        perp_market.insurance_claim.quote_settled_insurance = perp_market
            .insurance_claim
            .quote_settled_insurance
            .safe_add(if_payment.cast()?)?;

        // move if payment to pnl pool
        let spot_market = &mut spot_market_map.get_ref_mut(&QUOTE_SPOT_MARKET_INDEX)?;
        let oracle_price_data = oracle_map.get_price_data(&spot_market.oracle)?;
        update_spot_market_cumulative_interest(spot_market, Some(oracle_price_data), now)?;

        update_spot_balances(
            if_payment,
            &SpotBalanceType::Deposit,
            spot_market,
            &mut perp_market.pnl_pool,
            false,
        )?;

        if_payment
    };

    let losses_remaining: i128 = loss.safe_add(if_payment.cast::<i128>()?)?;
    validate!(
        losses_remaining <= 0,
        ErrorCode::InvalidPerpPositionToLiquidate,
        "losses_remaining must be non-positive"
    )?;

    let fee_pool_payment: i128 = if losses_remaining < 0 {
        let perp_market = &mut perp_market_map.get_ref_mut(&market_index)?;
        let spot_market = &mut spot_market_map.get_ref_mut(&QUOTE_SPOT_MARKET_INDEX)?;
        let fee_pool_tokens = get_fee_pool_tokens(perp_market, spot_market)?;
        msg!("fee_pool_tokens={:?}", fee_pool_tokens);

        losses_remaining.abs().min(fee_pool_tokens.cast()?)
    } else {
        0
    };
    validate!(
        fee_pool_payment >= 0,
        ErrorCode::InvalidPerpPositionToLiquidate,
        "fee_pool_payment must be non-negative"
    )?;

    if fee_pool_payment > 0 {
        let perp_market = &mut perp_market_map.get_ref_mut(&market_index)?;
        let spot_market = &mut spot_market_map.get_ref_mut(&QUOTE_SPOT_MARKET_INDEX)?;
        msg!("fee_pool_payment={:?}", fee_pool_payment);
        update_spot_balances(
            fee_pool_payment.unsigned_abs(),
            &SpotBalanceType::Borrow,
            spot_market,
            &mut perp_market.amm.fee_pool,
            false,
        )?;
    }

    let loss_to_socialize = losses_remaining.safe_add(fee_pool_payment.cast::<i128>()?)?;
    validate!(
        loss_to_socialize <= 0,
        ErrorCode::InvalidPerpPositionToLiquidate,
        "loss_to_socialize must be non-positive"
    )?;

    let cumulative_funding_rate_delta = calculate_funding_rate_deltas_to_resolve_bankruptcy(
        loss_to_socialize,
        perp_market_map.get_ref(&market_index)?.deref(),
    )?;

    // socialize loss
    if loss_to_socialize < 0 {
        let mut market = perp_market_map.get_ref_mut(&market_index)?;

        market.amm.total_social_loss = market
            .amm
            .total_social_loss
            .safe_add(loss_to_socialize.unsigned_abs())?;

        market.amm.cumulative_funding_rate_long = market
            .amm
            .cumulative_funding_rate_long
            .safe_add(cumulative_funding_rate_delta)?;

        market.amm.cumulative_funding_rate_short = market
            .amm
            .cumulative_funding_rate_short
            .safe_sub(cumulative_funding_rate_delta)?;
    }

    // clear bad debt
    {
        let mut market = perp_market_map.get_ref_mut(&market_index)?;
        let position_index = get_position_index(&user.perp_positions, market_index)?;
        let quote_asset_amount = user.perp_positions[position_index].quote_asset_amount;
        update_quote_asset_amount(
            &mut user.perp_positions[position_index],
            &mut market,
            -quote_asset_amount,
        )?;

        user.increment_total_socialized_loss(quote_asset_amount.unsigned_abs())?;
    }

    // exit bankruptcy
    if !is_user_bankrupt(user) {
        user.exit_bankruptcy();
    }

    let liquidation_id = user.next_liquidation_id.safe_sub(1)?;

    emit!(LiquidationRecord {
        ts: now,
        liquidation_id,
        liquidation_type: LiquidationType::PerpBankruptcy,
        user: *user_key,
        liquidator: *liquidator_key,
        margin_requirement,
        total_collateral,
        bankrupt: true,
        perp_bankruptcy: PerpBankruptcyRecord {
            market_index,
            if_payment,
            pnl: loss,
            clawback_user: None,
            clawback_user_payment: None,
            cumulative_funding_rate_delta,
        },
        ..LiquidationRecord::default()
    });

    if_payment.cast()
}

pub fn calculate_margin_freed(
    position: &Position,
    liquidation_margin_buffer_ratio: u32,
    initial_margin_shortage: u128,
) -> NormalResult<(u64, MarginCalculation)> {
    let margin_calculation_after =
        calculate_margin_requirement_and_total_collateral_and_liability_info(
            position,
            MarginContext::liquidation(liquidation_margin_buffer_ratio),
        )?;

    let new_margin_shortage = margin_calculation_after.margin_shortage()?;

    let margin_freed = initial_margin_shortage
        .saturating_sub(new_margin_shortage)
        .cast::<u64>()?;

    Ok((margin_freed, margin_calculation_after))
}

pub fn set_position_status_to_being_liquidated(position: &mut Position, now: u64) -> NormalResult {
    validate!(
        !position.is_bankrupt(),
        ErrorCode::UserBankrupt,
        "position bankrupt"
    )?;

    validate!(
        !position.is_being_liquidated(),
        ErrorCode::UserIsBeingLiquidated,
        "position is already being liquidated"
    )?;

    let liquidation_margin_buffer_ratio = state.liquidation_margin_buffer_ratio;
    let margin_calculation = calculate_margin_requirement_and_total_collateral_and_liability_info(
        position,
        perp_market_map,
        spot_market_map,
        oracle_map,
        MarginContext::liquidation(liquidation_margin_buffer_ratio),
    )?;

    if !position.is_being_liquidated() && margin_calculation.meets_margin_requirement() {
        msg!("margin calculation: {:?}", margin_calculation);
        return Err(ErrorCode::SufficientCollateral);
    } else {
        position.enter_liquidation(now)?;
    }
    Ok(())
}
