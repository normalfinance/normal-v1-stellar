use normal::{error::ErrorCode, types::IndexParams};
use soroban_sdk::{contractclient, Address, BytesN, Env, String, Vec};

use crate::storage::{Config, IndexInfo, Operation};

#[contractclient(name = "IndexTokenFactoryClient")]
pub trait IndexTokenFactoryTrait {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        index_wasm_hash: BytesN<32>,
        quote_token_whitelist: Vec<Address>,
        paused_operations: Vec<Operation>,
        max_manager_fee_bps: i64,
        protocol_fee_bps: i64,
        default_oracle: Address,
    );

    #[allow(clippy::too_many_arguments)]
    fn create_index_token(
        env: Env,
        sender: Address,
        initial_deposit: i128,
        params: IndexParams,
    ) -> Address;

    fn update_wasm_hashes(env: Env, index_token_wasm_hash: BytesN<32>);

    fn update_config(
        env: Env,
        paused_operations: Option<Vec<Operation>>,
        max_manager_fee_bps: Option<i64>,
        protocol_fee_bps: Option<i64>,
        default_oracle: Option<Address>,
    );

    // ################################################################
    //                             Queries
    // ################################################################

    fn query_indexes(env: Env) -> Vec<Address>;

    fn query_index_details(env: Env, index_address: Address) -> IndexInfo;

    fn query_all_indexes_details(env: Env) -> Vec<IndexInfo>;

    // For indexes to query AMMs via the Synth Market Factory
    // fn query_for_amm_by_market(env: Env, marekt: Address) -> Address;

    fn query_for_index_by_tuple(env: Env, name: String, symbol: String) -> Address;

    fn get_admin(env: Env) -> Address;

    fn get_config(env: Env) -> Config;

    // fn query_user_portfolio(env: Env, sender: Address, staking: bool) -> UserPortfolio;

    fn migrate_admin_key(env: Env) -> Result<(), ErrorCode>;
}
