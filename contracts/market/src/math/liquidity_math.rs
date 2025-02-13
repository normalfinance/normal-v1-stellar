use soroban_sdk::{panic_with_error, Env};

use crate::errors::PoolErrors;

// Adds a signed liquidity delta to a given integer liquidity amount.
// Errors on overflow or underflow.
pub fn add_liquidity_delta(env: &Env, liquidity: u128, delta: i128) -> u128 {
    if delta == 0 {
        return liquidity;
    }
    if delta > 0 {
        liquidity
            .checked_add(delta as u128)
            .ok_or(panic_with_error!(env, PoolErrors::LiquidityOverflow))
    } else {
        liquidity
            .checked_sub(delta.unsigned_abs())
            .ok_or(panic_with_error!(env, PoolErrors::LiquidityUnderflow))
    }
}

// Converts an unsigned liquidity amount to a signed liquidity delta
pub fn convert_to_liquidity_delta(env: &Env, liquidity_amount: u128, positive: bool) -> i128 {
    if liquidity_amount > (i128::MAX as u128) {
        // The liquidity_amount is converted to a liquidity_delta that is represented as an i128
        // By doing this conversion we lose the most significant bit in the u128
        // Here we enforce a max value of i128::MAX on the u128 to prevent loss of data.
        panic_with_error!(env, PoolErrors::LiquidityTooHigh);
    }
    if positive {
        liquidity_amount as i128
    } else {
        -(liquidity_amount as i128)
    }
}
