use soroban_sdk::{contractclient, Address, Env, Vec};

use crate::{
    msg::{ConfigResponse, ScheduledResponse},
    storage::ScheduleParams,
};

#[contractclient(name = "ScheduleClient")]
pub trait SchedulerTrait {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        synth_market_factory_address: Address,
        index_factory_address: Address,
        protocol_fee_bps: i64,
        keeper_fee_bps: i64,
    );

    #[allow(clippy::too_many_arguments)]
    fn update_config(
        env: Env,
        sender: Address,
        synth_market_factory_address: Option<Address>,
        index_factory_address: Option<Address>,
        protocol_fee_bps: Option<i64>,
        keeper_fee_bps: Option<i64>,
    );

    fn update_keepers(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>);

    fn collect_protocol_fees(env: Env, sender: Address, to: Address);

    // ################################################################
    //                             KEEPER
    // ################################################################

    fn execute_schedule(env: Env, sender: Address, user: Address, schedule_timestamp: u64);

    fn collect_keeper_fees(env: Env, sender: Address);

    // ################################################################
    //                             USER
    // ################################################################

    fn deposit(env: Env, sender: Address, asset: Address, amount: i128);

    fn withdraw(env: Env, sender: Address, asset: Address, amount: i128);

    fn create_schedule(env: Env, sender: Address, params: ScheduleParams);

    fn delete_schedule(env: Env, sender: Address, schedule_timestamp: u64);

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_config(env: Env) -> ConfigResponse;

    fn query_admin(env: Env) -> Address;

    fn query_scheduled(env: Env, address: Address) -> ScheduledResponse;
}
