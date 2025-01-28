use soroban_sdk::{contracttype, Vec};

use crate::storage::{Config, Schedule};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigResponse {
    pub config: Config,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledResponse {
    pub schedules: Vec<Schedule>,
}
