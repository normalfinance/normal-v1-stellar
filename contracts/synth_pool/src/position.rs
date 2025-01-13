use soroban_sdk::{ contracttype, Address, Env };

use crate::{ errors::ErrorCode, tick::Tick };

#[contracttype]
#[derive(Default)]
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

    pub reward_infos: [PositionRewardInfo; MAX_REWARDS], // 72
}

impl Position {
    pub fn is_position_empty(position: &Position) -> bool {
        let fees_not_owed = position.fee_owed_a == 0 && position.fee_owed_b == 0;
        let mut rewards_not_owed = true;
        for i in 0..MAX_REWARDS {
            rewards_not_owed = rewards_not_owed && position.reward_infos[i].amount_owed == 0;
        }
        position.liquidity == 0 && fees_not_owed && rewards_not_owed
    }

    pub fn update(&mut self, update: &PositionUpdate) {
        self.liquidity = update.liquidity;
        self.fee_growth_checkpoint_a = update.fee_growth_checkpoint_a;
        self.fee_growth_checkpoint_b = update.fee_growth_checkpoint_b;
        self.fee_owed_a = update.fee_owed_a;
        self.fee_owed_b = update.fee_owed_b;
        self.reward_infos = update.reward_infos;
    }

    pub fn open_position(&mut self, tick_lower_index: i32, tick_upper_index: i32) {
        if
            !Tick::check_is_usable_tick(tick_lower_index, amm.tick_spacing) ||
            !Tick::check_is_usable_tick(tick_upper_index, amm.tick_spacing) ||
            tick_lower_index >= tick_upper_index
        {
            return Err(ErrorCode::InvalidTickIndex.into());
        }

        // On tick spacing >= 2^15, should only be able to open full range positions
        if amm.tick_spacing >= FULL_RANGE_ONLY_TICK_SPACING_THRESHOLD {
            let (full_range_lower_index, full_range_upper_index) = Tick::full_range_indexes(
                amm.tick_spacing
            );
            if
                tick_lower_index != full_range_lower_index ||
                tick_upper_index != full_range_upper_index
            {
                return Err(ErrorCode::FullRangeOnlyPool.into());
            }
        }

        self.tick_lower_index = tick_lower_index;
        self.tick_upper_index = tick_upper_index;
    }

    pub fn reset_fees_owed(&mut self) {
        self.fee_owed_a = 0;
        self.fee_owed_b = 0;
    }

    pub fn update_reward_owed(&mut self, index: usize, amount_owed: u64) {
        self.reward_infos[index].amount_owed = amount_owed;
    }
}

pub struct Positions;

impl Positions {
    // Key for storing positions in contract storage
    fn key(address: &Address) -> String {
        format!("positions:{}", address.to_string())
    }

    // Get positions for an address
    pub fn get(env: &Env, address: &Address) -> Vec<Position> {
        env.storage().get::<Vec<Position>>(&Self::key(address)).unwrap_or(Vec::new(env)) // Return an empty Vec if no data exists
    }

    // Get a specific position by index for an address
    pub fn get_by_index(env: &Env, address: &Address, index: usize) -> Option<Position> {
        let positions = Self::get(env, address); // Fetch current positions
        if index < positions.len() {
            Some(positions.get(index).unwrap()) // Return the position if the index is valid
        } else {
            None // Return None if the index is out of bounds
        }
    }

    // Add a position for an address
    pub fn add(env: &Env, address: &Address, position: Position) {
        let mut positions = Self::get(env, address); // Fetch current positions
        positions.push_back(position); // Add the new position
        env.storage().set(&Self::key(address), &positions); // Save back to storage
    }

    // Remove a position for an address (example: by index)
    pub fn remove(env: &Env, address: &Address, index: usize) {
        let mut positions = Self::get(env, address); // Fetch current positions
        if index < positions.len() {
            positions.remove(index); // Remove the position at the given index
            env.storage().set(&Self::key(address), &positions); // Save back to storage
        }
    }
}

#[contracttype]
#[derive(Default, Debug, PartialEq)]
pub struct PositionUpdate {
    pub liquidity: u128,
    pub fee_growth_checkpoint_a: u128,
    pub fee_owed_a: u64,
    pub fee_growth_checkpoint_b: u128,
    pub fee_owed_b: u64,
    pub reward_infos: [PositionRewardInfo; MAX_REWARDS],
}
