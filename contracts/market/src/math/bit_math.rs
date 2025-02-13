use soroban_sdk::{panic_with_error, Env};

use crate::errors::Errors;

// use super::U256Muldiv;

pub const Q64_RESOLUTION: u8 = 64;
pub const Q64_MASK: u128 = 0xffff_ffff_ffff_ffff;
pub const TO_Q64: u128 = 1u128 << Q64_RESOLUTION;

pub fn checked_mul_div(env: &Env, n0: u128, n1: u128, d: u128) -> u128 {
    checked_mul_div_round_up_if(env, n0, n1, d, false)
}

pub fn checked_mul_div_round_up(env: &Env, n0: u128, n1: u128, d: u128) -> u128 {
    checked_mul_div_round_up_if(env, n0, n1, d, true)
}

pub fn checked_mul_div_round_up_if(env: &Env, n0: u128, n1: u128, d: u128, round_up: bool) -> u128 {
    if d == 0 {
        panic_with_error!(env, Errors::DivideByZero);
    }

    let p = n0.checked_mul(n1).ok_or(ErrorCode::MulDivOverflow)?;
    let n = p / d;

    if round_up && p % d > 0 {
        n + 1
    } else {
        n
    }
}

pub fn checked_mul_shift_right(env: &Env, n0: u128, n1: u128) -> u64 {
    checked_mul_shift_right_round_up_if(env, n0, n1, false)
}

/// Multiplies an integer u128 and a Q64.64 fixed point number.
/// Returns a product represented as a u64 integer.
pub fn checked_mul_shift_right_round_up_if(env: &Env, n0: u128, n1: u128, round_up: bool) -> u64 {
    // customized this function is used in try_get_amount_delta_b (token_math.rs)

    if n0 == 0 || n1 == 0 {
        return Ok(0);
    }

    let p = n0.checked_mul(n1).ok_or(panic_with_error!(
        env,
        Errors::MultiplicationShiftRightOverflow
    ))?;

    let result = (p >> Q64_RESOLUTION) as u64;

    let should_round = round_up && p & Q64_MASK > 0;
    if should_round && result == u64::MAX {
        panic_with_error!(env, Errors::MultiplicationOverflow);
    }

    if should_round {
        result + 1
    } else {
        result
    }
}

pub fn div_round_up(env: &Env, n: u128, d: u128) -> u128 {
    div_round_up_if(env, n, d, true)
}

pub fn div_round_up_if(env: &Env, n: u128, d: u128, round_up: bool) -> u128 {
    if d == 0 {
        panic_with_error!(env, Errors::DivideByZero);
    }

    let q = n / d;

    if round_up && n % d > 0 {
        q + 1
    } else {
        q
    }
}

pub fn div_round_up_if_u256(n: U256Muldiv, d: U256Muldiv, round_up: bool) -> u128 {
    let (quotient, remainder) = n.div(d, round_up);

    let result = if round_up && !remainder.is_zero() {
        quotient.add(U256Muldiv::new(0, 1))
    } else {
        quotient
    };

    result.try_into_u128()
}
