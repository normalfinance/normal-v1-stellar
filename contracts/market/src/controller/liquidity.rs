use soroban_sdk::{contracttype, Env, Vec};

use crate::{
    controller,
    errors::{Errors, NormalResult},
    math,
    state::{
        liquidity_position::{LiquidityPosition, LiquidityPositionUpdate},
        pool::Pool,
        reward::RewardInfo,
        tick::{Tick, TickUpdate},
        tick_array::TickArray,
    },
};

#[contracttype]
#[derive(Debug)]
pub struct ModifyLiquidityUpdate {
    pub amm_liquidity: u128,
    pub tick_lower_update: TickUpdate,
    pub tick_upper_update: TickUpdate,
    pub reward_infos: Vec<RewardInfo>,
    pub position_update: LiquidityPositionUpdate,
}

// Calculates state after modifying liquidity by the liquidity_delta for the given positon.
// Fee and reward growths will also be calculated by this function.
// To trigger only calculation of fee and reward growths, use calculate_fee_and_reward_growths.
pub fn calculate_modify_liquidity(
    env: &Env,
    pool: &Pool,
    position: &LiquidityPosition,
    tick_array_lower: &TickArray,
    tick_array_upper: &TickArray,
    liquidity_delta: i128,
    timestamp: u64,
) -> NormalResult<ModifyLiquidityUpdate> {
    let tick_lower = tick_array_lower.get_tick(position.tick_lower_index, pool.tick_spacing)?;
    let tick_upper = tick_array_upper.get_tick(position.tick_upper_index, pool.tick_spacing)?;

    _calculate_modify_liquidity(
        env,
        pool,
        position,
        tick_lower,
        tick_upper,
        position.tick_lower_index,
        position.tick_upper_index,
        liquidity_delta,
        timestamp,
    )
}

pub fn calculate_fee_and_reward_growths(
    env: &Env,
    pool: &Pool,
    position: &LiquidityPosition,
    tick_array_lower: &TickArray,
    tick_array_upper: &TickArray,
    timestamp: u64,
) -> NormalResult<(LiquidityPositionUpdate, Vec<RewardInfo>)> {
    let tick_lower = tick_array_lower.get_tick(position.tick_lower_index, pool.tick_spacing)?;
    let tick_upper = tick_array_upper.get_tick(position.tick_upper_index, pool.tick_spacing)?;

    // Pass in a liquidity_delta value of 0 to trigger only calculations for fee and reward growths.
    // Calculating fees and rewards for positions with zero liquidity will result in an error.
    let update = _calculate_modify_liquidity(
        env,
        pool,
        position,
        tick_lower,
        tick_upper,
        position.tick_lower_index,
        position.tick_upper_index,
        0,
        timestamp,
    )?;
    Ok((update.position_update, update.reward_infos))
}

// Calculates the state changes after modifying liquidity of a amm position.
#[allow(clippy::too_many_arguments)]
fn _calculate_modify_liquidity(
    env: &Env,
    pool: &Pool,
    position: &LiquidityPosition,
    tick_lower: &Tick,
    tick_upper: &Tick,
    tick_lower_index: i32,
    tick_upper_index: i32,
    liquidity_delta: i128,
    timestamp: u64,
) -> NormalResult<ModifyLiquidityUpdate> {
    // Disallow only updating position fee and reward growth when position has zero liquidity
    if liquidity_delta == 0 && position.liquidity == 0 {
        return Err(Errors::LiquidityZero);
    }

    let next_reward_infos = controller::pool::next_amm_reward_infos(pool, timestamp)?;

    let next_global_liquidity = controller::pool::next_amm_liquidity(
        pool,
        position.tick_upper_index,
        position.tick_lower_index,
        liquidity_delta,
    )?;

    let tick_lower_update = controller::tick::next_tick_modify_liquidity_update(
        env,
        tick_lower,
        tick_lower_index,
        pool.tick_current_index,
        pool.fee_growth_global_a,
        pool.fee_growth_global_b,
        &next_reward_infos,
        liquidity_delta,
        false,
    )?;

    let tick_upper_update = controller::tick::next_tick_modify_liquidity_update(
        env,
        tick_upper,
        tick_upper_index,
        pool.tick_current_index,
        pool.fee_growth_global_a,
        pool.fee_growth_global_b,
        &next_reward_infos,
        liquidity_delta,
        true,
    )?;

    let (fee_growth_inside_a, fee_growth_inside_b) = controller::tick::next_fee_growths_inside(
        pool.tick_current_index,
        tick_lower,
        tick_lower_index,
        tick_upper,
        tick_upper_index,
        pool.fee_growth_global_a,
        pool.fee_growth_global_b,
    );

    let reward_growths_inside = controller::tick::next_reward_growths_inside(
        env,
        pool.tick_current_index,
        tick_lower,
        tick_lower_index,
        tick_upper,
        tick_upper_index,
        &next_reward_infos,
    );

    let position_update = controller::liquidity_position::next_position_modify_liquidity_update(
        position,
        liquidity_delta,
        fee_growth_inside_a,
        fee_growth_inside_b,
        &reward_growths_inside,
    )?;

    Ok(ModifyLiquidityUpdate {
        amm_liquidity: next_global_liquidity,
        reward_infos: next_reward_infos,
        position_update,
        tick_lower_update,
        tick_upper_update,
    })
}

