use soroban_sdk::{contracttype, Address, Map, Vec};

use crate::storage::{Config, Schedule};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigResponse {
    pub config: Config,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledResponse {
    pub balances: Map<Address, i128>,
    pub schedules: Vec<Schedule>,
}
