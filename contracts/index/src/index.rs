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
        name: String,
        symbol: String,
        is_public: bool,
        delegate: Option<Address>,
        fee_authority: Option<Address>,
        access_authority: Option<Address>,
        rebalance_authority: Option<Address>,
        assets: Vec<IndexAssetInfo>,
        manager_fee_bps: i64,
        revenue_share_bps: i64,
        whitelist: Option<Vec<Pubkey>>,
        blacklist: Option<Vec<Pubkey>>
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

    fn rebalance(env: Env, sender: Address);

    fn collect_fees(env: Env, sender: Address, to: Option<Address>);

    // ################################################################
    //                             USER
    // ################################################################

    fn mint(env: Env, sender: Address, to: Option<Address>, amount: i128);

    fn redeem(env: Env, sender: Address, amount: i128);

    fn collect_revenue_share(env: Env, sender: Address, to: Option<Address>);
}
