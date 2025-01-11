#![no_std]

mod contract;
mod errors;
mod events;
mod index;
mod storage;
mod utils;

#[allow(clippy::too_many_arguments)]
pub mod amm_contract {
    // The import will code generate:
    // - A ContractClient type that can be used to invoke functions on the contract.
    // - Any types in the contract that were annotated with #[contracttype].
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_amm.wasm"
    );
}

pub mod index_token_contract {
    // The import will code generate:
    // - A ContractClient type that can be used to invoke functions on the contract.
    // - Any types in the contract that were annotated with #[contracttype].
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/index_token_contract.wasm"
    );
}

#[allow(clippy::too_many_arguments)]
pub mod amm_factory_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/amm_factory.wasm"
    );
}

pub mod token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

#[cfg(test)]
mod tests;
