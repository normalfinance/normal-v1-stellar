use normal::{
    error::{ ErrorCode, NormalResult },
    math::{ casting::Cast, helpers::{ get_proportion_u128, log10_iter }, safe_math::SafeMath },
    validate,
};
use soroban_sdk::{ Env, log };

use crate::storage::{ InsuranceFund, Stake };

pub fn vault_amount_to_if_shares(
    env: &Env,
    amount: i128,
    total_if_shares: u128,
    insurance_vault_amount: i128
) -> NormalResult<u128> {
    // relative to the entire pool + total amount minted
    let n_shares = if insurance_vault_amount > 0 {
        // assumes total_if_shares != 0 (in most cases) for nice result for user

        get_proportion_u128(
            env,
            amount.cast::<u128>(env)?,
            total_if_shares,
            insurance_vault_amount.cast::<u128>(env)?
        )?
    } else {
        // must be case that total_if_shares == 0 for nice result for user
        validate!(
            env,
            total_if_shares == 0,
            ErrorCode::InvalidIFSharesDetected,
            "assumes total_if_shares == 0"
        )?;

        amount.cast::<u128>(env)?
    };

    Ok(n_shares)
}

pub fn if_shares_to_vault_amount(
    env: &Env,
    n_shares: u128,
    total_if_shares: u128,
    insurance_vault_amount: i128
) -> NormalResult<i128> {
    validate!(
        env,
        n_shares <= total_if_shares,
        ErrorCode::InvalidIFSharesDetected,
        "n_shares({}) > total_if_shares({})",
        n_shares,
        total_if_shares
    )?;

    let amount = if total_if_shares > 0 {
        get_proportion_u128(
            env,
            insurance_vault_amount as u128,
            n_shares,
            total_if_shares
        )?.cast::<i128>(env)?
    } else {
        0
    };

    Ok(amount)
}

pub fn calculate_rebase_info(
    env: &Env,
    total_if_shares: u128,
    insurance_vault_amount: i128
) -> NormalResult<(u32, u128)> {
    let rebase_divisor_full = total_if_shares
        .safe_div(10, env)?
        .safe_div(insurance_vault_amount.cast::<u128>(env)?, env)?;

    let expo_diff = log10_iter(rebase_divisor_full).cast::<u32>(env)?;
    let rebase_divisor = (10_u128).pow(expo_diff);

    Ok((expo_diff, rebase_divisor))
}

pub fn calculate_if_shares_lost(
    env: &Env,
    stake: &Stake,
    insurance_fund: &InsuranceFund,
    insurance_vault_amount: i128
) -> NormalResult<u128> {
    let n_shares = stake.last_withdraw_request_shares;

    let amount = if_shares_to_vault_amount(
        env,
        n_shares,
        insurance_fund.total_shares,
        insurance_vault_amount
    )?;

    let if_shares_lost = if amount > stake.last_withdraw_request_value {
        let new_n_shares = vault_amount_to_if_shares(
            env,
            stake.last_withdraw_request_value,
            insurance_fund.total_shares.safe_sub(n_shares, env)?,
            insurance_vault_amount.safe_sub(stake.last_withdraw_request_value, env)?
        )?;

        validate!(
            env,
            new_n_shares <= n_shares,
            ErrorCode::InvalidIFSharesDetected,
            "Issue calculating delta if_shares after canceling request {} < {}",
            new_n_shares,
            n_shares
        )?;

        n_shares.safe_sub(new_n_shares, env)?
    } else {
        0
    };

    Ok(if_shares_lost)
}
