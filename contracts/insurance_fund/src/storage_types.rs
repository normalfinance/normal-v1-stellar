use soroban_sdk::{ contracttype, Address };

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    MaxInsurance,
    UnstakingPeriod,
    PausedOperations,
    // Stake(Address),
    // ...?
}

#[derive(Clone)]
#[contracttype]
pub struct InsuranceFund {
    pub pubkey: Address,
    pub authority: Address,
    pub vault: Address,
    pub total_shares: u128,
    pub user_shares: u128,
    pub shares_base: u128, // exponent for lp shares (for rebasing)
    // pub unstaking_period: i64,
    pub last_revenue_settle_ts: i64,
    pub revenue_settle_period: i64,
    pub total_factor: u32, // percentage of interest for total insurance
    pub user_factor: u32, // percentage of interest for user staked insurance
    // pub max_insurance: u64,
    pub paused_operations: u32,
}

#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum InsuranceFundOperation {
    Init = 0b00000001,
    Add = 0b00000010,
    RequestRemove = 0b00000100,
    Remove = 0b00001000,
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
