use std::convert::TryInto;

use crate::errors::ErrorCode;
use crate::math::*;

pub const NO_EXPLICIT_SQRT_PRICE_LIMIT: u128 = 0u128;

#[derive(PartialEq, Debug)]
pub struct SwapStepComputation {
	pub amount_in: u64,
	pub amount_out: u64,
	pub next_price: u128,
	pub fee_amount: u64,
}

pub fn compute_swap(
	amount_remaining: u64,
	fee_rate: u16,
	liquidity: u128,
	sqrt_price_current: u128,
	sqrt_price_target: u128,
	amount_specified_is_input: bool,
	synthetic_to_quote: bool
) -> Result<SwapStepComputation, ErrorCode> {
	// Since SplashPool (aka FullRange only pool) has only 2 initialized ticks at both ends,
	// the possibility of exceeding u64 when calculating "delta amount" is higher than concentrated pools.
	// This problem occurs with ExactIn.
	// The reason is that in ExactOut, "fixed delta" never exceeds the amount of tokens present in the pool and is clearly within the u64 range.
	// On the other hand, for ExactIn, "fixed delta" may exceed u64 because it calculates the amount of tokens needed to move the price to the end.
	// However, the primary purpose of initial calculation of "fixed delta" is to determine whether or not the iteration is "max swap" or not.
	// So the info that “the amount of tokens required exceeds the u64 range” is sufficient to determine that the iteration is NOT "max swap".
	//
	// delta <= u64::MAX: AmountDeltaU64::Valid
	// delta >  u64::MAX: AmountDeltaU64::ExceedsMax
	let initial_amount_fixed_delta = try_get_amount_fixed_delta(
		sqrt_price_current,
		sqrt_price_target,
		liquidity,
		amount_specified_is_input,
		synthetic_to_quote
	)?;

	let mut amount_calc = amount_remaining;
	if amount_specified_is_input {
		amount_calc = checked_mul_div(
			amount_remaining as u128,
			FEE_RATE_MUL_VALUE - (fee_rate as u128),
			FEE_RATE_MUL_VALUE
		)?.try_into()?;
	}

	let next_sqrt_price = if initial_amount_fixed_delta.lte(amount_calc) {
		sqrt_price_target
	} else {
		get_next_sqrt_price(
			sqrt_price_current,
			liquidity,
			amount_calc,
			amount_specified_is_input,
			synthetic_to_quote
		)?
	};

	let is_max_swap = next_sqrt_price == sqrt_price_target;

	let amount_unfixed_delta = get_amount_unfixed_delta(
		sqrt_price_current,
		next_sqrt_price,
		liquidity,
		amount_specified_is_input,
		synthetic_to_quote
	)?;

	// If the swap is not at the max, we need to readjust the amount of the fixed token we are using
	let amount_fixed_delta = if
		!is_max_swap ||
		initial_amount_fixed_delta.exceeds_max()
	{
		// next_sqrt_price is calculated by get_next_sqrt_price and the result will be in the u64 range.
		get_amount_fixed_delta(
			sqrt_price_current,
			next_sqrt_price,
			liquidity,
			amount_specified_is_input,
			synthetic_to_quote
		)?
	} else {
		// the result will be in the u64 range.
		initial_amount_fixed_delta.value()
	};

	let (amount_in, mut amount_out) = if amount_specified_is_input {
		(amount_fixed_delta, amount_unfixed_delta)
	} else {
		(amount_unfixed_delta, amount_fixed_delta)
	};

	// Cap output amount if using output
	if !amount_specified_is_input && amount_out > amount_remaining {
		amount_out = amount_remaining;
	}

	let fee_amount = if amount_specified_is_input && !is_max_swap {
		amount_remaining - amount_in
	} else {
		checked_mul_div_round_up(
			amount_in as u128,
			fee_rate as u128,
			FEE_RATE_MUL_VALUE - (fee_rate as u128)
		)?.try_into()?
	};

	Ok(SwapStepComputation {
		amount_in,
		amount_out,
		next_price: next_sqrt_price,
		fee_amount,
	})
}

