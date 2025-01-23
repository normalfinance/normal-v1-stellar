use normal::{ error::ErrorCode, math::{ casting::Cast, safe_math::SafeMath }, validate };
use soroban_sdk::{ log, Address, Env };

use crate::{
    events::InsuranceFundEvents,
    math::insurance::{
        calculate_if_shares_lost,
        calculate_rebase_info,
        if_shares_to_vault_amount,
        vault_amount_to_if_shares,
    },
    storage::{ save_insurance_fund, save_stake, InsuranceFund, Stake, StakeAction },
};

pub fn add_stake(
    env: &Env,
    insurance_fund: &mut InsuranceFund,
    amount: u64,
    insurance_balance: u64,
    stake: &mut Stake,
    now: u64
) -> NormalResult {
    validate!(
        !(insurance_balance == 0 && insurance_fund.total_shares != 0),
        ErrorCode::InvalidIFForNewStakes,
        "Insurance Fund balance should be non-zero for new stakers to enter"
    )?;

    apply_rebase_to_insurance_fund(env, insurance_balance, insurance_fund)?;
    apply_rebase_to_stake(env, stake, insurance_fund)?;

    let if_shares_before = stake.checked_if_shares(insurance_fund)?;
    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    let n_shares = vault_amount_to_if_shares(
        env,
        amount,
        insurance_fund.total_shares,
        insurance_balance
    )?;

    // reset cost basis if no shares
    stake.cost_basis = if if_shares_before == 0 {
        amount.cast(env)?
    } else {
        stake.cost_basis.safe_add(amount.cast(env)?, env)?
    };

    stake.increase_if_shares(n_shares, insurance_fund)?;

    insurance_fund.total_shares = insurance_fund.total_shares.safe_add(n_shares, env)?;

    insurance_fund.user_shares = insurance_fund.user_shares.safe_add(n_shares, env)?;

    let if_shares_after = stake.checked_if_shares(insurance_fund)?;

    // TODO: must we manually save here?
    save_insurance_fund(env, insurance_fund);
    save_stake(env, user, stake);

    InsuranceFundEvents::stake_record(
        env,
        now,
        user,
        StakeAction::Stake,
        amount,
        insurance_vault_amount,
        if_shares_before,
        user_if_shares_before,
        total_if_shares_before,
        if_shares_after,
        insurance_fund.total_shares,
        insurance_fund.user_shares
    );

    Ok(())
}

pub fn apply_rebase_to_insurance_fund(
    env: &Env,
    insurance_fund_vault_balance: u64,
    insurance_fund: &mut InsuranceFund
) -> NormalResult {
    if
        insurance_fund_vault_balance != 0 &&
        insurance_fund_vault_balance.cast::<u128>(env)? < insurance_fund.total_shares
    {
        let (expo_diff, rebase_divisor) = calculate_rebase_info(
            env,
            insurance_fund.total_shares,
            insurance_fund_vault_balance
        )?;

        insurance_fund.total_shares = insurance_fund.total_shares.safe_div(rebase_divisor, env)?;
        insurance_fund.user_shares = insurance_fund.user_shares.safe_div(rebase_divisor, env)?;
        insurance_fund.shares_base = insurance_fund.shares_base.safe_add(
            expo_diff.cast::<u128>(env)?,
            env
        )?;

        log!(env, "rebasing insurance fund: expo_diff={}", expo_diff);
    }

    if insurance_fund_vault_balance != 0 && insurance_fund.total_shares == 0 {
        insurance_fund.total_shares = insurance_fund_vault_balance.cast::<u128>(env)?;
    }

    Ok(())
}

pub fn apply_rebase_to_stake(
    env: &Env,
    stake: &mut Stake,
    insurance_fund: &mut InsuranceFund
) -> NormalResult {
    if insurance_fund.shares_base != stake.if_base {
        validate!(
            insurance_fund.shares_base > stake.if_base,
            ErrorCode::InvalidIFRebase,
            "Rebase expo out of bounds"
        )?;

        let expo_diff = (insurance_fund.shares_base - stake.if_base).cast::<u32>(env)?;

        let rebase_divisor = (10_u128).pow(expo_diff);

        log!(
            env,
            "rebasing insurance fund stake: base: {} -> {} ",
            stake.if_base,
            insurance_fund.shares_base
        );

        stake.if_base = insurance_fund.shares_base;

        let old_if_shares = stake.unchecked_if_shares();
        let new_if_shares = old_if_shares.safe_div(rebase_divisor, env)?;

        log!(env, "rebasing insurance fund stake: shares -> {} ", new_if_shares);

        stake.update_if_shares(new_if_shares, insurance_fund)?;

        stake.last_withdraw_request_shares = stake.last_withdraw_request_shares.safe_div(
            rebase_divisor,
            env
        )?;
    }

    Ok(())
}

