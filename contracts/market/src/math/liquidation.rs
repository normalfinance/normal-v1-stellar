use crate::errors::Errors;
use crate::state::margin_calculation::MarginContext;
use crate::state::market::Market;
use crate::state::market_position::MarketPosition;
use normal::constants::{
    BASE_PRECISION, LIQUIDATION_FEE_INCREASE_PER_SLOT, LIQUIDATION_FEE_PRECISION,
    LIQUIDATION_FEE_PRECISION_U128, LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO,
    LIQUIDATION_PCT_PRECISION, PRICE_PRECISION, PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO,
    QUOTE_PRECISION, SPOT_WEIGHT_PRECISION_U128,
};
use normal::math::casting::Cast;
use normal::math::safe_math::SafeMath;
use normal::validate;
use soroban_sdk::{contracttype, panic_with_error, Env};

use super::balance::{get_token_amount, BalanceType};
use super::margin::calculate_margin_requirement_and_total_collateral_and_liability_info;

pub const LIQUIDATION_FEE_ADJUST_GRACE_PERIOD_SLOTS: u64 = 1_500; // ~10 minutes

pub fn calculate_base_asset_amount_to_cover_margin_shortage(
    env: &Env,
    margin_shortage: u128,
    margin_ratio: u32,
    liquidation_fee: u32,
    if_liquidation_fee: u32,
    oracle_price: i64,
    quote_oracle_price: i64,
) -> u64 {
    let margin_ratio = margin_ratio.safe_mul(LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO, env);

    if oracle_price == 0 || margin_ratio <= liquidation_fee {
        return u64::MAX;
    }

    margin_shortage
        .safe_mul(PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO, env)
        .safe_div(
            oracle_price
                .cast::<u128>(env)
                .safe_mul(quote_oracle_price.cast(env), env)
                .safe_div(PRICE_PRECISION, env)
                .safe_mul(margin_ratio.safe_sub(liquidation_fee, env).cast(env), env)
                .safe_div(LIQUIDATION_FEE_PRECISION_U128, env)
                .safe_sub(
                    oracle_price
                        .cast::<u128>(env)
                        .safe_mul(if_liquidation_fee.cast(env), env)
                        .safe_div(LIQUIDATION_FEE_PRECISION_U128, env),
                    env,
                ),
            env,
        )
        .cast(env)
}

pub fn calculate_liability_transfer_to_cover_margin_shortage(
    env: &Env,
    margin_shortage: u128,
    asset_weight: u32,
    asset_liquidation_multiplier: u32,
    liability_weight: u32,
    liability_liquidation_multiplier: u32,
    liability_decimals: u32,
    liability_price: i64,
    if_liquidation_fee: u32,
) -> u128 {
    // If unsettled pnl asset weight is 1 and quote asset is 1, this calculation breaks
    if asset_weight >= liability_weight {
        return u128::MAX;
    }

    let (numerator_scale, denominator_scale) = if liability_decimals > 6 {
        ((10_u128).pow(liability_decimals - 6), 1)
    } else {
        (1, (10_u128).pow(6 - liability_decimals))
    };

    let liability_weight_component = liability_weight.cast::<u128>(env).safe_mul(10, env); // multiply market weights by extra 10 to increase precision

    let asset_weight_component = asset_weight
        .cast::<u128>(env)
        .safe_mul(10, env)
        .safe_mul(asset_liquidation_multiplier.cast(env), env)
        .safe_div(liability_liquidation_multiplier.cast(env), env);

    if asset_weight_component >= liability_weight_component {
        return u128::MAX;
    }

    margin_shortage
        .safe_mul(numerator_scale, env)
        .safe_mul(PRICE_PRECISION * SPOT_WEIGHT_PRECISION_U128 * 10, env)
        .safe_div(
            liability_price
                .cast::<u128>(env)
                .safe_mul(
                    liability_weight_component.safe_sub(asset_weight_component, env),
                    env,
                )
                .safe_sub(
                    liability_price
                        .cast::<u128>(env)
                        .safe_mul(if_liquidation_fee.cast(env), env)
                        .safe_div(LIQUIDATION_FEE_PRECISION_U128, env)
                        .safe_mul(liability_weight.cast(env), env)
                        .safe_mul(10, env),
                    env,
                ),
            env,
        )
        .safe_div(denominator_scale, env)
}

pub fn calculate_liability_transfer_implied_by_asset_amount(
    env: &Env,
    asset_amount: u128,
    asset_liquidation_multiplier: u32,
    asset_decimals: u32,
    asset_price: i64,
    liability_liquidation_multiplier: u32,
    liability_decimals: u32,
    liability_price: i64,
) -> u128 {
    let (numerator_scale, denominator_scale) = if liability_decimals > asset_decimals {
        ((10_u128).pow(liability_decimals - asset_decimals), 1)
    } else {
        (1, (10_u128).pow(asset_decimals - liability_decimals))
    };

    asset_amount
        .safe_mul(numerator_scale, env)
        .safe_mul(asset_price.cast(env), env)
        .safe_mul(liability_liquidation_multiplier.cast(env), env)
        .safe_div_ceil(
            liability_price
                .cast::<u128>(env)
                .safe_mul(asset_liquidation_multiplier.cast(env), env),
            env,
        )
        .safe_div_ceil(denominator_scale, env)
}

