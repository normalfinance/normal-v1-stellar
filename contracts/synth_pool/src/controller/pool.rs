use crate::math::bit_math::checked_mul_div;
use crate::reward::RewardInfo;
use crate::storage::Pool;
use crate::{ math, token_contract };
use normal::error::{ ErrorCode, NormalResult };
use soroban_sdk::{ Env, Vec };

pub fn update_pool_price(env: &Env, pool: &Pool) -> NormalResult {
    // let protocol_position =
    let price_diff = pool.get_oracle_price_deviance(env);
    let liquidity_delta = pool.get_liquidity_delta_for_price_impact(price_diff)?;

    if liquidity_delta == 0 {
        return Ok(());
    }

    let delta_b = 100;
    // let delta_b = super::liquidity::calculate_collateral_liquidity_token_delta(
    //     pool.tick_current_index,
    //     pool.sqrt_price,
    //     &position,
    //     liquidity_delta
    // )?;

    if liquidity_delta > 0 {
        // pull collateral from market
        token_contract::Client
            ::new(&env, &pool.token_b)
            .transfer_from(
                &env.current_contract_address(),
                &pool.market,
                &env.current_contract_address(),
                delta_b
            );
    } else {
        //    push collateral to the market
        token_contract::Client
            ::new(&env, &pool.token_b)
            .transfer(&env.current_contract_address(), &pool.market, &delta_b);
    }

    // update the pool and protocol position
    // position.up

    Ok(())
}

// Calculates the next global reward growth variables based on the given timestamp.
// The provided timestamp must be greater than or equal to the last updated timestamp.
pub fn next_amm_reward_infos(
    pool: &Pool,
    next_timestamp: u64
) -> Result<Vec<RewardInfo>, ErrorCode> {
    let curr_timestamp = pool.reward_last_updated_timestamp;
    if next_timestamp < curr_timestamp {
        return Err(ErrorCode::InvalidTimestamp);
    }

    // No-op if no liquidity or no change in timestamp
    if pool.liquidity == 0 || next_timestamp == curr_timestamp {
        return Ok(pool.reward_infos);
    }

    // Calculate new global reward growth
    let mut next_reward_infos = pool.reward_infos;
    let time_delta = u128::from(next_timestamp - curr_timestamp);
    for reward_info in next_reward_infos.iter_mut() {
        if !reward_info.initialized() {
            continue;
        }

        // Calculate the new reward growth delta.
        // If the calculation overflows, set the delta value to zero.
        // This will halt reward distributions for this reward.
        let reward_growth_delta = checked_mul_div(
            time_delta,
            reward_info.emissions_per_second_x64,
            pool.liquidity
        ).unwrap_or(0);

        // Add the reward growth delta to the global reward growth.
        let curr_growth_global = reward_info.growth_global_x64;
        reward_info.growth_global_x64 = curr_growth_global.wrapping_add(reward_growth_delta);
    }

    Ok(next_reward_infos)
}

// Calculates the next global liquidity for a amm depending on its position relative
// to the lower and upper tick indexes and the liquidity_delta.
pub fn next_amm_liquidity(
    pool: &Pool,
    tick_upper_index: i32,
    tick_lower_index: i32,
    liquidity_delta: i128
) -> Result<u128, ErrorCode> {
    if pool.tick_current_index < tick_upper_index && pool.tick_current_index >= tick_lower_index {
        math::liquidity_math::add_liquidity_delta(pool.liquidity, liquidity_delta)
    } else {
        Ok(pool.liquidity)
    }
}
