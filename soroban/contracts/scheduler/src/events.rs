use soroban_sdk::{ Address, Env, String, Symbol };

use crate::types::{ ProposalAction, VoteCount };

pub struct SchedulerEvents {}

impl SchedulerEvents {
    /// Emitted when a schedule is created for trading a single asset
    ///
    /// Note: Asset support is limited to active synth markets.
    /// 3rd party AMM support will be added in v2.
    ///
    /// - topics - `["new_asset_schedule", schedule_id: u32, creator: Address]`
    /// - data - `[amm_id: Address, params: ScheduleData]`
    pub fn new_asset_schedule(
        e: &Env,
        schedule_id: u32,
        creator: Address,
        amm_id: Address,
        params: ScheduleData
    ) {
        let topics = (Symbol::new(&e, "new_asset_schedule"), schedule_id, creator);
        e.events().publish(topics, (title, desc, action, vote_start, vote_end));
    }

    /// Emitted when a schedule is created for trading a cryto
    ///
    /// Note: Indexes do not have accompanying AMMs. Instead index tokens
    /// are minted/redeemed directly from the contract.
    ///
    /// - topics - `["new_index_schedule", schedule_id: u32, creator: Address]`
    /// - data - `[index_id: u32, params: ScheduleData]`
    pub fn new_index_schedule(
        e: &Env,
        schedule_id: u32,
        creator: Address,
        index_id: u32,
        params: ScheduleData
    ) {
        let topics = (Symbol::new(&e, "new_index_schedule"), schedule_id, creator);
        e.events().publish(topics, (index_id, params));
    }

    /// Emitted when a user deposits assets into the schedule address
    ///
    /// - topics - `["deposit", schedule_id: u32]`
    /// - data - `[user: Address, asset: Asset, amount: u128]`
    pub fn deposit(e: &Env, schedule_id: u32, user: Address, asset: Asset, amount: u128) {
        let topics = (Symbol::new(&e, "deposit"), schedule_id);
        e.events().publish(topics, (user, asset, amount));
    }

    /// Emitted when a keeper executes a schedule order
    ///
    /// - topics - `["order_execution", keeper: Address, schedule_id: u32]`
    /// - data - ()
    pub fn order_execution(e: &Env, keeper: Address, schedule_id: u32) {
        let topics = (Symbol::new(&e, "order_execution"), keeper, schedule_id);
        e.events().publish(topics, ());
    }

    /// Emitted when a user updates a schedule
    ///
    /// - topics - `["modify_schedule", schedule_id: u32]`
    /// - data - `[]`
    pub fn modify_schedule(e: &Env, schedule_id: u32) {
        let topics = (Symbol::new(&e, "modify_schedule"), schedule_id);
        e.events().publish(topics, ());
    }

    /// Emitted when a user withdraws assets from the schedule address
    ///
    /// - topics - `["withdrawal", schedule_id: u32]`
    /// - data - `[user: Address, asset: Asset, amount: u128]`
    pub fn withdrawal(e: &Env, schedule_id: u32, user: Address, asset: Asset, amount: u128) {
        let topics = (Symbol::new(&e, "withdrawal"), schedule_id);
        e.events().publish(topics, (user, asset, amount));
    }

    /// Emitted when a user deletes a schedule
    ///
    /// - topics - `["delete_schedule", schedule_id: u32]`
    /// - data - ()
    pub fn delete_schedule(e: &Env, schedule_id: u32) {
        let topics = (Symbol::new(&e, "delete_schedule"), schedule_id);
        e.events().publish(topics, ());
    }
}