fn get_amount_fixed_delta(
	sqrt_price_current: u128,
	sqrt_price_target: u128,
	liquidity: u128,
	amount_specified_is_input: bool,
	synthetic_to_quote: bool
) -> Result<u64, ErrorCode> {
	if synthetic_to_quote == amount_specified_is_input {
		get_amount_delta_synthetic(
			sqrt_price_current,
			sqrt_price_target,
			liquidity,
			amount_specified_is_input
		)
	} else {
		get_amount_delta_quote(
			sqrt_price_current,
			sqrt_price_target,
			liquidity,
			amount_specified_is_input
		)
	}
}

fn try_get_amount_fixed_delta(
	sqrt_price_current: u128,
	sqrt_price_target: u128,
	liquidity: u128,
	amount_specified_is_input: bool,
	synthetic_to_quote: bool
) -> Result<AmountDeltaU64, ErrorCode> {
	if synthetic_to_quote == amount_specified_is_input {
		try_get_amount_delta_synthetic(
			sqrt_price_current,
			sqrt_price_target,
			liquidity,
			amount_specified_is_input
		)
	} else {
		try_get_amount_delta_quote(
			sqrt_price_current,
			sqrt_price_target,
			liquidity,
			amount_specified_is_input
		)
	}
}

fn get_amount_unfixed_delta(
	sqrt_price_current: u128,
	sqrt_price_target: u128,
	liquidity: u128,
	amount_specified_is_input: bool,
	synthetic_to_quote: bool
) -> Result<u64, ErrorCode> {
	if synthetic_to_quote == amount_specified_is_input {
		get_amount_delta_quote(
			sqrt_price_current,
			sqrt_price_target,
			liquidity,
			!amount_specified_is_input
		)
	} else {
		get_amount_delta_synthetic(
			sqrt_price_current,
			sqrt_price_target,
			liquidity,
			!amount_specified_is_input
		)
	}
}

#[cfg(test)]
mod fuzz_tests {
	use super::*;
	use proptest::prelude::*;

