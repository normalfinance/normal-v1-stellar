use normal::{
    constants::{ PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD },
    error::{ ErrorCode, NormalResult },
};
use soroban_sdk::{ contracttype, Address, Env, Vec };

use crate::{
    math::tick_math::FULL_RANGE_ONLY_TICK_SPACING_THRESHOLD,
    reward::PositionRewardInfo,
    storage::Pool,
    tick::Tick,
};

#[contracttype]
#[derive(Default, Debug, PartialEq)]
pub struct PositionUpdate {
    pub liquidity: u128,
    pub fee_growth_checkpoint_a: u128,
    pub fee_owed_a: u64,
    pub fee_growth_checkpoint_b: u128,
    pub fee_owed_b: u64,
    pub reward_infos: Vec<PositionRewardInfo>,
}

#[contracttype]
#[derive(Default, Debug, Clone)]
pub struct Position {
    pub liquidity: u128,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,

    // Q64.64
    pub fee_growth_checkpoint_a: u128,
    pub fee_owed_a: u64,

    // Q64.64
    pub fee_growth_checkpoint_b: u128,
    pub fee_owed_b: u64,

    pub reward_infos: Vec<PositionRewardInfo>,

    pub position_ts: u64,
}

impl Position {
    pub fn is_position_empty(&self) -> bool {
        let fees_not_owed = self.fee_owed_a == 0 && self.fee_owed_b == 0;
        let mut rewards_not_owed = true;
        for (i, reward_info) in self.reward_infos.iter().enumerate() {
            rewards_not_owed = rewards_not_owed && reward_info.amount_owed == 0;
        }
        self.liquidity == 0 && fees_not_owed && rewards_not_owed
    }

    pub fn update(&mut self, update: &PositionUpdate) {
        self.liquidity = update.liquidity;
        self.fee_growth_checkpoint_a = update.fee_growth_checkpoint_a;
        self.fee_growth_checkpoint_b = update.fee_growth_checkpoint_b;
        self.fee_owed_a = update.fee_owed_a;
        self.fee_owed_b = update.fee_owed_b;
        self.reward_infos = update.reward_infos;
    }

    pub fn open_position(
        &mut self,
        pool: &Pool,
        tick_lower_index: i32,
        tick_upper_index: i32
    ) -> Result<(), ErrorCode> {
        if
            !Tick::check_is_usable_tick(tick_lower_index, pool.tick_spacing) ||
            !Tick::check_is_usable_tick(tick_upper_index, pool.tick_spacing) ||
            tick_lower_index >= tick_upper_index
        {
            return Err(ErrorCode::InvalidTickIndex);
        }

        // On tick spacing >= 2^15, should only be able to open full range positions
        if pool.tick_spacing >= FULL_RANGE_ONLY_TICK_SPACING_THRESHOLD {
            let (full_range_lower_index, full_range_upper_index) = Tick::full_range_indexes(
                pool.tick_spacing
            );
            if
                tick_lower_index != full_range_lower_index ||
                tick_upper_index != full_range_upper_index
            {
                return Err(ErrorCode::FullRangeOnlyPool);
            }
        }

        self.tick_lower_index = tick_lower_index;
        self.tick_upper_index = tick_upper_index;

        Ok(())
    }

    pub fn reset_fees_owed(&mut self) {
        self.fee_owed_a = 0;
        self.fee_owed_b = 0;
    }

    pub fn update_reward_owed(&mut self, index: usize, amount_owed: u64) {
        self.reward_infos[index].amount_owed = amount_owed;
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositionInfo {
    /// Vec of positions sorted by position timestamp
    pub positions: Vec<Position>,
}

pub fn get_position_info(env: &Env, key: &Address) -> PositionInfo {
    let position_info = match env.storage().persistent().get::<_, PositionInfo>(key) {
        Some(info) => info,
        None =>
            PositionInfo {
                positions: Vec::new(env),
            },
    };
    env.storage()
        .persistent()
        .has(&key)
        .then(|| {
            env.storage()
                .persistent()
                .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        });

    position_info
}

pub fn save_position_info(env: &Env, key: &Address, position_info: &PositionInfo) {
    env.storage().persistent().set(key, position_info);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

pub fn get_position_by_ts(env: &Env, key: &Address, ts: u64) -> NormalResult<Position> {
    let position = get_position_info(env, key);
    let target_position = match position.positions.iter().find(|p| p.position_ts == ts) {
        Some(p) => p,
        None => {
            return Err(ErrorCode::AdminNotSet);
        }
    };

    Ok(target_position)
}
