#![no_std]

pub mod constants;
pub mod error;
pub mod macros;
pub mod math;
pub mod oracle;
pub mod types;
pub mod utils;

pub mod band_std_reference {
    soroban_sdk::contractimport!(file = "../../dist/std_reference.wasm");
}
