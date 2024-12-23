use soroban_sdk::{ Address, Env };

use crate::storage_types::{ DataKey, Stake };

const SCHEDULE_ID_KEY: &str = "ScheduleId";

//********** Storage Keys **********//

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    // A map of schedule id to schedule data
    Data(u32),
}

/********** Persistent **********/

/// Set the next schedule id and bump if necessary
///
/// ### Arguments
/// * `schedule_id` - The new schedule_id
pub fn set_next_schedule_id(e: &Env, schedule_id: u32) {
    let key = Symbol::new(&e, SCHEDULE_ID_KEY);
    e.storage().persistent().set::<Symbol, u32>(&key, &schedule_id);
    e.storage().persistent().extend_ttl(&key, LEDGER_THRESHOLD, LEDGER_BUMP);
}

/// Get the current schedule id
pub fn get_next_schedule_id(e: &Env) -> u32 {
    let key = Symbol::new(&e, SCHEDULE_ID_KEY);
    get_persistent_default::<Symbol, u32>(&e, &key, 0_u32, LEDGER_THRESHOLD, LEDGER_BUMP)
}

/********** Temporary **********/

/***** Schedule Data *****/

// Get the schedule data for schedule at `schedule_id`
///
/// ### Arguments
/// * `schedule_id` - The schedule status id
pub fn get_schedule_data(e: &Env, schedule_id: u32) -> Option<ScheduleData> {
    let key = DataKey::Data(schedule_id);
    e.storage().temporary().get::<DataKey, ScheduleData>(&key)
}

/// Set the schedule data for schedule at `schedule_id`.
///
/// Does not perform a ledger ttl bump.
///
/// ### Arguments
/// * `schedule_id` - The schedule id
pub fn set_schedule_data(e: &Env, schedule_id: u32, schedule_data: &ScheduleData) {
    let key = DataKey::Data(schedule_id);
    e.storage().temporary().set::<DataKey, ScheduleData>(&key, &schedule_data);
}

/// Create the schedule status for schedule at `schedule_id` and bump
/// it for the life of the schedule.
///
/// ### Arguments
/// * `schedule_id` - The schedule id
pub fn create_schedule_data(e: &Env, schedule_id: u32, schedule_data: &ScheduleData) {
    let key = DataKey::Data(schedule_id);
    e.storage().temporary().set::<DataKey, ScheduleData>(&key, &schedule_data);
    e.storage().temporary().extend_ttl(&key, LEDGER_BUMP, LEDGER_BUMP);
}
