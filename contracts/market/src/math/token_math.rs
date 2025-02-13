use normal::error::{ErrorCode, NormalResult};
use soroban_decimal::{Decimal, Decimal256};
use soroban_sdk::{panic_with_error, Env};

use crate::errors::{ErrorCode, Errors};

use super::{
    bit_math::{div_round_up_if, div_round_up_if_u256, Q64_MASK, Q64_RESOLUTION},
    tick_math::{MAX_SQRT_PRICE_X64, MIN_SQRT_PRICE_X64},
    // u256_math::{ mul_u256, U256Muldiv },
};

// Fee rate is represented as hundredths of a basis point.
// Fee amount = total_amount * fee_rate / 1_000_000.
// Max fee rate supported is 3%.
pub const MAX_FEE_RATE: u16 = 30_000;

// Assuming that FEE_RATE is represented as hundredths of a basis point
// We want FEE_RATE_MUL_VALUE = 1/FEE_RATE_UNIT, so 1e6
pub const FEE_RATE_MUL_VALUE: u128 = 1_000_000;

// Protocol fee rate is represented as a basis point.
// Protocol fee amount = fee_amount * protocol_fee_rate / 10_000.
// Max protocol fee rate supported is 25% of the fee rate.
pub const MAX_PROTOCOL_FEE_RATE: u16 = 2_500;

// Assuming that PROTOCOL_FEE_RATE is represented as a basis point
// We want PROTOCOL_FEE_RATE_MUL_VALUE = 1/PROTOCOL_FEE_UNIT, so 1e4
pub const PROTOCOL_FEE_RATE_MUL_VALUE: u128 = 10_000;

#[derive(Debug)]
pub enum AmountDeltaI128 {
    Valid(i128),
    ExceedsMax(ErrorCode),
}

impl AmountDeltaI128 {
    pub fn lte(&self, other: i128) -> bool {
        match self {
            AmountDeltaI128::Valid(value) => *value <= other,
            AmountDeltaI128::ExceedsMax(_) => false,
        }
    }

    pub fn exceeds_max(&self) -> bool {
        match self {
            AmountDeltaI128::Valid(_) => false,
            AmountDeltaI128::ExceedsMax(_) => true,
        }
    }

    pub fn value(self) -> i128 {
        match self {
            AmountDeltaI128::Valid(value) => value,
            // This should never happen
            AmountDeltaI128::ExceedsMax(_) => {
                panic!("Called unwrap on AmountDeltaI128::ExceedsMax")
            }
        }
    }
}

//
// Get change in token_synthetic corresponding to a change in price
//

// 6.16
// Δt_a = Δ(1 / sqrt_price) * liquidity

// Replace delta
// Δt_a = (1 / sqrt_price_upper - 1 / sqrt_price_lower) * liquidity

// Common denominator to simplify
// Δt_a = ((sqrt_price_lower - sqrt_price_upper) / (sqrt_price_upper * sqrt_price_lower)) * liquidity

// Δt_a = (liquidity * (sqrt_price_lower - sqrt_price_upper)) / (sqrt_price_upper * sqrt_price_lower)
pub fn get_amount_delta_a(
    env: &Env,
    sqrt_price_0: u128,
    sqrt_price_1: u128,
    liquidity: u128,
    round_up: bool,
) -> NormalResult<i128> {
    match try_get_amount_delta_a(sqrt_price_0, sqrt_price_1, liquidity, round_up) {
        Ok(AmountDeltaI128::Valid(value)) => Ok(value),
        Ok(AmountDeltaI128::ExceedsMax(error)) => Err(error),
        Err(error) => Err(error),
    }
}

