use normal::{
    constants::SPOT_UTILIZATION_PRECISION,
    math::{casting::Cast, safe_math::SafeMath},
};
use soroban_sdk::{contracttype, Env};

use crate::state::market::Market;

#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BalanceType {
    #[default]
    Deposit,
    Borrow,
}

pub fn get_balance(
    env: &Env,
    token_amount: u128,
    market: &Market,
    balance_type: &BalanceType,
    round_up: bool,
) -> u128 {
    let precision_increase = (10_u128).pow((19_u32).safe_sub(market.decimals, env));

    // let cumulative_interest = match balance_type {
    //     BalanceType::Deposit => market.cumulative_deposit_interest,
    //     BalanceType::Borrow => market.cumulative_borrow_interest,
    // };

    let mut balance = token_amount
        .safe_mul(precision_increase, env)
        .safe_div(1, env); // cumulative_interest

    if round_up && balance != 0 {
        balance = balance.safe_add(1, env);
    }

    balance
}

pub fn get_token_amount(
    env: &Env,
    balance: u128,
    market: &Market,
    balance_type: &BalanceType,
) -> u128 {
    let precision_decrease = (10_u128).pow((19_u32).safe_sub(market.decimals, env));

    let token_amount = match balance_type {
        BalanceType::Deposit => balance.safe_mul(1, env).safe_div(precision_decrease, env),
        BalanceType::Borrow => balance
            .safe_mul(1, env)
            .safe_div_ceil(precision_decrease, env),
    };

    token_amount
}

pub fn get_signed_token_amount(env: &Env, token_amount: u128, balance_type: &BalanceType) -> i128 {
    match balance_type {
        BalanceType::Deposit => token_amount.cast(env),
        BalanceType::Borrow => token_amount
            .cast::<i128>(env)
            .map(|token_amount| -token_amount),
    }
}

pub fn calculate_utilization(
    env: &Env,
    collateral_token_amount: u128,
    debt_token_amount: u128,
    collateral_price: i64,
    debt_price: i64,
    precision: u128,
) -> u128 {
    let collateral_value = collateral_token_amount
        .cast::<u128>(env)
        .safe_mul(collateral_price.cast::<u128>(env), env)
        .safe_div(precision, env)
        .cast::<u64>(env);

    let debt_value = debt_token_amount
        .cast::<u128>(env)
        .safe_mul(debt_price.cast::<u128>(env), env)
        .safe_div(precision, env)
        .cast::<u64>(env);

    let utilization = debt_value
        .safe_mul(SPOT_UTILIZATION_PRECISION, env)
        .checked_div(collateral_value)
        .unwrap_or({
            if collateral_token_amount == 0 && debt_token_amount == 0 {
                0_u128
            } else {
                // if there are borrows without deposits, default to maximum utilization rate
                SPOT_UTILIZATION_PRECISION
            }
        });

    // let utilization = synthetic_token_amount
    //     .safe_mul(SPOT_UTILIZATION_PRECISION, env)
    //     .checked_div(collateral_token_amount)
    //     .unwrap_or({
    //         if collateral_token_amount == 0 && synthetic_token_amount == 0 {
    //             0_u128
    //         } else {
    //             // if there are borrows without deposits, default to maximum utilization rate
    //             SPOT_UTILIZATION_PRECISION
    //         }
    //     });

    utilization
}

pub fn calculate_market_utilization(env: &Env, market: &Market) -> u128 {
    let collateral_token_amount = get_token_amount(
        env,
        market.collateral.balance,
        market,
        &BalanceType::Deposit,
    );
    let synth_token_amount =
        get_token_amount(env, market.synthetic.balance, market, &BalanceType::Borrow);
    let utilization = calculate_utilization(env, collateral_token_amount, synth_token_amount);

    utilization
}
