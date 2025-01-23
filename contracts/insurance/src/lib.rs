#![no_std]

// Traits
mod buffer;
mod insurance_fund;

mod contract;
mod events;
mod controller;
mod storage;
mod math;
mod interfaces;

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