pub fn calculate_asset_transfer_for_liability_transfer(
    env: &Env,
    asset_amount: u128,
    asset_liquidation_multiplier: u32,
    asset_decimals: u32,
    asset_price: i64,
    liability_amount: u128,
    liability_liquidation_multiplier: u32,
    liability_decimals: u32,
    liability_price: i64,
) -> u128 {
    let (numerator_scale, denominator_scale) = if asset_decimals > liability_decimals {
        ((10_u128).pow(asset_decimals - liability_decimals), 1)
    } else {
        (1, (10_u128).pow(liability_decimals - asset_decimals))
    };

    let mut asset_transfer = liability_amount
        .safe_mul(numerator_scale, env)
        .safe_mul(liability_price.cast(env), env)
        .safe_mul(asset_liquidation_multiplier.cast(env), env)
        .safe_div(
            asset_price
                .cast::<u128>(env)
                .safe_mul(liability_liquidation_multiplier.cast(env), env),
            env,
        )
        .safe_div(denominator_scale, env)
        .max(1);

    // Need to check if asset_transfer should be rounded to asset amount
    let (asset_value_numerator_scale, asset_value_denominator_scale) = if asset_decimals > 6 {
        ((10_u128).pow(asset_decimals - 6), 1)
    } else {
        (1, (10_u128).pow(asset_decimals - 6))
    };

    let asset_delta = if asset_transfer > asset_amount {
        asset_transfer - asset_amount
    } else {
        asset_amount - asset_transfer
    };

    let asset_value_delta = asset_delta
        .safe_mul(asset_price.cast(env), env)
        .safe_div(PRICE_PRECISION, env)
        .safe_mul(asset_value_numerator_scale, env)
        .safe_div(asset_value_denominator_scale, env);

    if asset_value_delta < QUOTE_PRECISION {
        asset_transfer = asset_amount;
    }

    asset_transfer
}

pub fn is_position_being_liquidated(
    env: &Env,
    position: &MarketPosition,
    liquidation_margin_buffer_ratio: u32,
) -> bool {
    let margin_calculation = calculate_margin_requirement_and_total_collateral_and_liability_info(
        env,
        position,
        MarginContext::liquidation(liquidation_margin_buffer_ratio),
    )?;

    let is_being_liquidated = !margin_calculation.can_exit_liquidation()?;

    is_being_liquidated
}

pub fn validate_position_not_being_liquidated(
    env: &Env,
    position: &MarketPosition,
    liquidation_margin_buffer_ratio: u32,
) {
    if !position.is_being_liquidated() {
        return;
    }

    let is_still_being_liquidated =
        is_position_being_liquidated(env, position, liquidation_margin_buffer_ratio)?;

    if is_still_being_liquidated {
        panic_with_error!(env, Errors::PositionIsBeingLiquidated);
    } else {
        position.exit_liquidation();
    }
}

#[contracttype]
pub enum LiquidationMultiplierType {
    Discount,
    Premium,
}

pub fn calculate_liquidation_multiplier(
    env: &Env,
    liquidation_fee: u32,
    multiplier_type: LiquidationMultiplierType,
) -> u32 {
    match multiplier_type {
        LiquidationMultiplierType::Premium => {
            LIQUIDATION_FEE_PRECISION.safe_add(liquidation_fee, env)
        }
        LiquidationMultiplierType::Discount => {
            LIQUIDATION_FEE_PRECISION.safe_sub(liquidation_fee, env)
        }
    }
}

// calculate_funding_rate_deltas_to_resolve_bankruptcy

pub fn calculate_cumulative_deposit_interest_delta_to_resolve_bankruptcy(
    env: &Env,
    borrow: u128,
    market: &Market,
) -> u128 {
    let total_deposits =
        get_token_amount(env, market.deposit_balance, market, &BalanceType::Deposit)?;

    market
        .cumulative_deposit_interest
        .safe_mul(borrow)?
        .safe_div_ceil(total_deposits)
        .or(Ok(0))
}

pub fn validate_transfer_satisfies_limit_price(
    env: &Env,
    asset_transfer: u128,
    liability_transfer: u128,
    asset_decimals: u32,
    liability_decimals: u32,
    limit_price: Option<u64>,
) {
    let limit_price = match limit_price {
        Some(limit_price) => limit_price,
        None => {
            return;
        }
    };

    let swap_price = 1;
    // let swap_price = calculate_swap_price(
    //     asset_transfer,
    //     liability_transfer,
    //     asset_decimals,
    //     liability_decimals
    // );

    validate!(
        env,
        swap_price >= limit_price.cast(env),
        Errors::LiquidationDoesntSatisfyLimitPrice,
        "transfer price transfer_price ({}/1000000) < limit price ({}/1000000)",
        swap_price,
        limit_price
    )
}

