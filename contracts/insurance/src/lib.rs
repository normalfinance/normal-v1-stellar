#![no_std]

// Traits
mod buffer;
mod insurance_fund;

mod contract;
mod controller;
mod events;
mod interfaces;
mod math;
mod storage;

pub mod token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

pub mod pool_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/aqua_pool.wasm"
    );
}

#[cfg(test)]
mod tests;