pub fn calculate_liquidity_token_deltas(
    current_tick_index: i32,
    sqrt_price: u128,
    position: &LiquidityPosition,
    liquidity_delta: i128,
) -> NormalResult<(i128, i128)> {
    // if liquidity_delta == 0 {
    //     return Err(ContractError::LiquidityZero.into());
    // }

    let mut delta_a: i128 = 0;
    let mut delta_b: i128 = 0;

    let liquidity: u128 = liquidity_delta.unsigned_abs();
    let round_up = liquidity_delta > 0;

    let lower_price = math::tick_math::sqrt_price_from_tick_index(position.tick_lower_index);
    let upper_price = math::tick_math::sqrt_price_from_tick_index(position.tick_upper_index);

    if current_tick_index < position.tick_lower_index {
        // current tick below position
        delta_a =
            math::token_math::get_amount_delta_a(lower_price, upper_price, liquidity, round_up)?;
    } else if current_tick_index < position.tick_upper_index {
        // current tick inside position
        delta_a =
            math::token_math::get_amount_delta_a(sqrt_price, upper_price, liquidity, round_up)?;
        delta_b =
            math::token_math::get_amount_delta_b(lower_price, sqrt_price, liquidity, round_up)?;
    } else {
        // current tick above position
        delta_b =
            math::token_math::get_amount_delta_b(lower_price, upper_price, liquidity, round_up)?;
    }

    Ok((delta_a, delta_b))
}

pub fn sync_modify_liquidity_values(
    pool: &mut Pool,
    position: &mut LiquidityPosition,
    tick_array_lower: &mut TickArray,
    tick_array_upper: &mut TickArray,
    modify_liquidity_update: ModifyLiquidityUpdate,
    reward_last_updated_timestamp: u64,
) -> NormalResult<()> {
    position.update(&modify_liquidity_update.position_update);

    tick_array_lower.update_tick(
        position.tick_lower_index,
        pool.tick_spacing,
        &modify_liquidity_update.tick_lower_update,
    )?;

    tick_array_upper.update_tick(
        position.tick_upper_index,
        pool.tick_spacing,
        &modify_liquidity_update.tick_upper_update,
    )?;

    pool.update_rewards_and_liquidity(
        modify_liquidity_update.reward_infos,
        modify_liquidity_update.amm_liquidity,
        reward_last_updated_timestamp,
    );

    Ok(())
}

pub fn calculate_collateral_liquidity_token_delta(
    current_tick_index: i32,
    sqrt_price: u128,
    position: &LiquidityPosition,
    liquidity_delta: i128,
) -> NormalResult<u64> {
    if liquidity_delta == 0 {
        return Err(Errors::LiquidityZero.into());
    }

    let mut delta_b: u64 = 0;

    let liquidity: u128 = liquidity_delta.unsigned_abs();
    let round_up = liquidity_delta > 0;

    let lower_price = math::tick_math::sqrt_price_from_tick_index(position.tick_lower_index);
    let upper_price = math::tick_math::sqrt_price_from_tick_index(position.tick_upper_index);

    if current_tick_index < position.tick_lower_index {
        // current tick below position
    } else if current_tick_index < position.tick_upper_index {
        // current tick inside position

        delta_b =
            math::token_math::get_amount_delta_b(lower_price, sqrt_price, liquidity, round_up)?;
    } else {
        // current tick above position
        delta_b =
            math::token_math::get_amount_delta_b(lower_price, upper_price, liquidity, round_up)?;
    }

    Ok(delta_b)
}
