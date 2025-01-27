#![no_std]

mod constants;
mod contract;
mod controller;
mod events;
mod math;
mod storage;
mod synth_market;

pub mod token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

#[cfg(test)]
mod tests;
