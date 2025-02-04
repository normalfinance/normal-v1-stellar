use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::contract::{Scheduler, SchedulerClient};

pub fn deploy_scheduler_contract<'a>(
    env: &Env,
    admin: impl Into<Option<Address>>,
    market_factory: &Address,
    index_factory: &Address,
    protocol_fee_bps: impl Into<Option<i64>>,
    keeper_fee_bps: impl Into<Option<i64>>,
) -> SchedulerClient<'a> {
    let admin = admin.into().unwrap_or(Address::generate(env));
    let scheduler = SchedulerClient::new(env, &env.register(Scheduler, ()));

    scheduler.initialize(
        &admin,
        market_factory,
        index_factory,
        protocol_fee_bps,
        keeper_fee_bps,
    );

    scheduler
}
