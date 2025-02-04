use soroban_sdk::{Address, Env, Symbol};

use crate::storage::ScheduleType;

// use crate::storage::{Asset, Schedule};

pub struct SchedulerEvents {}

impl SchedulerEvents {
    /// Emitted when a the Scheduler is initialized
    ///
    /// - topics - `["initialize", admin: Address]`
    /// - data - ()
    pub fn initialize(env: &Env, admin: Address) {
        let topics = (Symbol::new(env, "initialize"), admin);
        env.events().publish(topics, ());
    }

    /// Emitted when a new schedule is created
    ///
    /// - topics - `["new_schedule", user: Address, ts: u64]`
    /// - data - `[schedule_type: ScheduleType, quote_asset: Address, target_contract_address: Address]`
    pub fn new_schedule(
        env: &Env,
        user: Address,
        ts: u64,
        schedule_type: ScheduleType,
        quote_asset: Address,
        target_contract_address: Address,
    ) {
        let topics = (Symbol::new(env, "new_schedule"), user, ts);
        env.events().publish(
            topics,
            (schedule_type, quote_asset, target_contract_address),
        );
    }

    /// Emitted when a user makes a deposit
    ///
    /// - topics - `["deposit", user: Address]`
    /// - data - `[asset: Address, amount: i128]`
    pub fn deposit(env: &Env, user: Address, asset: Address, amount: i128) {
        let topics = (Symbol::new(env, "deposit"), user);
        env.events().publish(topics, (asset, amount));
    }

    /// Emitted when a user withdraws assets from the schedule address
    ///
    /// - topics - `["withdrawal", user: Address]`
    /// - data - `[asset: Asset, amount: i128]`
    pub fn withdrawal(env: &Env, user: Address, asset: Address, amount: i128) {
        let topics = (Symbol::new(env, "withdrawal"), user);
        env.events().publish(topics, (asset, amount));
    }

    /// Emitted when a keeper executes a schedule order
    ///
    /// - topics - `["order_execution", keeper: Address, user: Address]`
    /// - data - (schedule_timestamp: u64)
    pub fn order_execution(env: &Env, keeper: Address, user: Address, schedule_timestamp: u64) {
        let topics = (Symbol::new(env, "order_execution"), keeper, user);
        env.events().publish(topics, schedule_timestamp);
    }

    /// Emitted when a user deletes a schedule
    ///
    /// - topics - `["delete_schedule", user: Address]`
    /// - data - [schedule_timestamp: u64]
    pub fn delete_schedule(env: &Env, user: Address, schedule_timestamp: u64) {
        let topics = (Symbol::new(env, "delete_schedule"), user);
        env.events().publish(topics, schedule_timestamp);
    }
}
