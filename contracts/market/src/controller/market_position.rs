use normal::{math::casting::Cast, safe_decrement, safe_increment, validate};
use soroban_sdk::Env;

use crate::{
    math::{balance::BalanceType, withdraw::check_withdraw_limits},
    state::{
        market::{Market, MarketOperation, MarketStatus},
        market_position::MarketPosition,
    },
};

use super::balance::update_balances;

pub fn update_balances_and_cumulative_deposits(
    env: &Env,
    token_amount: u128,
    update_direction: &BalanceType,
    market: &mut Market,
    position: &mut MarketPosition,
    is_leaving_normal: bool,
    cumulative_deposit_delta: Option<u128>,
) {
    update_balances(
        env,
        token_amount,
        update_direction,
        market,
        position,
        is_leaving_normal,
    );

    let cumulative_deposit_delta = cumulative_deposit_delta.unwrap_or(token_amount);
    match update_direction {
        BalanceType::Deposit => {
            safe_increment!(position.cumulative_deposits, cumulative_deposit_delta, env)
        }
        BalanceType::Borrow => {
            safe_decrement!(position.cumulative_deposits, cumulative_deposit_delta, env)
        }
    }
}

pub fn update_balances_and_cumulative_deposits_with_limits(
    env: &Env,
    token_amount: u128,
    update_direction: &BalanceType,
    market: &mut Market,
    position: &mut MarketPosition,
) {
    update_balances_and_cumulative_deposits(
        env,
        token_amount,
        update_direction,
        market,
        &mut position,
        true,
        None,
    );

    let valid_withdraw = check_withdraw_limits(env, market, position, Some(token_amount));

    validate!(
        env,
        valid_withdraw,
        Errors::DailyWithdrawLimit,
        "Market {} has hit daily withdraw limit. Attempted withdraw amount of {} by {}",
        market.name,
        token_amount,
        user.authority
    );

    validate!(
        env,
        matches!(
            market.status,
            MarketStatus::Active | MarketStatus::ReduceOnly | MarketStatus::Settlement
        ),
        Errors::MarketWithdrawPaused,
        "Market {} withdraws are currently paused, market not active or in settlement",
        market.name
    );

    validate!(
        env,
        !market.is_operation_paused(MarketOperation::Withdraw),
        Errors::MarketWithdrawPaused,
        "Market {} withdraws are currently paused",
        market.name
    );
}
