use soroban_sdk::{ contractclient, Address, Env, String };

#[contractclient(name = "IndexClient")]
pub trait IndexTrait {
    // ################################################################
    //                             ADMIN
    // ################################################################

    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        factory_addr: Address,
        name: String,
        symbol: String,
        initial_price: i32,
        initial_deposit: i128,
        is_public: bool,
        active_status: IndexStatus,
        delegate: Option<Address>,
        fee_authority: Option<Address>,
        access_authority: Option<Address>,
        rebalance_authority: Option<Address>,
        assets: Vec<IndexAssetInfo>,
        manager_fee_bps: i64,
        revenue_share_bps: i64,
        whitelist: Option<Vec<Address>>,
        blacklist: Option<Vec<Address>>
    );

    fn update_fees(
        env: Env,
        sender: Address,
        manager_fee_bps: Option<i64>,
        revenue_share_bps: Option<i64>
    );

    fn update_is_public(env: Env, sender: Address, is_public: bool);

    fn initialize_shutdown(env: Env, sender: Address, expiry_ts: i64);

    fn update_paused_operations(env: Env, admin: Address, operations: Vec<Operation>);

    fn update_whitelist(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>);

    fn update_blacklist(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>);

    fn update_weights(env: Env, sender: Address);

    fn rebalance(env: Env, sender: Address);

    fn collect_fees(env: Env, sender: Address, to: Option<Address>);

    // ################################################################
    //                             USER
    // ################################################################

    fn mint(env: Env, sender: Address, index_token_amount: i128, to: Option<Address>);

    fn redeem(env: Env, sender: Address, index_token_amount: i128, to: Option<Address>);

    fn collect_revenue_share(env: Env, sender: Address, to: Option<Address>);

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_index(env: Env) -> Index;

    fn query_price(env: Env) -> i128;

    fn query_nav(env: Env) -> i128;
}
