use soroban_sdk::{contracttype, Address, String, Vec};

// ################################################################
//                             Synthetic
// ################################################################

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord)]
pub enum SynthTier {
    /// max insurance capped at A level
    A,
    /// max insurance capped at B level
    B,
    /// max insurance capped at C level
    C,
    /// no insurance
    Speculative,
    /// no insurance, another tranches below
    HighlySpeculative,
    /// no insurance, only single position allowed
    Isolated,
}

impl SynthTier {
    pub fn is_as_safe_as_synth(&self, other: &SynthTier) -> bool {
        // Synth Tier A safest
        self <= other
    }
}

// ################################################################
//                             Auction
// ################################################################

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

// ################################################################
//                             OTHER
// ################################################################

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum OrderDirection {
    Buy,
    Sell,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SynthMarketInitInfo {}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexAsset {
    /// Address of the synth market
    pub market_address: Address,
    /// The portfolio allocation of the asset
    pub weight: i128,
    pub last_updated_ts: i64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexTokenInitInfo {
    // Token Info
    pub decimal: u32,
    pub name: String,
    pub symbol: String,
    // Index Info
    pub initial_price: i32,
    pub initial_deposit: i128,
    pub is_public: bool,
    pub component_assets: Vec<IndexAsset>,
    pub manager_fee_bps: i64,
}
