#![no_std]

mod admin;
mod allowance;
mod balance;
mod contract;
mod errors;
mod metadata;
mod storage_types;
mod test;

pub use crate::contract::TokenClient;
