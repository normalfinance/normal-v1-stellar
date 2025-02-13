use soroban_sdk::{contracttype, Address, String, Vec};

use crate::oracle::OracleSource;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexAsset {
    /// Address of the synth market
    pub market: Address,
    /// The portfolio allocation of the asset
    pub weight: i128,
    pub last_updated_ts: i64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexParams {
    // Token
    pub decimal: u32,
    pub name: String,
    pub symbol: String,
    // Index
    pub quote_token: Address,
    pub oracle: Address,
    pub oracle_source: OracleSource,
    pub is_public: bool,
    pub manager_fee_bps: i64,
    pub whitelist: Vec<Address>,
    pub blacklist: Vec<Address>,
    pub initial_price: i128,
    pub initial_deposit: i128,
    pub component_assets: Vec<IndexAsset>,
    pub rebalance_threshold: u64,
}