pub fn try_get_amount_delta_a(
    env: &Env,
    sqrt_price_0: u128,
    sqrt_price_1: u128,
    liquidity: u128,
    round_up: bool,
) -> NormalResult<AmountDeltaI128> {
    let (sqrt_price_lower, sqrt_price_upper) = increasing_price_order(sqrt_price_0, sqrt_price_1);

    let sqrt_price_diff = Decimal256::new(env, sqrt_price_upper - sqrt_price_lower);

    // let numerator = mul_u256(liquidity, sqrt_price_diff)
    //     .checked_shift_word_left()
    //     .ok_or(ErrorCode::MultiplicationOverflow)?;

    let numerator = sqrt_price_diff.mul(env, liquidity);
    // .checked_shift_word_left()
    // .ok_or(ErrorCode::MultiplicationOverflow)?;

    // let denominator = mul_u256(sqrt_price_upper, sqrt_price_lower);
    let denominator = Decimal256::new(env, sqrt_price_upper).mul(env, sqrt_price_lower);

    let (quotient, remainder) = numerator.div(env, denominator); // round_up

    let result = if round_up && !remainder.is_zero() {
        quotient.add(U256Muldiv::new(0, 1)).try_into_u128()
    } else {
        quotient.try_into_u128()
    };

    match result {
        Ok(result) => {
            if result > (u64::MAX as u128) {
                return Ok(AmountDeltaI128::ExceedsMax(ErrorCode::TokenMaxExceeded));
            }

            Ok(AmountDeltaI128::Valid(result as u64))
        }
        Err(err) => Ok(AmountDeltaI128::ExceedsMax(err)),
    }
}

//
// Get change in token_quote corresponding to a change in price
//

// 6.14
// Δt_b = Δ(sqrt_price) * liquidity

// Replace delta
// Δt_b = (sqrt_price_upper - sqrt_price_lower) * liquidity
pub fn get_amount_delta_b(
    env: &Env,
    sqrt_price_0: u128,
    sqrt_price_1: u128,
    liquidity: u128,
    round_up: bool,
) -> i128 {
    match try_get_amount_delta_b(sqrt_price_0, sqrt_price_1, liquidity, round_up) {
        AmountDeltaI128::Valid(value) => value,
        AmountDeltaI128::ExceedsMax(error) => Err(error),
        Err(error) => Err(error),
    }
}

pub fn try_get_amount_delta_b(
    env: &Env,
    sqrt_price_0: u128,
    sqrt_price_1: u128,
    liquidity: u128,
    round_up: bool,
) -> AmountDeltaI128 {
    let (sqrt_price_lower, sqrt_price_upper) = increasing_price_order(sqrt_price_0, sqrt_price_1);

    // customized checked_mul_shift_right_round_up_if

    let n0 = liquidity;
    let n1 = sqrt_price_upper - sqrt_price_lower;

    if n0 == 0 || n1 == 0 {
        return AmountDeltaI128::Valid(0);
    }

    if let Some(p) = n0.checked_mul(n1) {
        let result = (p >> Q64_RESOLUTION) as u64;

        let should_round = round_up && p & Q64_MASK > 0;
        if should_round && result == u64::MAX {
            return AmountDeltaI128::ExceedsMax(ErrorCode::MultiplicationOverflow);
        }

        AmountDeltaI128::Valid(if should_round { result + 1 } else { result })
    } else {
        AmountDeltaI128::ExceedsMax(ErrorCode::MultiplicationShiftRightOverflow)
    }
}

pub fn increasing_price_order(sqrt_price_0: u128, sqrt_price_1: u128) -> (u128, u128) {
    if sqrt_price_0 > sqrt_price_1 {
        (sqrt_price_1, sqrt_price_0)
    } else {
        (sqrt_price_0, sqrt_price_1)
    }
}

