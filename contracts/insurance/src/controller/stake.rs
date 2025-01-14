use normal::{ error::ErrorCode, validate };
use soroban_sdk::{ log, Address, Env };

use crate::{ events::InsuranceFundEvents, storage::{ InsuranceFund, Stake } };

pub fn add_stake(
    env: &Env,
    insurance_fund: &mut InsuranceFund,
    amount: u64,
    insurance_balance: u64,
    stake: &mut Stake
) -> NormalResult {
    validate!(
        !(insurance_balance == 0 && insurance_fund.total_shares != 0),
        ErrorCode::InvalidIFForNewStakes,
        "Insurance Fund balance should be non-zero for new stakers to enter"
    )?;

    apply_rebase_to_insurance_fund(env, insurance_balance, insurance_fund)?;
    apply_rebase_to_stake(stake, insurance_fund)?;

    let if_shares_before = stake.checked_if_shares(insurance_fund)?;
    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    let n_shares = vault_amount_to_if_shares(
        amount,
        insurance_fund.total_shares,
        insurance_balance
    )?;

    // reset cost basis if no shares
    stake.cost_basis = if if_shares_before == 0 {
        amount.cast()?
    } else {
        stake.cost_basis.safe_add(amount.cast()?)?
    };

    stake.increase_if_shares(n_shares, insurance_fund)?;

    insurance_fund.total_shares = insurance_fund.total_shares.safe_add(n_shares)?;

    insurance_fund.user_shares = insurance_fund.user_shares.safe_add(n_shares)?;

    let if_shares_after = stake.checked_if_shares(insurance_fund)?;

    InsuranceFundEvents::stake(env, user, asset, amount);
    // emit!(InsuranceFundStakeRecord {
    // 	ts: now,
    // 	user_authority: user_stats.authority,
    // 	action: StakeAction::Stake,
    // 	amount,
    // 	insurance_balance_before: insurance_balance,
    // 	if_shares_before,
    // 	user_if_shares_before,
    // 	total_if_shares_before,
    // 	if_shares_after,
    // 	total_if_shares_after: insurance_fund.total_shares,
    // 	user_if_shares_after: insurance_fund.user_shares,
    // });

    Ok(())
}

pub fn apply_rebase_to_insurance_fund(
    env: &Env,
    insurance_fund_vault_balance: u64,
    insurance_fund: &mut InsuranceFund
) -> NormalResult {
    if
        insurance_fund_vault_balance != 0 &&
        insurance_fund_vault_balance.cast::<u128>()? < insurance_fund.total_shares
    {
        let (expo_diff, rebase_divisor) = calculate_rebase_info(
            insurance_fund.total_shares,
            insurance_fund_vault_balance
        )?;

        insurance_fund.total_shares = insurance_fund.total_shares.safe_div(rebase_divisor)?;
        insurance_fund.user_shares = insurance_fund.user_shares.safe_div(rebase_divisor)?;
        insurance_fund.shares_base = insurance_fund.shares_base.safe_add(
            expo_diff.cast::<u128>()?
        )?;

        log!(env, "rebasing insurance fund: expo_diff={}", expo_diff);
    }

    if insurance_fund_vault_balance != 0 && insurance_fund.total_shares == 0 {
        insurance_fund.total_shares = insurance_fund_vault_balance.cast::<u128>()?;
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

        let expo_diff = (insurance_fund.shares_base - stake.if_base).cast::<u32>()?;

        let rebase_divisor = (10_u128).pow(expo_diff);

        log!(
            env,
            "rebasing insurance fund stake: base: {} -> {} ",
            stake.if_base,
            insurance_fund.shares_base
        );

        stake.if_base = insurance_fund.shares_base;

        let old_if_shares = stake.unchecked_if_shares();
        let new_if_shares = old_if_shares.safe_div(rebase_divisor)?;

        log!(env, "rebasing insurance fund stake: shares -> {} ", new_if_shares);

        stake.update_if_shares(new_if_shares, spot_market)?;

        stake.last_withdraw_request_shares =
            stake.last_withdraw_request_shares.safe_div(rebase_divisor)?;
    }

    Ok(())
}