	proptest! {
        #[test]
        fn test_compute_swap(
            amount in 1..u64::MAX,
            liquidity in 1..u32::MAX as u128,
            fee_rate in 1..u16::MAX,
            price_0 in MIN_SQRT_PRICE_X64..MAX_SQRT_PRICE_X64,
            price_1 in MIN_SQRT_PRICE_X64..MAX_SQRT_PRICE_X64,
            amount_specified_is_input in proptest::bool::ANY,
        ) {
            prop_assume!(price_0 != price_1);

            // Rather than use logic to correctly input the prices, we just use the distribution to determine direction
            let synthetic_to_quote = price_0 >= price_1;

            let swap_computation = compute_swap(
                amount,
                fee_rate,
                liquidity,
                price_0,
                price_1,
                amount_specified_is_input,
                synthetic_to_quote,
            ).ok().unwrap();

            let amount_in = swap_computation.amount_in;
            let amount_out = swap_computation.amount_out;
            let next_price = swap_computation.next_price;
            let fee_amount = swap_computation.fee_amount;

            // Amount_in can not exceed maximum amount
            assert!(amount_in <= u64::MAX - fee_amount);

            // Amounts calculated are less than amount specified
            let amount_used = if amount_specified_is_input {
                amount_in + fee_amount
            } else {
                amount_out
            };

            if next_price != price_1 {
                assert!(amount_used == amount);
            } else {
                assert!(amount_used <= amount);
            }

            let (price_lower, price_upper) = increasing_price_order(price_0, price_1);
            assert!(next_price >= price_lower);
            assert!(next_price <= price_upper);
        }

        #[test]
        fn test_compute_swap_inversion(
            amount in 1..u64::MAX,
            liquidity in 1..u32::MAX as u128,
            fee_rate in 1..u16::MAX,
            price_0 in MIN_SQRT_PRICE_X64..MAX_SQRT_PRICE_X64,
            price_1 in MIN_SQRT_PRICE_X64..MAX_SQRT_PRICE_X64,
            amount_specified_is_input in proptest::bool::ANY,
        ) {
            prop_assume!(price_0 != price_1);

            // Rather than use logic to correctly input the prices, we just use the distribution to determine direction
            let synthetic_to_quote = price_0 >= price_1;

            let swap_computation = compute_swap(
                amount,
                fee_rate,
                liquidity,
                price_0,
                price_1,
                amount_specified_is_input,
                synthetic_to_quote,
            ).ok().unwrap();

            let amount_in = swap_computation.amount_in;
            let amount_out = swap_computation.amount_out;
            let next_price = swap_computation.next_price;
            let fee_amount = swap_computation.fee_amount;

            let inverted_amount = if amount_specified_is_input {
                amount_out
            } else {
                amount_in + fee_amount
            };

            if inverted_amount != 0 {
                let inverted = compute_swap(
                    inverted_amount,
                    fee_rate,
                    liquidity,
                    price_0,
                    price_1,
                    !amount_specified_is_input,
                    synthetic_to_quote,
                ).ok().unwrap();

                // A to B = price decreasing

                // Case 1
                // Normal: is_input, synthetic_to_quote
                // Input is fixed, consume all input to produce amount_out
                // amount_in = fixed, ceil
                // amount_out = unfixed, floor

                // Inverted: !is_input, synthetic_to_quote
                // amount_in = unfixed, ceil
                // amount_out = fixed, floor
                // Amount = amount_out, inverted.amount_in and fee <= original input and fee, inverted.amount_out ~~ amount_out, inverted.next_price >= original.next_price


                // Case 2
                // Normal: !is_input, synthetic_to_quote
                // Find amount required to get amount_out
                // amount_in = unfixed, ceil
                // amount_out = fixed, floor

                // Inverted: is_input, synthetic_to_quote
                // amount_in = fixed, ceil
                // amount_out = unfixed, floor
                // Get max amount_out for input, inverted.amount_in + fee ~~ original input and fee, inverted.amount_out >= amount_out, inverted.next_price <= original.next_price


                // Price increasing
                // Case 3
                // Normal: is_input, !synthetic_to_quote
                // Input is fixed, consume all input to produce amount_out
                // amount_in = fixed, ceil
                // amount_out = unfixed, floor

                // Inverted: !is_input, !synthetic_to_quote
                // Amount = amount_out, inverted.amount_in and fee <= original input and fee, inverted.amount_out ~~ amount_out, inverted.next_price <= original.next_price

                // Case 4
                // Normal: !is_input, !synthetic_to_quote
                // Find amount required to get amount_out
                // amount_in = fixed, floor
                // amount_out = unfixed, ceil
                // Inverted: is_input, !synthetic_to_quote
                // Get max amount_out for input, inverted.amount_in + fee ~~ original input and fee, inverted.amount_out >= amount_out
                // Since inverted.amount_out >= amount_out and amount in is the same, more of token a is being removed, so
                // inverted.next_price >= original.next_price

                // Next sqrt price goes from round up to round down
                // assert!(inverted.next_price + 1 >= next_price);

                if inverted.next_price != price_1 {
                    if amount_specified_is_input {
                        // If synthetic_to_quote, then goes round up => round down,
                        assert!(inverted.amount_in <= amount_in);
                        assert!(inverted.fee_amount <= fee_amount);
                    } else {
                        assert!(inverted.amount_in >= amount_in);
                        assert!(inverted.fee_amount >= fee_amount);
                    }
                    assert!(inverted.amount_out >= amount_out);
                    if synthetic_to_quote == amount_specified_is_input {
                        // Next sqrt price goes from round up to round down
                        assert!(inverted.next_price >= next_price);
                    } else {
                        // Next sqrt price goes from round down to round up
                        assert!(inverted.next_price <= next_price);
                    }

                    // Ratio calculations
                    // let ratio_in = (u128::from(inverted.amount_in) << 64) / u128::from(amount_in);
                    // let ratio_out = (u128::from(inverted.amount_out) << 64) / u128::from(amount_out);
                    // println!("RATIO IN/OUT WHEN INVERTED {} \t| {} ", ratio_in, ratio_out);

                    // if ratio_out > (2 << 64) || ratio_in < (1 << 63) {
                    //     if ratio_out > (2 << 64) {
                    //         println!("OUT > {}", ratio_out / (1 << 64));
                    //     }
                    //     if ratio_in < (1 << 63) {
                    //         println!("IN < 1/{}", (1 << 64) / ratio_in);
                    //     }

                    //     println!("liq {} | fee {} | price_0 {} | price_1 {} | synthetic_to_quote {}", liquidity, fee_rate, price_0, price_1, synthetic_to_quote);
                    //     println!("Amount {} | is_input {}", amount, amount_specified_is_input);
                    //     println!("Inverted Amount {} | is_input {}", inverted_amount, !amount_specified_is_input);
                    //     println!("{:?}", swap_computation);
                    //     println!("{:?}", inverted);
                    // }
                }
            }
        }
    }
}
