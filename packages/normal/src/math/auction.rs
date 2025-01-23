use core::cmp::min;

use soroban_sdk::Env;

use crate::{
    constants::AUCTION_DERIVE_PRICE_FRACTION,
    error::NormalResult,
    math_error,
    oracle::OraclePriceData,
    types::{ Auction, OrderDirection },
};

use super::{ casting::Cast, safe_math::SafeMath };

pub fn calculate_auction_prices(
    env: &Env,
    oracle_price_data: &OraclePriceData,
    direction: OrderDirection,
    limit_price: u64
) -> NormalResult<(i64, i64)> {
    let oracle_price = oracle_price_data.price;
    let limit_price = limit_price.cast::<i64>(env)?;
    if limit_price > 0 {
        let (auction_start_price, auction_end_price) = match direction {
            // Long and limit price is better than oracle price
            OrderDirection::Buy if limit_price < oracle_price => {
                let limit_derive_start_price = limit_price.safe_sub(
                    limit_price / AUCTION_DERIVE_PRICE_FRACTION,
                    env
                )?;
                let oracle_derive_start_price = oracle_price.safe_sub(
                    oracle_price / AUCTION_DERIVE_PRICE_FRACTION,
                    env
                )?;

                (limit_derive_start_price.min(oracle_derive_start_price), limit_price)
            }
            // Long and limit price is worse than oracle price
            OrderDirection::Buy if limit_price >= oracle_price => {
                let oracle_derive_end_price = oracle_price.safe_add(
                    oracle_price / AUCTION_DERIVE_PRICE_FRACTION,
                    env
                )?;

                (oracle_price, limit_price.min(oracle_derive_end_price))
            }
            // Short and limit price is better than oracle price
            OrderDirection::Sell if limit_price > oracle_price => {
                let limit_derive_start_price = limit_price.safe_add(
                    limit_price / AUCTION_DERIVE_PRICE_FRACTION,
                    env
                )?;
                let oracle_derive_start_price = oracle_price.safe_add(
                    oracle_price / AUCTION_DERIVE_PRICE_FRACTION,
                    env
                )?;

                (limit_derive_start_price.max(oracle_derive_start_price), limit_price)
            }
            // Short and limit price is worse than oracle price
            OrderDirection::Sell if limit_price <= oracle_price => {
                let oracle_derive_end_price = oracle_price.safe_sub(
                    oracle_price / AUCTION_DERIVE_PRICE_FRACTION,
                    env
                )?;

                (oracle_price, limit_price.max(oracle_derive_end_price))
            }
            _ => unreachable!(),
        };

        return Ok((auction_start_price, auction_end_price));
    }

    let auction_end_price = match direction {
        OrderDirection::Buy => {
            oracle_price.safe_add(oracle_price / AUCTION_DERIVE_PRICE_FRACTION, env)?
        }
        OrderDirection::Sell => {
            oracle_price.safe_sub(oracle_price / AUCTION_DERIVE_PRICE_FRACTION, env)?
        }
    };

    Ok((oracle_price, auction_end_price))
}

pub fn calculate_auction_price(
    env: &Env,
    auction: &Auction,
    now: u64,
    tick_size: u64
) -> NormalResult<u64> {
    let slots_elapsed = now.safe_sub(auction.start_ts, env)?;

    let delta_numerator = min(slots_elapsed, auction.duration);
    let delta_denominator = auction.duration;

    // let auction_start_price = auction.start_price.cast::<u64>(env)?;
    // let auction_end_price = auction.end_price.cast::<u64>(env)?;

    if delta_denominator == 0 {
        return standardize_price(env, auction.end_price, tick_size, auction.direction);
    }

    let price_delta = match auction.direction {
        OrderDirection::Buy =>
            auction.end_price
                .safe_sub(auction.start_price, env)?
                .safe_mul(delta_numerator, env)?
                .safe_div(delta_denominator, env)?,
        OrderDirection::Sell =>
            auction.start_price
                .safe_sub(auction.end_price, env)?
                .safe_mul(delta_numerator, env)?
                .safe_div(delta_denominator, env)?,
    };

    let price = match auction.direction {
        OrderDirection::Buy => auction.start_price.safe_add(price_delta, env)?,
        OrderDirection::Sell => auction.start_price.safe_sub(price_delta, env)?,
    };

    standardize_price(env, price, tick_size, auction.direction)
}

pub fn is_auction_complete(
    env: &Env,
    order_slot: u64,
    auction_duration: u64,
    now: u64
) -> NormalResult<bool> {
    if auction_duration == 0 {
        return Ok(true);
    }

    let slots_elapsed = now.safe_sub(order_slot, env)?;

    Ok(slots_elapsed > auction_duration)
}

// From orders.math

pub fn standardize_price(
    env: &Env,
    price: u64,
    tick_size: u64,
    direction: OrderDirection
) -> NormalResult<u64> {
    if price == 0 {
        return Ok(0);
    }

    let remainder = price.checked_rem_euclid(tick_size).ok_or_else(math_error!())?;

    if remainder == 0 {
        return Ok(price);
    }

    match direction {
        OrderDirection::Buy => price.safe_sub(remainder, env),
        OrderDirection::Sell => price.safe_add(tick_size, env)?.safe_sub(remainder, env),
    }
}
