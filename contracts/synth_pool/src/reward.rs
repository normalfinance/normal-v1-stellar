/// Stores the state relevant for tracking liquidity mining rewards at the `AMM` level.
/// These values are used in conjunction with `PositionRewardInfo`, `Tick.reward_growths_outside`,
/// and `AMM.reward_last_updated_timestamp` to determine how many rewards are earned by open
/// positions.
#[contracttype]
#[derive(Clone)]
pub struct RewardInfo {
    /// Reward token mint.
    pub token: Address,
    /// Reward vault token account.
    pub vault: Address,
    /// Authority account that has permission to initialize the reward and set emissions.
    pub authority: Address,
    /// Q64.64 number that indicates how many tokens per second are earned per unit of liquidity.
    pub emissions_per_second_x64: u128,
    /// Q64.64 number that tracks the total tokens earned per unit of liquidity since the reward
    /// emissions were turned on.
    pub growth_global_x64: u128,
    /// The timestamp when the stake was made
    pub reward_timestamp: u64,
}

impl RewardInfo {
    /// Returns true if this reward is initialized.
    /// Once initialized, a reward cannot transition back to uninitialized.
    pub fn initialized(&self) -> bool {
        self.mint.ne(&Pubkey::default())
    }

    /// Maps all reward data to only the reward growth accumulators
    pub fn to_reward_growths(reward_infos: &[RewardInfo; MAX_REWARDS]) -> [u128; MAX_REWARDS] {
        let mut reward_growths = [0u128; MAX_REWARDS];
        for i in 0..MAX_REWARDS {
            reward_growths[i] = reward_infos[i].growth_global_x64;
        }
        reward_growths
    }
}

#[contracttype]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct PositionRewardInfo {
    // Q64.64
    pub growth_inside_checkpoint: u128,
    pub amount_owed: u64,
}

// Calculates the next global reward growth variables based on the given timestamp.
// The provided timestamp must be greater than or equal to the last updated timestamp.
pub fn next_amm_reward_infos(
    amm: &AMM,
    next_timestamp: u64
) -> Result<[AMMRewardInfo; NUM_REWARDS], ErrorCode> {
    let curr_timestamp = amm.reward_last_updated_timestamp;
    if next_timestamp < curr_timestamp {
        return Err(ErrorCode::InvalidTimestamp);
    }

    // No-op if no liquidity or no change in timestamp
    if amm.liquidity == 0 || next_timestamp == curr_timestamp {
        return Ok(amm.reward_infos);
    }

    // Calculate new global reward growth
    let mut next_reward_infos = amm.reward_infos;
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
            amm.liquidity
        ).unwrap_or(0);

        // Add the reward growth delta to the global reward growth.
        let curr_growth_global = reward_info.growth_global_x64;
        reward_info.growth_global_x64 = curr_growth_global.wrapping_add(reward_growth_delta);
    }

    Ok(next_reward_infos)
}
