use normal::{
    oracle::OracleSource,
    types::market::{MarketInfo, MarketParams, MarketResponse, SynthTier},
};
use soroban_sdk::{contractclient, Address, BytesN, Env, String, Vec};

use crate::state::market::{Market, MarketOperation, MarketStatus};

#[contractclient(name = "MarketClient")]
pub trait MarketTrait {
    // ################################################################
    //                             Admin
    // ################################################################
    fn initialize(
        env: Env,
        token_wasm_hash: BytesN<32>,
        params: MarketParams,
        factory_addr: Address,
        insurance_addr: Address,
    );

    fn initialize_shutdown(env: Env, sender: Address, expiry_ts: u64);

    // fn delete(env: Env, sender: Address);

    fn update_paused_operations(env: Env, sender: Address, operations: Vec<MarketOperation>);

    fn update_debt_limit(
        env: Env,
        sender: Address,
        debt_floor: Option<u32>,
        debt_ceiling: Option<u128>,
    );

    fn extend_expiry_ts(env: Env, sender: Address, expiry_ts: u64);

    fn update_margin_config(
        env: Env,
        sender: Address,
        margin_ratio_initial: u32,
        margin_ratio_maintenance: u32,
        imf_factor: Option<u32>,
    );

    fn update_liquidation_config(
        env: Env,
        sender: Address,
        liquidator_fee: u32,
        if_liquidation_fee: u32,
        liquidation_penalty: Option<u32>,
    );

    fn update_name(env: Env, sender: Address, name: String);

    fn update_status(env: Env, sender: Address, status: MarketStatus);

    fn update_synth_tier(env: Env, sender: Address, synth_tier: SynthTier);

    fn update_emissions(env: Env, sender: Address, amount: u128, deadline: u64);

    // ################################################################
    //                          Super Keeper
    // ################################################################

    fn update_collateral_oracle(
        env: Env,
        sender: Address,
        oracle: Address,
        oracle_source: OracleSource,
    );

    fn update_collateral_oracle_freeze(env: Env, sender: Address, frozen: bool);

    // ################################################################
    //                             Keeper
    // ################################################################

    /// Revenue is settled to 2 places: Normal Insurance and the Governor.
    /// A portion of revenue is sent to the Normal Buffer is filled to its max balance,
    /// while any overflow goes to the Insurance Fund.
    ///
    /// The remaining majority of revenue is sent to the Governor to be
    /// distributed to voters/
    ///
    fn settle_revenue(env: Env, sender: Address);

    /// Deposits excess/idle collateral to lending/borrowing markets
    /// to earn addiotional yield payed to deposits.
    ///
    ///
    fn lend_collateral(env: Env, sender: Address);

    /// Recalls a necessary amount of collateral from lending/borrowing
    /// markets in the event risk needs lowering.
    ///
    ///
    fn unlend_collateral(env: Env, sender: Address);

    fn liquidate_position(
        env: Env,
        liquidator: Address,
        user: Address,
        max_base_asset_amount: u64,
        limit_price: Option<u64>,
    );

    fn resolve_position_bankruptcy(env: Env, sender: Address);

    // ################################################################
    //                             User
    // ################################################################

    /// 1) Update position
    ///     - Increase balance
    ///     - Increase total_deposits
    /// 2) Update market
    ///     - collateral.balance
    ///     - collateral.token_twap
    ///     - collateral.utilization_twap (c-ratio)
    ///     - last_twap_ts
    ///     - next_deposit_record_id
    /// 3) Receive tokens
    fn deposit_collateral(env: Env, sender: Address, amount: i128, reduce_only: bool);

    /// 1)
    fn withdraw_collateral(env: Env, sender: Address, amount: i128, reduce_only: bool);

    /// Mints synthetic tokens against deposited collateral and automatically
    /// provides the minted tokens and respective amount of collateral as
    /// liquidity to the Protocol Pool liquidity position.
    ///
    /// Validations:
    /// - Market status
    /// - Paused operations
    /// - Position not liquidated
    /// - Debt ceiling and floor
    /// - Mint doesn't exceed max_margin_ratio
    /// - Mint amount under max mint amount
    /// - Check against market.max_position_size
    ///
    /// Update position
    /// Update market
    /// Mint tokens
    /// Provide tokens as LP
    fn borrow_and_increase_liquidity(env: Env, sender: Address, amount: i128);

    fn remove_liquidity_and_repay(env: Env, sender: Address, amount: i128);

    // ################################################################
    //                             Queries
    // ################################################################

    // Returns the configuration structure containing the addresses
    fn query_market(env: Env) -> Market;

    // Returns the address for the pool share token
    fn query_synth_token_address(env: Env) -> Address;

    // Returns the address for the pool stake contract
    fn query_lp_contract_address(env: Env) -> Address;

    // Returns  the total amount of LP tokens and assets in a specific pool
    fn query_market_info(env: Env) -> MarketResponse;

    fn query_market_info_for_factory(env: Env) -> MarketInfo;

    // fn migrate_admin_key(env: Env) -> Result<(), ErrorCode>;
}
