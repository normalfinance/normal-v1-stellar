use soroban_sdk::{ contracttype, Address };

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    MaxInsurance,
    UnstakingPeriod,
    PausedOperations,
    TotalShares,
    UserShares,
    SharesBase, // exponent for lp shares (for rebasing)
    LastRevenueSettleTs,
    RevenueSettlePeriod,
    TotalFactor, // percentage of interest for total insurance
    UserFactor, // percentage of interest for user staked insurance
    Stake(Address),
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, contracttype)]
pub enum Operation {
    Stake,
    Unstake,
}

#[derive(Clone)]
#[contracttype]
pub struct Stake {
    pub authority: Address,
    if_shares: u128,
    pub last_withdraw_request_shares: u128, // get zero as 0 when not in escrow
    pub if_base: u128, // exponent for if_shares decimal places (for rebase)
    pub last_valid_ts: i64,
    pub last_withdraw_request_value: u64,
    pub last_withdraw_request_ts: i64,
    pub cost_basis: i64,
}