pub fn request_remove_stake(
    env: &Env,
    n_shares: u128,
    insurance_vault_amount: u64,
    stake: &mut Stake,
    insurance_fund: &mut InsuranceFund,
    now: u64
) -> NormalResult {
    log!(env, "n_shares {}", n_shares);
    stake.last_withdraw_request_shares = n_shares;

    apply_rebase_to_insurance_fund(env, insurance_vault_amount, insurance_fund)?;
    apply_rebase_to_stake(env, stake, insurance_fund)?;

    let if_shares_before = stake.checked_if_shares(insurance_fund)?;
    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    validate!(
        stake.last_withdraw_request_shares <= stake.checked_if_shares(insurance_fund)?,
        ErrorCode::InvalidInsuranceUnstakeSize,
        "last_withdraw_request_shares exceeds if_shares {} > {}"
        // stake.last_withdraw_request_shares,
        // stake.checked_if_shares(insurance_fund)?
    )?;

    validate!(
        stake.if_base == insurance_fund.shares_base,
        ErrorCode::InvalidIFRebase,
        "if stake base != spot market base"
    )?;

    stake.last_withdraw_request_value = if_shares_to_vault_amount(
        env,
        stake.last_withdraw_request_shares,
        insurance_fund.total_shares,
        insurance_vault_amount
    )?.min(insurance_vault_amount.saturating_sub(1));

    validate!(
        stake.last_withdraw_request_value == 0 ||
            stake.last_withdraw_request_value < insurance_vault_amount,
        ErrorCode::InvalidIFUnstakeSize,
        "Requested withdraw value is not below Insurance Fund balance"
    )?;

    let if_shares_after = stake.checked_if_shares(insurance_fund)?;

    // TODO: must we manually save here?
    save_insurance_fund(env, insurance_fund);

    InsuranceFundEvents::stake_record(
        env,
        now,
        user,
        StakeAction::Unstake,
        stake.last_withdraw_request_value,
        insurance_vault_amount,
        if_shares_before,
        user_if_shares_before,
        total_if_shares_before,
        if_shares_after,
        insurance_fund.total_shares,
        insurance_fund.user_shares
    );

    stake.last_withdraw_request_ts = now;

    // TODO: must we manually save here?
    save_stake(env, user, stake);

    Ok(())
}

pub fn cancel_request_remove_stake(
    env: &Env,
    insurance_vault_amount: u64,
    insurance_fund: &mut InsuranceFund,
    stake: &mut Stake,
    now: u64
) -> NormalResult {
    apply_rebase_to_insurance_fund(env, insurance_vault_amount, insurance_fund)?;
    apply_rebase_to_stake(env, stake, insurance_fund)?;

    let if_shares_before = stake.checked_if_shares(insurance_fund)?;
    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    validate!(
        stake.if_base == insurance_fund.shares_base,
        ErrorCode::InvalidIFRebase,
        "if stake base != spot market base"
    )?;

    validate!(
        stake.last_withdraw_request_shares != 0,
        ErrorCode::InvalidIFUnstakeCancel,
        "No withdraw request in progress"
    )?;

    let if_shares_lost = calculate_if_shares_lost(
        env,
        stake,
        insurance_fund,
        insurance_vault_amount
    )?;

    stake.decrease_if_shares(if_shares_lost, insurance_fund)?;

    insurance_fund.total_shares = insurance_fund.total_shares.safe_sub(if_shares_lost, env)?;

    insurance_fund.user_shares = insurance_fund.user_shares.safe_sub(if_shares_lost, env)?;

    let if_shares_after = stake.checked_if_shares(insurance_fund)?;

    // TODO: must we manually save here?
    save_insurance_fund(env, insurance_fund);
    
    InsuranceFundEvents::stake_record(
        env,
        now,
        user,
        StakeAction::UnstakeCancelRequest,
        0,
        insurance_vault_amount,
        if_shares_before,
        user_if_shares_before,
        total_if_shares_before,
        if_shares_after,
        insurance_fund.total_shares,
        insurance_fund.user_shares
    );

    stake.last_withdraw_request_shares = 0;
    stake.last_withdraw_request_value = 0;
    stake.last_withdraw_request_ts = now;

    Ok(())
}

pub fn remove_stake(
    env: &Env,
    insurance_vault_amount: u64,
    stake: &mut InsuranceFundStake,
    insurance_fund: &mut InsuranceFund,
    now: u64
) -> NormalResult<u64> {
    let time_since_withdraw_request = now.safe_sub(stake.last_withdraw_request_ts, env)?;

    validate!(
        time_since_withdraw_request >= insurance_fund.unstaking_period,
        ErrorCode::TryingToRemoveLiquidityTooFast,
        ""
    )?;

    apply_rebase_to_insurance_fund(env, insurance_vault_amount, insurance_fund)?;
    apply_rebase_to_stake(env, stake, insurance_fund)?;

    let if_shares_before = stake.checked_if_shares(insurance_fund)?;
    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    let n_shares = stake.last_withdraw_request_shares;

    validate!(
        n_shares > 0,
        ErrorCode::InvalidIFUnstake,
        "Must submit withdraw request and wait the escrow period"
    )?;

    validate!(if_shares_before >= n_shares, ErrorCode::InsufficientIFShares, "")?;

    let amount = if_shares_to_vault_amount(
        env,
        n_shares,
        insurance_fund.total_shares,
        insurance_vault_amount
    )?;

    let _if_shares_lost = calculate_if_shares_lost(
        env,
        stake,
        insurance_fund,
        insurance_vault_amount
    )?;

    let withdraw_amount = amount.min(stake.last_withdraw_request_value);

    stake.decrease_if_shares(n_shares, insurance_fund)?;

    stake.cost_basis = stake.cost_basis.safe_sub(withdraw_amount.cast(env)?, env)?;

    insurance_fund.total_shares = insurance_fund.total_shares.safe_sub(n_shares, env)?;

    insurance_fund.user_shares = insurance_fund.user_shares.safe_sub(n_shares, env)?;

    // reset stake withdraw request info
    stake.last_withdraw_request_shares = 0;
    stake.last_withdraw_request_value = 0;
    stake.last_withdraw_request_ts = now;

    let if_shares_after = stake.checked_if_shares(insurance_fund)?;

    InsuranceFundEvents::stake_record(
        env,
        now,
        user,
        StakeAction::Unstake,
        withdraw_amount,
        insurance_vault_amount,
        if_shares_before,
        user_if_shares_before,
        total_if_shares_before,
        if_shares_after,
        insurance_fund.total_shares,
        insurance_fund.user_shares
    );

    Ok(withdraw_amount)
}