pub fn request_remove_stake(
    env: &Env,
    n_shares: u128,
    insurance_vault_amount: u64,
    stake: &mut InsuranceFundStake,
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
        stake.last_withdraw_request_shares <= stake.checked_if_shares(spot_market)?,
        ErrorCode::InvalidInsuranceUnstakeSize,
        "last_withdraw_request_shares exceeds if_shares {} > {}",
        stake.last_withdraw_request_shares,
        stake.checked_if_shares(spot_market)?
    )?;

    validate!(
        stake.if_base == insurance_fund.shares_base,
        ErrorCode::InvalidIFRebase,
        "if stake base != spot market base"
    )?;

    stake.last_withdraw_request_value = if_shares_to_vault_amount(
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

    let if_shares_after = stake.checked_if_shares(spot_market)?;

    // update_user_stats_if_stake_amount(0, insurance_vault_amount, stake, insurance_fund, now)?;

    InsuranceFundEvents::unstake(e, user, asset, amount);
    // emit!(InsuranceFundStakeRecord {
    //     ts: now,
    //     user_authority: user_stats.authority,
    //     action: StakeAction::UnstakeRequest,
    //     amount: stake.last_withdraw_request_value,
    //     insurance_vault_amount_before: insurance_vault_amount,
    //     if_shares_before,
    //     user_if_shares_before,
    //     total_if_shares_before,
    //     if_shares_after,
    //     total_if_shares_after: spot_market.insurance_fund.total_shares,
    //     user_if_shares_after: spot_market.insurance_fund.user_shares,
    // });

    stake.last_withdraw_request_ts = now;

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

    let if_shares_lost = calculate_if_shares_lost(stake, spot_market, insurance_vault_amount)?;

    stake.decrease_if_shares(if_shares_lost, spot_market)?;

    insurance_fund.total_shares = insurance_fund.total_shares.safe_sub(if_shares_lost)?;

    insurance_fund.user_shares = insurance_fund.user_shares.safe_sub(if_shares_lost)?;

    let if_shares_after = stake.checked_if_shares(spot_market)?;

    // update_user_stats_if_stake_amount(
    //     0,
    //     insurance_vault_amount,
    //     stake,
    //     user_stats,
    //     spot_market,
    //     now
    // )?;

    emit!(InsuranceFundStakeRecord {
        ts: now,
        user_authority: user_stats.authority,
        action: StakeAction::UnstakeCancelRequest,
        amount: 0,
        market_index: market_index,
        insurance_vault_amount_before: insurance_vault_amount,
        if_shares_before,
        user_if_shares_before,
        total_if_shares_before,
        if_shares_after,
        total_if_shares_after: insurance_fund.total_shares,
        user_if_shares_after: insurance_fund.user_shares,
    });

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
    let time_since_withdraw_request = now.safe_sub(stake.last_withdraw_request_ts)?;

    validate!(
        time_since_withdraw_request >= insurance_fund.unstaking_period,
        ErrorCode::TryingToRemoveLiquidityTooFast
    )?;

    apply_rebase_to_insurance_fund(insurance_vault_amount, spot_market)?;
    apply_rebase_to_stake(stake, spot_market)?;

    let if_shares_before = stake.checked_if_shares(insurance_fund)?;
    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    let n_shares = stake.last_withdraw_request_shares;

    validate!(
        n_shares > 0,
        ErrorCode::InvalidIFUnstake,
        "Must submit withdraw request and wait the escrow period"
    )?;

    validate!(if_shares_before >= n_shares, ErrorCode::InsufficientIFShares)?;

    let amount = if_shares_to_vault_amount(
        n_shares,
        insurance_fund.total_shares,
        insurance_vault_amount
    )?;

    let _if_shares_lost = calculate_if_shares_lost(stake, spot_market, insurance_vault_amount)?;

    let withdraw_amount = amount.min(stake.last_withdraw_request_value);

    stake.decrease_if_shares(n_shares, spot_market)?;

    stake.cost_basis = stake.cost_basis.safe_sub(withdraw_amount.cast()?)?;

    insurance_fund.total_shares = insurance_fund.total_shares.safe_sub(n_shares)?;

    insurance_fund.user_shares = insurance_fund.user_shares.safe_sub(n_shares)?;

    // reset stake withdraw request info
    stake.last_withdraw_request_shares = 0;
    stake.last_withdraw_request_value = 0;
    stake.last_withdraw_request_ts = now;

    let if_shares_after = stake.checked_if_shares(insurance_fund)?;

    // emit!(InsuranceFundStakeRecord {
    //     ts: now,
    //     user_authority: user_stats.authority,
    //     action: StakeAction::Unstake,
    //     amount: withdraw_amount,
    //     market_index: spot_market.market_index,
    //     insurance_vault_amount_before: insurance_vault_amount,
    //     if_shares_before,
    //     user_if_shares_before,
    //     total_if_shares_before,
    //     if_shares_after,
    //     total_if_shares_after: spot_market.insurance_fund.total_shares,
    //     user_if_shares_after: spot_market.insurance_fund.user_shares,
    // });

    Ok(withdraw_amount)
}

