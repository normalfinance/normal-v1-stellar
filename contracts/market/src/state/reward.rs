use normal::error::ErrorCode;
use soroban_sdk::{contracttype, Address, Env, Vec};

/// Stores the state relevant for tracking liquidity mining rewards at the `AMM` level.
/// These values are used in conjunction with `PositionRewardInfo`, `Tick.reward_growths_outside`,
/// and `AMM.reward_last_updated_timestamp` to determine how many rewards are earned by open
/// positions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardInfo {
    /// Reward token.
    pub token: Address,
    /// Authority account that has permission to initialize the reward and set emissions.
    pub authority: Address,
    /// TODO: The Market's balance of the reward token
    pub initial_balance: i128,
    pub current_balance: i128,
    /// Q64.64 number that indicates how many tokens per second are earned per unit of liquidity.
    pub emissions_per_second_x64: u128,
    /// Q64.64 number that tracks the total tokens earned per unit of liquidity since the reward
    /// emissions were turned on.
    pub growth_global_x64: u128,
}

impl RewardInfo {
    /// Returns true if this reward is initialized.
    /// Once initialized, a reward cannot transition back to uninitialized.
    pub fn initialized(&self) -> bool {
        self.token.is_some()
    }

    /// Maps all reward data to only the reward growth accumulators
    pub fn to_reward_growths(env: &Env, reward_infos: &Vec<RewardInfo>) -> Vec<u128> {
        let mut reward_growths: Vec<u128> = Vec::new(env);
        for (i, reward_info) in reward_infos.iter().enumerate() {
            reward_growths[i] = reward_info.growth_global_x64;
        }
        reward_growths
    }

    pub fn get_reward_by_token(reward_infos: &Vec<RewardInfo>, token: Address) -> RewardInfo {
        for reward in reward_infos.iter() {
            if reward.token == token {
                return reward;
            }
        }
        return Err(ErrorCode::AdminNotSet);
    }
}

#[contracttype]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct LiquidityPositionRewardInfo {
    // Q64.64
    pub growth_inside_checkpoint: u128,
    pub amount_owed: u64,
}

pub fn calculate_collect_reward(
    position_reward: LiquidityPositionRewardInfo,
    vault_amount: u64,
) -> (u64, u64) {
    let amount_owed = position_reward.amount_owed;
    let (transfer_amount, updated_amount_owed) = if amount_owed > vault_amount {
        (vault_amount, amount_owed - vault_amount)
    } else {
        (amount_owed, 0)
    };

    (transfer_amount, updated_amount_owed)
}
