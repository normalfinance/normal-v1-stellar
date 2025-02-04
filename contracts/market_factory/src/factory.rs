use normal::oracle::OracleGuardRails;
use soroban_sdk::{contractclient, Address, BytesN, Env, String, Symbol, Vec};

use crate::{
    errors::ContractError,
    storage::{Config, MarketInfo},
};

#[contractclient(name = "MarketFactoryClient")]
pub trait MarketFactoryTrait {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        governor: Address,
        synth_market_wasm_hash: BytesN<32>,
        token_wasm_hash: BytesN<32>,
    );

    #[allow(clippy::too_many_arguments)]
    fn create_market(
        env: Env,
        sender: Address,
        params: MarketParams,
        token_wasm_hash: BytesN<32>,
        synth_token_name: String,
        synth_token_symbol: String,
    ) -> Address;

    fn update_emergency_oracles(
        env: Env,
        sender: Address,
        to_add: Vec<Address>,
        to_remove: Vec<Address>,
    );

    fn update_wasm_hashes(
        env: Env,
        market_wasm_hash: Option<BytesN<32>>,
        token_wasm_hash: Option<BytesN<32>>,
    );

    fn update_oracle_guard_rails(env: Env, oracle_guard_rails: OracleGuardRails);

    // ################################################################
    //                             Queries
    // ################################################################

    fn query_markets(env: Env) -> Vec<Address>;

    fn query_market_details(env: Env, market_address: Address) -> MarketInfo;

    fn query_all_markets_details(env: Env) -> Vec<MarketInfo>;

    fn query_for_market_by_token_pair(env: Env, token_a: Symbol, token_b: Symbol) -> Address;

    fn get_admin(env: Env) -> Address;

    fn get_config(env: Env) -> Config;

    fn query_emergency_oracle(env: Env, oracle: Address) -> bool;

    fn migrate_admin_key(env: Env) -> Result<(), ContractError>;
}