pub fn admin_remove_stake(
    env: &Env,
    insurance_vault_amount: u64,
    insurance_fund: &mut InsuranceFund,
    n_shares: u128,
    now: u64
) -> NormalResult<u64> {
    apply_rebase_to_insurance_fund(env, insurance_vault_amount, insurance_fund)?;

    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    let if_shares_before = total_if_shares_before.safe_sub(user_if_shares_before)?;

    validate!(
        if_shares_before >= n_shares,
        ErrorCode::InsufficientIFShares,
        "if_shares_before={} < n_shares={}",
        if_shares_before,
        n_shares
    )?;

    let withdraw_amount = if_shares_to_vault_amount(
        n_shares,
        insurance_fund.total_shares,
        insurance_vault_amount
    )?;

    insurance_fund.total_shares = insurance_fund.total_shares.safe_sub(n_shares)?;

    let if_shares_after = insurance_fund.total_shares.safe_sub(user_if_shares_before)?;

    InsuranceFundEvents::admin_unstake(env, user, asset, amount);
    // emit!(InsuranceFundStakeRecord {
    //     ts: now,
    //     user_authority: admin_pubkey,
    //     action: StakeAction::Unstake,
    //     amount: withdraw_amount,
    //     market_index: spot_market.market_index,
    //     insurance_vault_amount_before: insurance_vault_amount,
    //     if_shares_before,
    //     user_if_shares_before,
    //     total_if_shares_before,
    //     if_shares_after,
    //     total_if_shares_after: spot_market.insurance_fund.total_shares,
    //     user_if_shares_after: spot_market.insurance_fund.user_shares,
    // });

    Ok(withdraw_amount)
}

pub fn transfer_protocol_stake(
    env: &Env,
    insurance_vault_amount: u64,
    n_shares: u128,
    target_stake: &mut InsuranceFundStake,
    insurance_fund: &mut InsuranceFund,
    now: u64,
    signer_pubkey: Pubkey
) -> NormalResult<u64> {
    apply_rebase_to_insurance_fund(env, insurance_vault_amount, insurance_fund)?;

    let total_if_shares_before = insurance_fund.total_shares;
    let user_if_shares_before = insurance_fund.user_shares;

    let if_shares_before = total_if_shares_before.safe_sub(user_if_shares_before)?;
    let target_if_shares_before = target_stake.checked_if_shares(insurance_fund)?;
    validate!(
        if_shares_before >= n_shares,
        ErrorCode::InsufficientIFShares,
        "if_shares_before={} < n_shares={}",
        if_shares_before,
        n_shares
    )?;

    insurance_fund.user_shares = insurance_fund.user_shares.safe_add(n_shares)?;

    target_stake.increase_if_shares(n_shares, insurance_fund)?;

    let target_if_shares_after = target_stake.checked_if_shares(insurance_fund)?;


    let withdraw_amount = if_shares_to_vault_amount(
        n_shares,
        insurance_fund.total_shares,
        insurance_vault_amount
    )?;
    let user_if_shares_after = insurance_fund.user_shares;

    let protocol_if_shares_after = insurance_fund.total_shares.safe_sub(user_if_shares_after)?;

    InsuranceFundEvents::transfer_stake(e, user, asset, amount);

    // emit!(InsuranceFundStakeRecord {
    //     ts: now,
    //     user_authority: signer_pubkey,
    //     action: StakeAction::UnstakeTransfer,
    //     amount: withdraw_amount,
    //     market_index: spot_market.market_index,
    //     insurance_vault_amount_before: insurance_vault_amount,
    //     if_shares_before,
    //     user_if_shares_before,
    //     total_if_shares_before,
    //     if_shares_after: protocol_if_shares_after,
    //     total_if_shares_after: spot_market.insurance_fund.total_shares,
    //     user_if_shares_after: spot_market.insurance_fund.user_shares,
    // });

    // emit!(InsuranceFundStakeRecord {
    //     ts: now,
    //     user_authority: target_stake.authority,
    //     action: StakeAction::StakeTransfer,
    //     amount: withdraw_amount,
    //     market_index: spot_market.market_index,
    //     insurance_vault_amount_before: insurance_vault_amount,
    //     if_shares_before: target_if_shares_before,
    //     user_if_shares_before,
    //     total_if_shares_before,
    //     if_shares_after: target_stake.checked_if_shares(spot_market)?,
    //     total_if_shares_after: spot_market.insurance_fund.total_shares,
    //     user_if_shares_after: spot_market.insurance_fund.user_shares,
    // });

    Ok(withdraw_amount)
}
