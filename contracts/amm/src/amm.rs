use soroban_sdk::{ contractclient, Address, BytesN, Env, String };

use crate::{ position::PositionUpdate, storage::{ AMMParams, Config } };

#[contractclient(name = "AMMClient")]
pub trait AMMTrait {
    // Sets the token contract addresses for this pool
    // token_wasm_hash is the WASM hash of the deployed token contract for the pool share token
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        token_wasm_hash: BytesN<32>,
        params: AMMParams,
        protocol_fee_rate: u16,
        share_token_decimals: u32,
        share_token_name: String,
        share_token_symbol: String
    );

    fn initialize_tick_array(env: Env, start_tick_index: i32);

    // Allows admin address set during initialization to change some parameters of the
    // configuration
    #[allow(clippy::too_many_arguments)]
    fn update_config(
        env: Env,
        fee_rate: Option<i64>,
        protocol_fee_rate: Option<i64>,
        max_allowed_slippage_bps: Option<i64>,
        max_allowed_spread_bps: Option<i64>,
        max_allowed_variance_bps: Option<i64>
    );

    fn reset_oracle_twap(env: Env);
    fn update_oracle_twap(env: Env);

    fn initialize_reward(env: Env, reward_index: u8);
    fn set_reward_emissions(env: Env, reward_timestamp: u64, emissions_per_second_x64: u128);

    // ################################################################
    //                             KEEPER
    // ################################################################

    //    ...

    // ################################################################
    //                             USER
    // ################################################################

    fn create_position(env: Env, sender: Address, tick_lower_index: i32, tick_upper_index: i32);

    fn modify_position(env: Env, sender: Address, position_ts: u64, update: PositionUpdate);

    fn increase_liquidity(
        env: Env,
        sender: Address,
        position_timestamp: u64,
        liquidity_amount: u128,
        token_max_a: u64,
        token_max_b: u64
    );

    fn decrease_liquidity(
        env: Env,
        sender: Address,
        position_timestamp: u64,
        liquidity_amount: u128,
        token_max_a: u64,
        token_max_b: u64
    );

    fn close_position(env: Env, sender: Address, position_timestamp: u64);

    fn swap(
        env: Env,
        sender: Address,
        amount: u64,
        other_amount_threshold: u64,
        sqrt_price_limit: u128,
        amount_specified_is_input: bool,
        a_to_b: bool // Zero for one
    );

    fn collect_fees(env: Env, sender: Address, to: Address, position_timestamp: u64);
    fn collect_reward(env: Env, sender: Address, to: Address, reward_timestamp: u64);

    // Queries

    // Returns the configuration structure containing the addresses
    fn query_config(env: Env) -> Config;

    // Returns the address for the pool share token
    fn query_share_token_address(env: Env) -> Address;

    // Returns  the total amount of LP tokens and assets in a specific pool
    fn query_pool_info(env: Env) -> PoolResponse;

    // Simulate swap transaction
    fn simulate_swap(env: Env, offer_asset: Address, sell_amount: i128) -> SimulateSwapResponse;

    fn query_share(env: Env, amount: i128) -> (Asset, Asset);

    fn query_total_issued_lp(env: Env) -> i128;
}
