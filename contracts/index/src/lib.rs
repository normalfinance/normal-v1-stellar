#![no_std]

mod contract;
mod errors;
mod events;
mod storage;
mod utils;

pub mod index_token_contract {
    // The import will code generate:
    // - A ContractClient type that can be used to invoke functions on the contract.
    // - Any types in the contract that were annotated with #[contracttype].
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/index_token_contract.wasm"
    );
}

#[cfg(test)]
mod tests;
