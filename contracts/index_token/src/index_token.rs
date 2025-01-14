use normal::types::IndexTokenInitInfo;
use soroban_sdk::{contractclient, Address, Env, String, Vec};

use crate::{
    contract::Index,
    storage::{IndexAsset, IndexOperation, TransferWithFees},
};

#[contractclient(name = "IndexTokenClient")]
pub trait IndexTokenTrait {
    // ################################################################
    //                             ADMIN
    // ################################################################

    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        factory: Address,
        quote_token: Address,
        rebalance_threshold: i64,
        params: IndexTokenInitInfo,
    );

    fn update_manager_fee(env: Env, sender: Address, manager_fee_bps: i64);

    fn update_paused_operations(
        env: Env,
        admin: Address,
        to_add: Vec<IndexOperation>,
        to_remove: Vec<IndexOperation>,
    );

    fn update_whitelist(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>);

    fn update_blacklist(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>);

    fn update_rebalance_threshold(env: Env, sender: Address, rebalance_threshold: i64);

    // ################################################################
    //                              KEEPER
    // ################################################################

    fn rebalance(env: Env, sender: Address, updated_assets: Vec<IndexAsset>);

    // ################################################################
    //                             USER
    // ################################################################

    fn mint(env: Env, sender: Address, index_token_amount: i128, to: Option<Address>);

    fn redeem(env: Env, sender: Address, index_token_amount: i128, to: Option<Address>);

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_index(env: Env) -> Index;

    fn query_price(env: Env) -> i128;

    fn query_nav(env: Env) -> i128;

    fn query_fee_exemption(env: Env, user: Address) -> bool;

    fn query_fees_for_transfer(env: &Env, from: Address, amount: i128) -> TransferWithFees;
}
