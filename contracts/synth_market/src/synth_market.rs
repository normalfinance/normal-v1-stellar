use soroban_sdk::{contractclient, Address, Env, String};

#[contractclient(name = "SynthMarketClient")]
pub trait SynthMarketTrait {
    // ################################################################
    //                             ADMIN
    // ################################################################
    fn initialize(env: Env, sender: Address, params: SynthMarketParams);

    fn initialize_shutdown(env: Env, sender: Address, expiry_ts: i64);

    fn update_paused_operations(env: Env, admin: Address, operations: Vec<Operation>);

    fn update_amm(env: Env, admin: Address, amm: Address);

    fn update_debt_limit(
        env: Env,
        admin: Address,
        debt_floor: Option<u32>,
        debt_ceiling: Option<u128>,
    );

    fn extend_expiry_ts(env: Env, admin: Address, expiry_timestamp: i64);

    fn update_margin_config(
        env: Env,
        admin: Address,
        margin_ratio_initial: u32,
        margin_ratio_maintenance: u32,
        imf_factor: Option<u32>,
    );

    fn update_liquidation_config(
        env: Env,
        admin: Address,
        liquidation_fee: u32,
        if_liquidation_fee: u32,
        liquidation_penalty: Option<u32>,
    );

    fn update_name(env: Env, admin: Address, name: String);

    fn update_status(env: Env, admin: Address, status: MarketStatus);

    fn update_synth_tier(env: Env, admin: Address, synth_tier: SynthTier);

    // ################################################################
    //                             KEEPER
    // ################################################################

    fn update_oracle(env: Env, keeper: Address, oracle: Address, oracle_source: OracleSource);

    fn freeze_oracle(env: Env, keeper: Address) {}

    fn lend_collateral(e: Env, fee_rate: u128);

    fn unlend_collateral(e: Env, fee_rate: u128);

    fn liquidate_position(e: Env, fee_rate: u128);

    fn resolve_position_bankruptcy(e: Env, fee_rate: u128);

    // ################################################################
    //                             USER
    // ################################################################

    fn deposit_collateral(env: Env, user: Address, amount: i128);

    fn transfer_collateral(env: Env, user: Address, amount: i128);

    fn withdraw_collateral(env: Env, user: Address, amount: i128);

    fn borrow_synthetic_and_provide_liquidity(env: Env, user: Address, amount: i128);
}
