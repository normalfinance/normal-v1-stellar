use normal::oracle::OracleSource;
use soroban_sdk::{contractclient, Address, Env, Vec};

use crate::state::liquidity_position::LiquidityPositionUpdate;

#[contractclient(name = "PoolClient")]
pub trait PoolTrait {
    fn initialize_tick_array(env: Env, sender: Address, start_tick_index: i32);

    #[allow(clippy::too_many_arguments)]
    fn update_pool(
        env: Env,
        sender: Address,
        fee_rate: Option<i64>,
        protocol_fee_rate: Option<i64>,
        max_allowed_slippage_bps: Option<i64>,
        max_allowed_variance_bps: Option<i64>,
    );

    fn initialize_reward(
        env: Env,
        sender: Address,
        reward_token: Address,
        initial_balance: i128,
        emissions_per_second_x64: u128,
    );

    fn set_reward_emissions(
        env: Env,
        sender: Address,
        reward_token: Address,
        emissions_per_second_x64: u128,
    );

    fn set_reward_authority(
        env: Env,
        sender: Address,
        reward_token: Address,
        new_reward_authority: Address,
    );

    fn reset_oracle_twap(env: Env, sender: Address);

    fn update_oracle_twap(env: Env, sender: Address);

    // ################################################################
    //                          Super Keeper
    // ################################################################

    fn update_oracle(env: Env, sender: Address, oracle: Address, oracle_source: OracleSource);

    fn update_oracle_freeze(env: Env, sender: Address, frozen: bool);

    // ################################################################
    //                             User
    // ################################################################

    fn create_position(env: Env, sender: Address, tick_lower_index: i32, tick_upper_index: i32);

    fn modify_position(
        env: Env,
        sender: Address,
        position_ts: u64,
        update: LiquidityPositionUpdate,
    );

    fn close_position(env: Env, sender: Address, position_ts: u64);

    fn increase_liquidity(
        env: Env,
        sender: Address,
        position_ts: u64,
        liquidity_amount: u128,
        token_max_a: u64,
        token_max_b: u64,
        tick_array_lower_index: i32,
        tick_array_upper_index: i32,
    );

    fn decrease_liquidity(
        env: Env,
        sender: Address,
        position_ts: u64,
        liquidity_amount: u128,
        token_max_a: u64,
        token_max_b: u64,
        tick_array_lower_index: i32,
        tick_array_upper_index: i32,
    );

    fn swap(
        env: Env,
        sender: Address,
        amount: i128,
        other_amount_threshold: u64,
        sqrt_price_limit: u128,
        amount_specified_is_input: bool,
        a_to_b: bool, // Zero for one
        tick_array_indexes: Vec<i32>,
    );

    fn collect_fees(env: Env, sender: Address, position_ts: u64);

    fn collect_reward(env: Env, sender: Address, reward_token: Address, position_ts: u64);

    // ################################################################
    //                             Queries
    // ################################################################

    // Returns  the total amount of LP tokens and assets in a specific pool
    // fn query_pool_info(env: Env) -> PoolResponse;

    // Simulate swap transaction
    // fn simulate_swap(env: Env, offer_asset: Address, sell_amount: i128) -> SimulateSwapResponse;

    // fn query_share(env: Env, amount: i128) -> (Asset, Asset);

    // fn query_total_issued_lp(env: Env) -> i128;
}
