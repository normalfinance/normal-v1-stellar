#![no_std]

mod allowance;
mod balance;
mod contract;
mod events;
mod index_token;
mod metadata;
mod storage;

mod index_factory_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_index_factory_contract.wasm"
    );
}

#[allow(clippy::too_many_arguments)]
pub mod synth_pool_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_synth_pool.wasm"
    );
}

#[allow(clippy::too_many_arguments)]
pub mod synth_market_factory_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_synth_market_factory.wasm"
    );
}

pub mod token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

#[cfg(test)]
mod tests;

pub use crate::contract::IndexTokenClient;
