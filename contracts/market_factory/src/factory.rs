use normal::{
    oracle::OracleGuardRails,
    types::market::{MarketFactoryConfig, MarketInfo, MarketParams},
};
use soroban_sdk::{contractclient, Address, BytesN, Env, Vec};

#[contractclient(name = "MarketFactoryClient")]
pub trait MarketFactoryTrait {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        governor: Address,
        insurance: Address,
        token_wasm_hash: BytesN<32>,
        market_wasm_hash: BytesN<32>,
    );

    #[allow(clippy::too_many_arguments)]
    fn create_market(env: Env, params: MarketParams) -> Address;

    fn update_super_keepers(env: Env, to_add: Vec<Address>, to_remove: Vec<Address>);

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

    fn query_for_market_by_token_pair(env: Env, token_a: Address, token_b: Address) -> Address;

    fn get_admin(env: Env) -> Address;

    fn get_config(env: Env) -> MarketFactoryConfig;

    // fn migrate_admin_key(env: Env) -> Result<(), ContractError>;
}
