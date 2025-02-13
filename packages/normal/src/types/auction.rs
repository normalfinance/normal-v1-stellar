use soroban_sdk::contracttype;

use super::misc::OrderDirection;

/// Auction types
#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum AuctionType {
    /// selling collateral from a liquidation
    Collateral,
    /// selling newly minted NORM to cover Protocol Debt (the deficit from Collateral Auctions)
    Debt,
    /// selling excess synthetic token proceeds over the Insurance Fund max limit for NORM to be burned
    Surplus,
}

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum AuctionLocation {
    /// Sell the asset directly to users via Normal interface
    Native,
    /// Sell the asset via a 3rd-party DEX
    External,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Auction {
    pub amount: i128,
    pub direction: OrderDirection,
    pub location: AuctionLocation,
    pub duration: u64,
    pub start_ts: u64,
    pub total_auctioned: i128,
    pub start_price: u64,
    pub end_price: u64,
}
