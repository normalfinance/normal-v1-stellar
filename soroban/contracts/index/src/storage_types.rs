use soroban_sdk::{ contracttype, Address };

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Admin,
    Name,
    Privacy,
    ExpenseRatio,
    RevenueShare,
    Assets(IndexAsset),
    Whitelist(Address),
    Blacklist(Address),
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, contracttype)]
pub enum Operation {
    Mint,
    Redeem,
    Rebalance,
    Update,
}

pub struct IndexAsset {
    pub market_index: u16,
    pub weight: u16, // The asset's allocation (in basis points)
    pub last_updated_ts: i64,
}
