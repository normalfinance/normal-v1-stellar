#![no_std]

mod pool;
mod contract;
mod errors;
mod events;
mod position;
mod storage;
mod reward;
mod swap;
mod tick_array;
mod tick;
mod utils;
mod math;
mod controller;

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
