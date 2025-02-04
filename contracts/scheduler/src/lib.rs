#![no_std]

mod contract;
mod events;
mod msg;
mod scheduler;
mod storage;

pub mod token_contract {
    // The import will code generate:
    // - A ContractClient type that can be used to invoke functions on the contract.
    // - Any types in the contract that were annotated with #[contracttype].
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

pub mod index_token {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_index_token.wasm"
    );
}

pub mod market {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_market.wasm"
    );
}

#[cfg(test)]
mod tests;
