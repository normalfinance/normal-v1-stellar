use normal::types::OrderDirection;
use soroban_sdk::{ contractclient, Address, Env, Vec };

use crate::storage::{ Asset, Schedule, ScheduleType };

#[contractclient(name = "ScheduleClient")]
pub trait SchedulerTrait {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        synth_market_factory_address: Address,
        index_factory_address: Address,
        keeper_accounts: Vec<Address>,
        protocol_fee_bps: i64,
        keeper_fee_bps: i64
    );

    // Allows admin address set during initialization to change some parameters of the
    // configuration
    #[allow(clippy::too_many_arguments)]
    fn update_config(
        env: Env,
        new_admin: Option<Address>,
        synth_market_factory_address: Option<Address>,
        index_factory_address: Option<Address>,
        protocol_fee_bps: Option<i64>,
        keeper_fee_bps: Option<i64>
    );

    fn update_keeper_accounts(
        env: Env,
        sender: Address,
        to_add: Vec<Address>,
        to_remove: Vec<Address>
    );

    fn collect_protocol_fees(env: Env, sender: Address, to: Address);

    // ################################################################
    //                             KEEPER
    // ################################################################

    fn execute_schedule(env: Env, sender: Address, user: Address, schedule_timestamp: u64);

    fn collect_keeper_fees(env: Env, sender: Address, to: Option<Address>);

    // ################################################################
    //                             USER
    // ################################################################

    fn deposit(env: Env, user: Address, asset: Asset, amount: i128);

    fn withdraw(env: Env, user: Address, asset: Asset, amount: i128);

    #[allow(clippy::too_many_arguments)]
    fn create_schedule(
        env: Env,
        user: Address,
        schedule_type: ScheduleType,
        target_contract_address: Address,
        base_asset_amount_per_interval: u64,
        direction: OrderDirection,
        active: bool,
        interval_seconds: u64,
        min_price: Option<u16>,
        max_price: Option<u16>
    );

    // Allows user to change some parameters of a schedule
    #[allow(clippy::too_many_arguments)]
    fn update_schedule(
        env: Env,
        user: Address,
        schedule_timestamp: u64,
        base_asset_amount_per_interval: Option<u64>,
        direction: Option<OrderDirection>,
        active: Option<bool>,
        interval_seconds: Option<u64>,
        total_orders: Option<u16>,
        min_price: Option<u16>,
        max_price: Option<u16>
    );

    fn delete_schedule(env: Env, user: Address, schedule_timestamp: u64);

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_schedules(env: Env) -> Vec<Address>;

    fn query_schedule_details(env: Env, index_address: Address) -> Schedule;

    fn query_all_schedules_details(env: Env) -> Vec<Schedule>;

    fn query_for_schedules_by_address(env: Env, user: Address) -> Vec<Address>;
}
