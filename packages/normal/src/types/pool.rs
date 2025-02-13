use soroban_sdk::{contracttype, Address};

use crate::oracle::OracleSource;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolParams {
    pub oracle: Address,
    pub oracle_source: OracleSource,
    pub tick_spacing: u32,
    pub initial_sqrt_price: u128,
    pub fee_rate: i64,
    pub protocol_fee_rate: i64,
    pub max_allowed_slippage_bps: i64,
    pub max_allowed_variance_bps: i64,
}
