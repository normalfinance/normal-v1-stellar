// pub fn calculate_min_deposit_token_amount(
//     deposit_token_twap: u128,
//     withdraw_guard_threshold: u128,
// ) -> DriftResult<u128> {
//     // minimum required deposit amount after withdrawal
//     // minimum deposit amount lower of 75% of TWAP or withdrawal guard threshold below TWAP
//     // for high withdrawal guard threshold, minimum deposit amount is 0

//     let min_deposit_token = deposit_token_twap
//         .safe_sub((deposit_token_twap / 4).max(withdraw_guard_threshold.min(deposit_token_twap)))?;

//     Ok(min_deposit_token)
// }

pub fn calculate_max_borrow_token_amount(
    deposit_token_amount: u128,
    deposit_token_twap: u128,
    borrow_token_twap: u128,
    withdraw_guard_threshold: u128,
    max_token_borrows: u128,
) -> u128 {
    // maximum permitted borrows after withdrawal
    // allows at least up to the withdraw_guard_threshold
    // and between ~15-80% utilization with friction on twap in 10% increments

    let lesser_deposit_amount = deposit_token_amount.min(deposit_token_twap);

    let max_borrow_token = withdraw_guard_threshold
        .max(
            (lesser_deposit_amount / 6)
                .max(borrow_token_twap.safe_add(lesser_deposit_amount / 10)?)
                .min(lesser_deposit_amount.safe_sub(lesser_deposit_amount / 5)?),
        )
        .min(max_token_borrows);

    max_borrow_token
}

// pub fn calculate_token_utilization_limits(
//     deposit_token_amount: u128,
//     borrow_token_amount: u128,
//     spot_market: &SpotMarket,
// ) -> DriftResult<(u128, u128)> {
//     // Calculates the allowable minimum deposit and maximum borrow amounts after withdrawal based on market utilization.
//     // First, it determines a maximum withdrawal utilization from the market's target and historic utilization.
//     // Then, it deduces corresponding deposit/borrow amounts.
//     // Note: For deposit sizes below the guard threshold, withdrawals aren't blocked.

//     let max_withdraw_utilization: u128 = spot_market.optimal_utilization.cast::<u128>()?.max(
//         spot_market.utilization_twap.cast::<u128>()?.safe_add(
//             SPOT_UTILIZATION_PRECISION.saturating_sub(spot_market.utilization_twap.cast()?) / 2,
//         )?,
//     );

//     let mut min_deposit_tokens_for_utilization = borrow_token_amount
//         .safe_mul(SPOT_UTILIZATION_PRECISION)?
//         .safe_div(max_withdraw_utilization)?;

//     // dont block withdraws for deposit sizes below guard threshold
//     min_deposit_tokens_for_utilization = min_deposit_tokens_for_utilization
//         .min(deposit_token_amount.saturating_sub(spot_market.withdraw_guard_threshold.cast()?));

//     let mut max_borrow_tokens_for_utilization = max_withdraw_utilization
//         .safe_mul(deposit_token_amount)?
//         .safe_div(SPOT_UTILIZATION_PRECISION)?;

//     // dont block borrows for sizes below guard threshold
//     max_borrow_tokens_for_utilization =
//         max_borrow_tokens_for_utilization.max(spot_market.withdraw_guard_threshold.cast()?);

//     Ok((
//         min_deposit_tokens_for_utilization,
//         max_borrow_tokens_for_utilization,
//     ))
// }

use soroban_sdk::Env;

use crate::state::{market::Market, market_position::MarketPosition};

use super::balance::{get_token_amount, BalanceType};

pub fn check_withdraw_limits(
    env: &Env,
    market: &Market,
    position: &MarketPosition,
    token_amount_withdrawn: Option<u128>,
) -> bool {
    // calculates min/max deposit/borrow amounts permitted for immediate withdraw
    // takes the stricter of absolute caps on level changes and utilization changes vs 24hr moving averrages
    let deposit_token_amount = get_token_amount(
        env,
        market.collateral.balance,
        market,
        &BalanceType::Deposit,
    );
    let borrow_token_amount =
        get_token_amount(env, market.synthetic.balance, market, &BalanceType::Borrow);

    let max_token_borrows: u128 = if market.max_token_borrows_fraction > 0 {
        market
            .max_token_deposits
            .safe_mul(market.max_token_borrows_fraction.cast())
            .safe_div(10000)
            .cast()
    } else {
        u128::MAX
    };

    let max_borrow_token_for_twap = calculate_max_borrow_token_amount(
        deposit_token_amount,
        market.deposit_token_twap.cast(),
        market.borrow_token_twap.cast(),
        market.withdraw_guard_threshold.cast(),
        max_token_borrows,
    );

    let (min_deposit_token_for_utilization, max_borrow_token_for_utilization) =
        calculate_token_utilization_limits(deposit_token_amount, borrow_token_amount, market);

    let max_borrow_token = max_borrow_token_for_twap.min(max_borrow_token_for_utilization);

    let min_deposit_token_for_twap = calculate_min_deposit_token_amount(
        market.deposit_token_twap.cast(),
        market.withdraw_guard_threshold.cast(),
    );

    let min_deposit_token = min_deposit_token_for_twap.max(min_deposit_token_for_utilization);

    // for resulting deposit or ZERO, check if deposits above minimum
    // for resulting borrow, check both deposit and borrow constraints
    let valid_global_withdrawal = if let Some(user) = user {
        if position.balance_type == &BalanceType::Borrow {
            borrow_token_amount <= max_borrow_token && deposit_token_amount >= min_deposit_token
        } else {
            deposit_token_amount >= min_deposit_token
        }
    } else {
        deposit_token_amount >= min_deposit_token && borrow_token_amount <= max_borrow_token
    };

    valid_global_withdrawal
}

