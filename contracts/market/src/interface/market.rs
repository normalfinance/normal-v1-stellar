use normal::{ oracle::OracleSource, types::SynthTier };
use soroban_sdk::{ contractclient, Address, BytesN, Env, String, Vec };

use crate::state::market::{ MarketOperation, MarketParams, MarketStatus };

#[contractclient(name = "MarketClient")]
pub trait MarketTrait {
    // ################################################################
    //                             Admin
    // ################################################################
    fn initialize(
        env: Env,
        sender: Address,
        params: MarketParams,
        token_wasm_hash: BytesN<32>,
        synth_token_name: String,
        synth_token_symbol: String
    );

    fn initialize_shutdown(env: Env, sender: Address, expiry_ts: u64);

    // fn delete(env: Env, sender: Address);

    fn update_paused_operations(env: Env, sender: Address, operations: Vec<MarketOperation>);

    fn update_debt_limit(
        env: Env,
        sender: Address,
        debt_floor: Option<u32>,
        debt_ceiling: Option<u128>
    );

    fn extend_expiry_ts(env: Env, sender: Address, expiry_timestamp: i64);

    fn update_margin_config(
        env: Env,
        sender: Address,
        margin_ratio_initial: u32,
        margin_ratio_maintenance: u32,
        imf_factor: Option<u32>
    );

    fn update_liquidation_config(
        env: Env,
        sender: Address,
        liquidator_fee: u32,
        if_liquidation_fee: u32,
        liquidation_penalty: Option<u32>
    );

    fn update_name(env: Env, sender: Address, name: String);

    fn update_status(env: Env, sender: Address, status: MarketStatus);

    fn update_synth_tier(env: Env, sender: Address, synth_tier: SynthTier);

    fn reset_oracle_twap(env: Env, sender: Address);

    // ################################################################
    //                             Keeper
    // ################################################################

    fn update_oracle_twap(env: Env, keeper: Address);

    fn update_oracle(env: Env, keeper: Address, oracle: Address, oracle_source: OracleSource);

    fn freeze_oracle(env: Env, emergency_oracle: Address) {}

    // fn lend_collateral(e: Env, fee_rate: u128);

    // fn unlend_collateral(e: Env, fee_rate: u128);

    // fn liquidate_position(e: Env, fee_rate: u128);

    // fn resolve_position_bankruptcy(e: Env, fee_rate: u128);

    // ################################################################
    //                             User
    // ################################################################

    fn deposit_collateral(env: Env, sender: Address, amount: i128);

    fn withdraw_collateral(env: Env, sender: Address, amount: i128);

    /// Mints synthetic tokens against deposited collateral and automatically
    /// provides the minted tokens and respective amount of collateral as
    /// liquidity to the Protocol Pool liquidity position.
    ///
    /// # Arguments
    ///
    /// * `env` - The path to the file.
    /// * `sender` - The path to the file.
    /// * `amount` - The path to the file.
    ///
    fn borrow_synth(env: Env, sender: Address, amount: i128);
}
