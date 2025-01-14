use soroban_sdk::{contracttype, Address, String, Vec};

#[derive(Clone)]
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
