use normal::{ math::{ casting::Cast, safe_math::SafeMath }, validate };
use soroban_sdk::{ log, Address, Env };

use crate::{
    errors::Errors,
    events::InsuranceFundEvents,
    math::insurance::{
        calculate_if_shares_lost,
        calculate_rebase_info,
        if_shares_to_vault_amount,
        vault_amount_to_if_shares,
    },
    storage::{ InsuranceFund, Stake, StakeAction },
};

pub fn add_stake(
    env: &Env,
    user: &Address,
    amount: i128,
    insurance_vault_amount: i128,
    stake: &mut Stake,
    insurance_fund: &mut InsuranceFund,
    now: u64
) {
    validate!(
        env,
        !(insurance_vault_amount == 0 && insurance_fund.total_shares != 0),
        Errors::IFWithdrawRequestTooSmall,
        "Insurance Fund balance should be non-zero for new stakers to enter"
    );

    apply_rebase_to_insurance_fund(env, insurance_vault_amount, insurance_fund);
    apply_rebase_to_stake(env, stake, insurance_fund);

    let if_shares_before = stake.checked_if_shares(env, insurance_fund);
    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    let n_shares = vault_amount_to_if_shares(
        env,
        amount,
        insurance_fund.total_shares,
        insurance_vault_amount
    );

    // reset cost basis if no shares
    stake.cost_basis = if if_shares_before == 0 {
        amount.cast(env)
    } else {
        stake.cost_basis.safe_add(amount.cast(env), env)
    };

    stake.increase_if_shares(env, n_shares, insurance_fund);

    insurance_fund.total_shares = insurance_fund.total_shares.safe_add(n_shares, env);

    insurance_fund.user_shares = insurance_fund.user_shares.safe_add(n_shares, env);

    let if_shares_after = stake.checked_if_shares(env, insurance_fund);

    InsuranceFundEvents::if_stake_record(
        env,
        now,
        user.clone(),
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

    // Ok(())
}

pub fn apply_rebase_to_insurance_fund(
    env: &Env,
    insurance_vault_amount: i128,
    insurance_fund: &mut InsuranceFund
) {
    if
        insurance_vault_amount != 0 &&
        insurance_vault_amount.cast::<u128>(env) < insurance_fund.total_shares
    {
        let (expo_diff, rebase_divisor) = calculate_rebase_info(
            env,
            insurance_fund.total_shares,
            insurance_vault_amount
        );

        insurance_fund.total_shares = insurance_fund.total_shares.safe_div(rebase_divisor, env);
        insurance_fund.user_shares = insurance_fund.user_shares.safe_div(rebase_divisor, env);
        insurance_fund.shares_base = insurance_fund.shares_base.safe_add(
            expo_diff.cast::<u128>(env),
            env
        );

        log!(env, "rebasing insurance fund: expo_diff={}", expo_diff);
    }

    if insurance_vault_amount != 0 && insurance_fund.total_shares == 0 {
        insurance_fund.total_shares = insurance_vault_amount.cast::<u128>(env);
    }
}

pub fn apply_rebase_to_stake(env: &Env, stake: &mut Stake, insurance_fund: &mut InsuranceFund) {
    if insurance_fund.shares_base != stake.if_base {
        validate!(
            env,
            insurance_fund.shares_base > stake.if_base,
            Errors::InvalidIFRebase,
            "Rebase expo out of bounds"
        );

        let expo_diff = (insurance_fund.shares_base - stake.if_base).cast::<u32>(env);

        let rebase_divisor = (10_u128).pow(expo_diff);

        log!(
            env,
            "rebasing insurance fund stake: base: {} -> {} ",
            stake.if_base,
            insurance_fund.shares_base
        );

        stake.if_base = insurance_fund.shares_base;

        let old_if_shares = stake.unchecked_if_shares();
        let new_if_shares = old_if_shares.safe_div(rebase_divisor, env);

        log!(env, "rebasing insurance fund stake: shares -> {} ", new_if_shares);

        stake.update_if_shares(env, new_if_shares, insurance_fund);

        stake.last_withdraw_request_shares = stake.last_withdraw_request_shares.safe_div(
            rebase_divisor,
            env
        );
    }
}

pub fn request_remove_stake(
    env: &Env,
    user: &Address,
    n_shares: u128,
    insurance_vault_amount: i128,
    stake: &mut Stake,
    insurance_fund: &mut InsuranceFund,
    now: u64
) {
    log!(env, "n_shares {}", n_shares);
    stake.last_withdraw_request_shares = n_shares;

    apply_rebase_to_insurance_fund(env, insurance_vault_amount, insurance_fund);
    apply_rebase_to_stake(env, stake, insurance_fund);

    let if_shares_before = stake.checked_if_shares(env, insurance_fund);
    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    validate!(
        env,
        stake.last_withdraw_request_shares <= stake.checked_if_shares(env, insurance_fund),
        Errors::InvalidInsuranceUnstakeSize,
        "last_withdraw_request_shares exceeds if_shares {} > {}" // stake.last_withdraw_request_shares,
        // stake.checked_if_shares(insurance_fund)
    );

    validate!(
        env,
        stake.if_base == insurance_fund.shares_base,
        Errors::InvalidIFRebase,
        "if stake base != spot market base"
    );

    stake.last_withdraw_request_value = if_shares_to_vault_amount(
        env,
        stake.last_withdraw_request_shares,
        insurance_fund.total_shares,
        insurance_vault_amount
    ).min(insurance_vault_amount.saturating_sub(1));

    validate!(
        env,
        stake.last_withdraw_request_value == 0 ||
            stake.last_withdraw_request_value < insurance_vault_amount,
        Errors::InvalidIFUnstakeSize,
        "Requested withdraw value is not below Insurance Fund balance"
    );

    let if_shares_after = stake.checked_if_shares(env, insurance_fund);

    InsuranceFundEvents::if_stake_record(
        env,
        now,
        user.clone(),
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
}

pub fn cancel_request_remove_stake(
    env: &Env,
    user: &Address,
    insurance_vault_amount: i128,
    insurance_fund: &mut InsuranceFund,
    stake: &mut Stake,
    now: u64
) {
    apply_rebase_to_insurance_fund(env, insurance_vault_amount, insurance_fund);
    apply_rebase_to_stake(env, stake, insurance_fund);

    let if_shares_before = stake.checked_if_shares(env, insurance_fund);
    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    validate!(
        &env,
        stake.if_base == insurance_fund.shares_base,
        Errors::InvalidIFRebase,
        "if stake base != spot market base"
    );

    validate!(
        &env,
        stake.last_withdraw_request_shares != 0,
        Errors::InvalidIFUnstakeCancel,
        "No withdraw request in progress"
    );

    let if_shares_lost = calculate_if_shares_lost(
        env,
        stake,
        insurance_fund,
        insurance_vault_amount
    );

    stake.decrease_if_shares(env, if_shares_lost, insurance_fund);

    insurance_fund.total_shares = insurance_fund.total_shares.safe_sub(if_shares_lost, env);

    insurance_fund.user_shares = insurance_fund.user_shares.safe_sub(if_shares_lost, env);

    let if_shares_after = stake.checked_if_shares(env, insurance_fund);

    InsuranceFundEvents::if_stake_record(
        env,
        now,
        user.clone(),
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
}

pub fn remove_stake(
    env: &Env,
    user: &Address,
    insurance_vault_amount: i128,
    stake: &mut Stake,
    insurance_fund: &mut InsuranceFund,
    now: u64
) -> i128 {
    let time_since_withdraw_request = now.safe_sub(stake.last_withdraw_request_ts, env);

    validate!(
        env,
        time_since_withdraw_request >= insurance_fund.unstaking_period.cast::<u64>(env),
        Errors::TryingToRemoveLiquidityTooFast,
        ""
    );

    apply_rebase_to_insurance_fund(env, insurance_vault_amount, insurance_fund);
    apply_rebase_to_stake(env, stake, insurance_fund);

    let if_shares_before = stake.checked_if_shares(env, insurance_fund);
    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    let n_shares = stake.last_withdraw_request_shares;

    validate!(
        env,
        n_shares > 0,
        Errors::InvalidIFUnstake,
        "Must submit withdraw request and wait the escrow period"
    );

    validate!(env, if_shares_before >= n_shares, Errors::InsufficientIFShares, "");

    let amount = if_shares_to_vault_amount(
        env,
        n_shares,
        insurance_fund.total_shares,
        insurance_vault_amount
    );

    let _if_shares_lost = calculate_if_shares_lost(
        env,
        stake,
        insurance_fund,
        insurance_vault_amount
    );

    let withdraw_amount = amount.min(stake.last_withdraw_request_value);

    stake.decrease_if_shares(env, n_shares, insurance_fund);

    stake.cost_basis = stake.cost_basis.safe_sub(withdraw_amount.cast(env), env);

    insurance_fund.total_shares = insurance_fund.total_shares.safe_sub(n_shares, env);

    insurance_fund.user_shares = insurance_fund.user_shares.safe_sub(n_shares, env);

    // reset stake withdraw request info
    stake.last_withdraw_request_shares = 0;
    stake.last_withdraw_request_value = 0;
    stake.last_withdraw_request_ts = now;

    let if_shares_after = stake.checked_if_shares(env, insurance_fund);

    InsuranceFundEvents::if_stake_record(
        env,
        now,
        user.clone(),
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

    withdraw_amount
}
