#![no_std]

mod buffer;
mod contract;
mod controller;
pub mod errors;
mod events;
mod insurance_fund;
mod interfaces;
mod math;
mod storage;

pub mod token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

// pub mod pool_contract {
//     soroban_sdk::contractimport!(
//         file = "../../target/wasm32-unknown-unknown/release/aqua_pool.wasm"
//     );
// }

#[cfg(test)]
mod tests;
