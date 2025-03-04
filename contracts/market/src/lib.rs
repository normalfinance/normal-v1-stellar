#![no_std]

mod contract;
pub mod controller;
mod errors;
mod events;
mod interface;
pub mod math;
pub mod state;
mod storage;
mod utils;
mod validation;

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