//
// Get change in price corresponding to a change in token_a supply
//
// 6.15
// Δ(1 / sqrt_price) = Δt_a / liquidity
//
// Replace delta
// 1 / sqrt_price_new - 1 / sqrt_price = amount / liquidity
//
// Move sqrt price to other side
// 1 / sqrt_price_new = (amount / liquidity) + (1 / sqrt_price)
//
// Common denominator for right side
// 1 / sqrt_price_new = (sqrt_price * amount + liquidity) / (sqrt_price * liquidity)
//
// Invert fractions
// sqrt_price_new = (sqrt_price * liquidity) / (liquidity + amount * sqrt_price)
pub fn get_next_sqrt_price_from_a_round_up(
    env: &Env,
    sqrt_price: u128,
    liquidity: u128,
    amount: u64,
    amount_specified_is_input: bool,
) -> u128 {
    if amount == 0 {
        return sqrt_price;
    }
    let product = Decimal256::new(env, sqrt_price).mul(env, &Decimal256::new(env, amount as u128));

    let numerator = Decimal256::new(env, liquidity).mul(env, &Decimal256::new(env, sqrt_price));

    // In this scenario the denominator will end up being < 0
    // let liquidity_shift_left = Decimal256::new(env, 0).shi
    let liquidity_shift_left = U256Muldiv::new(0, liquidity).shift_word_left();
    if !amount_specified_is_input && liquidity_shift_left.lte(product) {
        panic_with_error!(env, Errors::DivideByZero);
    }

    let denominator = if amount_specified_is_input {
        liquidity_shift_left.add(product)
    } else {
        liquidity_shift_left.sub(product)
    };

    let price = div_round_up_if_u256(numerator, denominator, true)?;
    if price < MIN_SQRT_PRICE_X64 {
        panic_with_error!(env, Errors::TokenMinSubceeded);
    } else if price > MAX_SQRT_PRICE_X64 {
        panic_with_error!(env, Errors::TokenMaxExceeded);
    }

    price
}
// pub fn get_next_sqrt_price_from_a_round_up(
//     sqrt_price: u128,
//     liquidity: u128,
//     amount: u64,
//     amount_specified_is_input: bool
// ) -> NormalResult<u128> {
//     if amount == 0 {
//         return Ok(sqrt_price);
//     }
//     let product = mul_u256(sqrt_price, amount as u128);

//     let numerator = mul_u256(liquidity, sqrt_price)
//         .checked_shift_word_left()
//         .ok_or(ErrorCode::MultiplicationOverflow)?;

//     // In this scenario the denominator will end up being < 0
//     let liquidity_shift_left = U256Muldiv::new(0, liquidity).shift_word_left();
//     if !amount_specified_is_input && liquidity_shift_left.lte(product) {
//         return Err(ErrorCode::DivideByZero);
//     }

//     let denominator = if amount_specified_is_input {
//         liquidity_shift_left.add(product)
//     } else {
//         liquidity_shift_left.sub(product)
//     };

//     let price = div_round_up_if_u256(numerator, denominator, true)?;
//     if price < MIN_SQRT_PRICE_X64 {
//         return Err(ErrorCode::TokenMinSubceeded);
//     } else if price > MAX_SQRT_PRICE_X64 {
//         return Err(ErrorCode::TokenMaxExceeded);
//     }

//     Ok(price)
// }

//
// Get change in price corresponding to a change in token_b supply
//
// 6.13
// Δ(sqrt_price) = Δt_b / liquidity
pub fn get_next_sqrt_price_from_b_round_down(
    env: &Env,
    sqrt_price: u128,
    liquidity: u128,
    amount: u64,
    amount_specified_is_input: bool,
) -> u128 {
    // We always want square root price to be rounded down, which means
    // Case 3. If we are fixing input (adding B), we are increasing price, we want delta to be floor(delta)
    // sqrt_price + floor(delta) < sqrt_price + delta
    //
    // Case 4. If we are fixing output (removing B), we are decreasing price, we want delta to be ceil(delta)
    // sqrt_price - ceil(delta) < sqrt_price - delta

    // Q64.0 << 64 => Q64.64
    let amount_x64 = (amount as u128) << Q64_RESOLUTION;

    // Q64.64 / Q64.0 => Q64.64
    let delta = div_round_up_if(env, amount_x64, liquidity, !amount_specified_is_input)?;

    // Q64(32).64 +/- Q64.64
    if amount_specified_is_input {
        // We are adding token b to supply, causing price to increase
        sqrt_price
            .checked_add(delta)
            .ok_or(panic_with_error!(env, Errors::SqrtPriceOutOfBounds))
    } else {
        // We are removing token b from supply,. causing price to decrease
        sqrt_price
            .checked_sub(delta)
            .ok_or(panic_with_error!(env, Errors::SqrtPriceOutOfBounds))
    }
}

