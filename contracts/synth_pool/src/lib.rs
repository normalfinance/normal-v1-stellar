#![no_std]

mod contract;
mod controller;
mod errors;
mod events;
pub mod math;
mod pool;
mod position;
mod reward;
mod storage;
mod tick;
mod tick_array;
mod utils;
mod market;

pub mod token_contract {
    // The import will code generate:
    // - A ContractClient type that can be used to invoke functions on the contract.
    // - Any types in the contract that were annotated with #[contracttype].
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

#[cfg(test)]
mod tests;