pub fn calculate_max_pct_to_liquidate(
    env: &Env,
    position: &MarketPosition,
    margin_shortage: u128,
    slot: u64,
    initial_pct_to_liquidate: u128,
    liquidation_duration: u128,
) -> u128 {
    // if margin shortage is tiny, accelerate liquidation
    if margin_shortage < 50 * QUOTE_PRECISION {
        return LIQUIDATION_PCT_PRECISION;
    }

    let slots_elapsed = slot.safe_sub(position.last_active_slot, env);

    let pct_freeable = slots_elapsed
        .cast::<u128>(env)
        .safe_mul(LIQUIDATION_PCT_PRECISION, env)
        .safe_div(liquidation_duration, env) // ~ 1 minute if per slot is 400ms
        .unwrap_or(LIQUIDATION_PCT_PRECISION) // if divide by zero, default to 100%
        .safe_add(initial_pct_to_liquidate)
        .min(LIQUIDATION_PCT_PRECISION);

    let total_margin_shortage =
        margin_shortage.safe_add(position.liquidation_margin_freed.cast(env), env);
    let max_margin_freed = total_margin_shortage
        .safe_mul(pct_freeable, env)
        .safe_div(LIQUIDATION_PCT_PRECISION, env);
    let margin_freeable =
        max_margin_freed.saturating_sub(position.liquidation_margin_freed.cast(env));

    margin_freeable
        .safe_mul(LIQUIDATION_PCT_PRECISION, env)
        .safe_div(margin_shortage, env)
}

pub fn calculate_position_insurance_fund_fee(
    env: &Env,
    margin_shortage: u128,
    user_base_asset_amount: u64,
    margin_ratio: u32,
    liquidator_fee: u32,
    oracle_price: i64,
    quote_oracle_price: i64,
    max_if_liquidation_fee: u32,
) -> u32 {
    let margin_ratio = margin_ratio.safe_mul(LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO, env);

    if oracle_price == 0
        || quote_oracle_price == 0
        || margin_ratio <= liquidator_fee
        || user_base_asset_amount == 0
    {
        return 0;
    }

    let price = oracle_price
        .cast::<u128>(env)
        .safe_mul(quote_oracle_price.cast(env), env)
        .safe_div(PRICE_PRECISION, env);

    // margin ratio - liquidator fee - (margin shortage / (user base asset amount * price))
    let implied_if_fee = margin_ratio
        .saturating_sub(liquidator_fee)
        .saturating_sub(
            margin_shortage
                .safe_mul(BASE_PRECISION, env)
                .safe_div(user_base_asset_amount.cast(env), env)
                .safe_mul(PRICE_PRECISION, env)
                .safe_div(price, env)
                .cast::<u32>(env)
                .unwrap_or(u32::MAX),
        )
        // multiply by 95% to avoid situation where fee leads to deposits == negative pnl
        // leading to bankruptcy
        .safe_mul(19, env)
        .safe_div(20, env);

    max_if_liquidation_fee.min(implied_if_fee)
}

// TODO: rework for DEX swap
// pub fn get_liquidation_order_params(
//     market_index: u16,
//     existing_direction: OrderDirection,
//     base_asset_amount: u64,
//     oracle_price: i64,
//     liquidation_fee: u32
// ) -> OrderParams {
//     let direction = existing_direction.opposite();

//     let oracle_price_u128 = oracle_price.abs().cast::<u128>()?;
//     let limit_price = match direction {
//         PositionDirection::Long =>
//             oracle_price_u128
//                 .safe_add(
//                     oracle_price_u128
//                         .safe_mul(liquidation_fee.cast()?)?
//                         .safe_div(LIQUIDATION_FEE_PRECISION_U128)?
//                 )?
//                 .cast::<u64>()?,
//         PositionDirection::Short =>
//             oracle_price_u128
//                 .safe_sub(
//                     oracle_price_u128
//                         .safe_mul(liquidation_fee.cast()?)?
//                         .safe_div(LIQUIDATION_FEE_PRECISION_U128)?
//                 )?
//                 .cast::<u64>()?,
//     };

//     let order_params = OrderParams {
//         market_index,
//         direction,
//         price: limit_price,
//         order_type: OrderType::Limit,
//         market_type: MarketType::Perp,
//         base_asset_amount,
//         reduce_only: true,
//         ..OrderParams::default()
//     };

//     order_params
// }

pub fn get_liquidation_fee(
    env: &Env,
    base_liquidation_fee: u32,
    max_liquidation_fee: u32,
    last_active_user_ts: u64,
    current_ts: u64,
) -> u32 {
    let slots_elapsed = current_ts.safe_sub(last_active_user_ts, env);
    if slots_elapsed < LIQUIDATION_FEE_ADJUST_GRACE_PERIOD_SLOTS {
        return base_liquidation_fee;
    }

    let liquidation_fee = base_liquidation_fee.saturating_add(
        slots_elapsed
            .safe_mul(LIQUIDATION_FEE_INCREASE_PER_SLOT.cast::<u64>(env), env)
            .cast::<u32>(env)
            .unwrap_or(u32::MAX),
    );
    liquidation_fee.min(max_liquidation_fee)
}
