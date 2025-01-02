use soroban_sdk::{ contractclient, Address, Env, String };

use crate::types::{ Schedule, OrderDirection, Asset };

#[contractclient(name = "ScheduleClient")]
pub trait Schedule {
    fn create_asset_schedule(e: Env, amm_id: Address, params: ScheduleData);

    fn create_index_schedule(e: Env, index_id: u32, params: ScheduleData);

    /// Get a schedule by its id
    ///
    /// Returns None if the schedule does not exist
    ///
    /// ### Arguments
    /// * `schedule_id` - The id of the schedule to get
    fn get_schedule(e: Env, schedule_id: u32) -> Option<Schedule>;

    fn deposit(e: Env, user: Address, asset: Asset, amount: u128);

    /// Execute a proposal. Execution required the proposal has been queued for execution and the timelock has passed.
    ///
    /// ### Arguments
    /// * `proposal_id` - The id of the proposal to execute
    ///
    /// ### Panics
    /// * If the proposal_id is invalid
    /// * If the proposal is not ready to be executed
    fn execute(e: Env, by: Address, schedule_id: u32);

    fn modify(e: Env);

    fn withdraw(e: Env, user: Address, asset: Asset, amount: u128);

    /// Close the voting period for a proposal. Closing a proposal requires the quorum to be reached or the voting
    /// period to have ended. The proposal will be queued for execution if the quorum is reached and the vote passes.
    /// Otherwise, the proposal will be marked as failed.
    ///
    /// ### Arguments
    /// * `proposal_id` - The id of the proposal to close
    ///
    /// ### Panics
    /// * If the proposal_id is invalid
    /// * If the proposal is not ready to be closed
    fn delete(e: Env, schedule_id: u32);
}
