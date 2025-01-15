#![no_std]

// Traits
mod buffer;
mod insurance_fund;

mod contract;
mod events;
mod controller;
mod storage;
mod math;

pub mod token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

#[cfg(test)]
mod tests;
