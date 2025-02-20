use soroban_sdk::{contracttype, Address, BytesN, String, Vec};

use crate::oracle::{OracleGuardRails, OracleSource};

use super::pool::PoolParams;

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarketFactoryConfig {
    pub admin: Address,
    pub insurance: Address,
    pub market_wasm_hash: BytesN<32>,
    pub token_wasm_hash: BytesN<32>,
    pub super_keepers: Vec<Address>,
    pub oracle_guard_rails: OracleGuardRails,
    // pub lp_token_decimals: u32,
}

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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketParams {
    // Token
    pub token_decimals: u32,
    pub synth_token_name: String,          // Normal Bitcoin
    pub synth_token_symbol: String,        // nBTC
    pub synth_target_token_symbol: String, // BTC
    pub quote_token: Address,
    pub quote_token_symbol: String, // XLM
    pub lp_token_symbol: String,    // nBTC-XLM LP
    // Market
    pub name: String,
    pub decimals: u32,
    pub active_status: bool,
    pub tier: SynthTier,
    pub oracle_source: OracleSource, // Oracle
    pub oracle: Address,
    pub margin_ratio_initial: u32, // Margin
    pub margin_ratio_maintenance: u32,
    pub imf_factor: u32,
    pub liquidation_penalty: u32, // Liquidation
    pub liquidator_fee: u32,
    pub if_liquidation_fee: u32,
    pub debt_ceiling: u128,
    pub debt_floor: u32,
    // Pool
    pub pool: PoolParams,
}

/// This struct is used to return a query result with the total amount of LP tokens and assets in a specific pool.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarketResponse {
    pub name: String,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarketInfo {
    pub market_address: Address,
    pub market_response: MarketResponse,
}