// pub fn get_max_withdraw_for_market_with_token_amount(
//     spot_market: &SpotMarket,
//     token_amount: i128,
//     is_leaving_drift: bool,
// ) -> DriftResult<u128> {
//     let deposit_token_amount = get_token_amount(
//         spot_market.collateral.balance,
//         spot_market,
//         &BalanceType::Deposit,
//     )?;

//     let borrow_token_amount = get_token_amount(
//         spot_market.synthetic.balance,
//         spot_market,
//         &BalanceType::Borrow,
//     )?;

//     // if leaving drift, need to consider utilization limits
//     let (min_deposit_token_for_utilization, max_borrow_token_for_utilization) = if is_leaving_drift
//     {
//         calculate_token_utilization_limits(deposit_token_amount, borrow_token_amount, spot_market)?
//     } else {
//         (0, u128::MAX)
//     };

//     let mut max_withdraw_amount = 0_u128;
//     if token_amount > 0 {
//         let min_deposit_token_for_twap = calculate_min_deposit_token_amount(
//             spot_market.deposit_token_twap.cast()?,
//             spot_market.withdraw_guard_threshold.cast()?,
//         )?;
//         let min_deposit_token = min_deposit_token_for_twap.max(min_deposit_token_for_utilization);
//         let withdraw_limit = deposit_token_amount.saturating_sub(min_deposit_token);

//         let token_amount = token_amount.unsigned_abs();
//         if withdraw_limit <= token_amount && is_leaving_drift {
//             return Ok(withdraw_limit);
//         }

//         max_withdraw_amount = token_amount;
//     }

//     let max_token_borrows: u128 = if spot_market.max_token_borrows_fraction > 0 {
//         spot_market
//             .max_token_deposits
//             .safe_mul(spot_market.max_token_borrows_fraction.cast()?)?
//             .safe_div(10000)?
//             .cast()?
//     } else {
//         u128::MAX
//     };

//     let max_borrow_token_for_twap = calculate_max_borrow_token_amount(
//         deposit_token_amount,
//         spot_market.deposit_token_twap.cast()?,
//         spot_market.borrow_token_twap.cast()?,
//         spot_market.withdraw_guard_threshold.cast()?,
//         max_token_borrows,
//     )?;

//     let max_borrow_token = max_borrow_token_for_twap.min(max_borrow_token_for_utilization);

//     let mut borrow_limit = max_borrow_token
//         .saturating_sub(borrow_token_amount)
//         .min(deposit_token_amount.saturating_sub(borrow_token_amount));

//     if spot_market.max_token_borrows_fraction > 0 {
//         // min with max allowed borrows
//         let borrows = spot_market.get_borrows()?;
//         let max_token_borrows = spot_market
//             .max_token_deposits
//             .safe_mul(spot_market.max_token_borrows_fraction.cast()?)?
//             .safe_div(10000)?
//             .cast::<u128>()?;
//         borrow_limit = borrow_limit.min(max_token_borrows.saturating_sub(borrows));
//     }

//     max_withdraw_amount.safe_add(borrow_limit)
// }

// pub fn validate_spot_balances(spot_market: &SpotMarket) -> DriftResult<i64> {
//     let depositors_amount: u64 = get_token_amount(
//         spot_market.collateral.balance,
//         spot_market,
//         &BalanceType::Deposit,
//     )?
//     .cast()?;
//     let borrowers_amount: u64 = get_token_amount(
//         spot_market.synthetic.balance,
//         spot_market,
//         &BalanceType::Borrow,
//     )?
//     .cast()?;

//     let revenue_amount: u64 = get_token_amount(
//         spot_market.revenue_pool.scaled_balance,
//         spot_market,
//         &BalanceType::Deposit,
//     )?
//     .cast()?;

//     let depositors_claim = depositors_amount
//         .cast::<i64>()?
//         .safe_sub(borrowers_amount.cast()?)?;

//     validate!(
//         revenue_amount <= depositors_amount,
//         ErrorCode::SpotMarketVaultInvariantViolated,
//         "revenue_amount={} greater or equal to the depositors_amount={} (depositors_claim={}, spot_market.collateral.balance={})",
//         revenue_amount,
//         depositors_amount,
//         depositors_claim,
//         spot_market.collateral.balance
//     )?;

//     Ok(depositors_claim)
// }

// pub fn validate_spot_market_vault_amount(
//     spot_market: &SpotMarket,
//     vault_amount: u64,
// ) -> DriftResult<i64> {
//     let depositors_claim = validate_spot_balances(spot_market)?;

//     validate!(
//         vault_amount.cast::<i64>()? >= depositors_claim,
//         ErrorCode::SpotMarketVaultInvariantViolated,
//         "spot market vault ={} holds less than remaining depositor claims = {}",
//         vault_amount,
//         depositors_claim
//     )?;

//     Ok(depositors_claim)
// }
