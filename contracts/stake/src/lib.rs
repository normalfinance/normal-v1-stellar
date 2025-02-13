#![no_std]

#[cfg(any(test, feature = "testutils"))]
extern crate std;

mod contract;
mod errors;
mod events;
mod staking;
mod storage;

pub mod token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

pub use contract::*;
