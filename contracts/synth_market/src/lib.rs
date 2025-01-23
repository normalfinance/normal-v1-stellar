#![no_std]

mod constants;
mod contract;
mod events;
mod storage;
mod synth_market;
mod controller;
mod math;

pub mod token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

#[cfg(test)]
mod tests;
