use normal::{
    error::{ErrorCode, NormalResult},
    types::{IndexAsset, IndexParams},
};
use soroban_sdk::{contractclient, Address, Env, Vec};

use crate::{msg::IndexResponse, storage::IndexOperation};

#[contractclient(name = "IndexTokenClient")]
pub trait IndexTokenTrait {
    fn initialize(
        env: Env,
        admin: Address,
        factory: Address,
        initial_deposit: i128,
        params: IndexParams,
    ) -> Result<(), ErrorCode>;

    fn update_manager_fee(env: Env, sender: Address, manager_fee_bps: i64);

    fn update_paused_operations(
        env: Env,
        admin: Address,
        to_add: Vec<IndexOperation>,
        to_remove: Vec<IndexOperation>,
    );

    fn update_whitelist(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>);

    fn update_blacklist(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>);

    fn update_rebalance_threshold(env: Env, sender: Address, rebalance_threshold: u64);

    // ################################################################
    //                              KEEPER
    // ################################################################

    fn rebalance(env: Env, sender: Address, updated_assets: Vec<IndexAsset>);

    // ################################################################
    //                             USER
    // ################################################################

    fn mint(env: Env, sender: Address, index_token_amount: i128) -> NormalResult;

    fn redeem(env: Env, sender: Address, index_token_amount: i128) -> NormalResult;

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_index(env: Env) -> IndexResponse;

    // fn query_price(env: Env) -> i128;

    // fn query_nav(env: Env) -> i128;

    // fn query_index_info_for_factory(env: Env) -> IndexInfo;
}
