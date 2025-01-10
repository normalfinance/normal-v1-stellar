#![no_std]

mod contract;
mod error;
mod storage;
mod utils;

pub mod band_std_reference {
    soroban_sdk::contractimport!(file = "../../dist/std_reference.wasm");
}

pub mod reflector_price_oracle {
    soroban_sdk::contractimport!(file = "../../dist/reflector_price_oracle.wasm");
}

#[cfg(test)]
mod tests;
