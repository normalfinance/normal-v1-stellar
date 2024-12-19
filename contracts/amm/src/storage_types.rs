use soroban_sdk::{ contracttype, Address };

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Admin,
    TokenA = 0,
    TokenB = 1,
    ReserveA = 2,
    ReserveB = 3,
    TickSpacing,
    TickCurrentIndex,
    Liquidity,
    SqrtPrice,
    FeeRate,
    ProtocolFeeRate,
    FeeGrowthGlobalA,
    FeeGrowthGlobalB,
    ProtocolFeeOwedA,
    ProtocolFeeOwedB,
    RewardAuthority,
    RewardLastUpdatedTs,
    RewardInfos(u64),
}

#[derive(Clone)]
#[contracttype]
pub struct AMMRewardInfo {
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
}