pub fn get_next_sqrt_price(
    env: &Env,
    sqrt_price: u128,
    liquidity: u128,
    amount: u64,
    amount_specified_is_input: bool,
    a_to_b: bool,
) -> u128 {
    if amount_specified_is_input == a_to_b {
        // We are fixing A
        // Case 1. amount_specified_is_input = true, a_to_b = true
        // We are exchanging A to B with at most _amount_ of A (input)
        //
        // Case 2. amount_specified_is_input = false, a_to_b = false
        // We are exchanging B to A wanting to guarantee at least _amount_ of A (output)
        //
        // In either case we want the sqrt_price to be rounded up.
        //
        // Eq 1. sqrt_price = sqrt( b / a )
        //
        // Case 1. amount_specified_is_input = true, a_to_b = true
        // We are adding token A to the supply, causing price to decrease (Eq 1.)
        // Since we are fixing input, we can not exceed the amount that is being provided by the user.
        // Because a higher price is inversely correlated with an increased supply of A,
        // a higher price means we are adding less A. Thus when performing math, we wish to round the
        // price up, since that means that we are guaranteed to not exceed the fixed amount of A provided.
        //
        // Case 2. amount_specified_is_input = false, a_to_b = false
        // We are removing token A from the supply, causing price to increase (Eq 1.)
        // Since we are fixing output, we want to guarantee that the user is provided at least _amount_ of A
        // Because a higher price is correlated with a decreased supply of A,
        // a higher price means we are removing more A to give to the user. Thus when performing math, we wish
        // to round the price up, since that means we guarantee that user receives at least _amount_ of A
        get_next_sqrt_price_from_a_round_up(
            env,
            sqrt_price,
            liquidity,
            amount,
            amount_specified_is_input,
        )
    } else {
        // We are fixing B
        // Case 3. amount_specified_is_input = true, a_to_b = false
        // We are exchanging B to A using at most _amount_ of B (input)
        //
        // Case 4. amount_specified_is_input = false, a_to_b = true
        // We are exchanging A to B wanting to guarantee at least _amount_ of B (output)
        //
        // In either case we want the sqrt_price to be rounded down.
        //
        // Eq 1. sqrt_price = sqrt( b / a )
        //
        // Case 3. amount_specified_is_input = true, a_to_b = false
        // We are adding token B to the supply, causing price to increase (Eq 1.)
        // Since we are fixing input, we can not exceed the amount that is being provided by the user.
        // Because a lower price is inversely correlated with an increased supply of B,
        // a lower price means that we are adding less B. Thus when performing math, we wish to round the
        // price down, since that means that we are guaranteed to not exceed the fixed amount of B provided.
        //
        // Case 4. amount_specified_is_input = false, a_to_b = true
        // We are removing token B from the supply, causing price to decrease (Eq 1.)
        // Since we are fixing output, we want to guarantee that the user is provided at least _amount_ of B
        // Because a lower price is correlated with a decreased supply of B,
        // a lower price means we are removing more B to give to the user. Thus when performing math, we
        // wish to round the price down, since that means we guarantee that the user receives at least _amount_ of B
        get_next_sqrt_price_from_b_round_down(
            env,
            sqrt_price,
            liquidity,
            amount,
            amount_specified_is_input,
        )
    }
}
