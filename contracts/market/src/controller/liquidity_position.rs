use normal::error::NormalResult;
use soroban_sdk::Vec;

use crate::{
    math::{ bit_math::checked_mul_shift_right, liquidity_math::add_liquidity_delta },
    state::liquidity_position::LiquidityPosition,
};

pub fn next_position_modify_liquidity_update(
    position: &LiquidityPosition,
    liquidity_delta: i128,
    fee_growth_inside_a: u128,
    fee_growth_inside_b: u128,
    reward_growths_inside: &Vec<u128>
) -> NormalResult<PositionUpdate> {
    let mut update = PositionUpdate::default();

    // Calculate fee deltas.
    // If fee deltas overflow, default to a zero value. This means the position loses
    // all fees earned since the last time the position was modified or fees collected.
    let growth_delta_synthetic = fee_growth_inside_a.wrapping_sub(
        position.fee_growth_checkpoint_a
    );
    let fee_delta_synthetic = checked_mul_shift_right(
        position.liquidity,
        growth_delta_synthetic
    ).unwrap_or(0);

    let growth_delta_quote = fee_growth_inside_b.wrapping_sub(position.fee_growth_checkpoint_b);
    let fee_delta_quote = checked_mul_shift_right(position.liquidity, growth_delta_quote).unwrap_or(
        0
    );

    update.fee_growth_checkpoint_a = fee_growth_inside_a;
    update.fee_growth_checkpoint_b = fee_growth_inside_b;

    // Overflows allowed. Must collect fees owed before overflow.
    update.fee_owed_a = position.fee_owed_a.wrapping_add(fee_delta_synthetic);
    update.fee_owed_b = position.fee_owed_b.wrapping_add(fee_delta_quote);

    for (i, update) in update.reward_infos.iter_mut().enumerate() {
        let reward_growth_inside = reward_growths_inside[i];
        let curr_reward_info = position.reward_infos[i];

        // Calculate reward delta.
        // If reward delta overflows, default to a zero value. This means the position loses all
        // rewards earned since the last time the position was modified or rewards were collected.
        let reward_growth_delta = reward_growth_inside.wrapping_sub(
            curr_reward_info.growth_inside_checkpoint
        );
        let amount_owed_delta = checked_mul_shift_right(
            position.liquidity,
            reward_growth_delta
        ).unwrap_or(0);

        update.growth_inside_checkpoint = reward_growth_inside;

        // Overflows allowed. Must collect rewards owed before overflow.
        update.amount_owed = curr_reward_info.amount_owed.wrapping_add(amount_owed_delta);
    }

    update.liquidity = add_liquidity_delta(position.liquidity, liquidity_delta)?;

    Ok(update)
}
