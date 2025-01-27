use normal::error::ErrorCode;
use soroban_sdk::{ Address, Vec };

use crate::storage::Config;

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
        // TODO:
        // self.mint.ne(&Pubkey::default())
    }

    /// Maps all reward data to only the reward growth accumulators
    pub fn to_reward_growths(reward_infos: &Vec<RewardInfo>) -> Vec<u128> {
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

pub fn calculate_collect_reward(position_reward: PositionRewardInfo, vault_amount: u64) -> (u64, u64) {
    let amount_owed = position_reward.amount_owed;
    let (transfer_amount, updated_amount_owed) = if amount_owed > vault_amount {
        (vault_amount, amount_owed - vault_amount)
    } else {
        (amount_owed, 0)
    };

    (transfer_amount, updated_amount_owed)
}
