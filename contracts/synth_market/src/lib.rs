#![no_std]

mod constants;
mod contract;
mod storage;

pub mod token_contract {
    // The import will code generate:
    // - A ContractClient type that can be used to invoke functions on the contract.
    // - Any types in the contract that were annotated with #[contracttype].
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
}

fn generate_market_symbol(e: &Env, market_name: &str) -> Symbol {
    Symbol::new(&e, market_name)
}

fn publish_updated_event<T>(e: &Env, sub_topic: &Symbol, data: T) where T: IntoVal<Env, Val> {
    e.events().publish(
        (REFLECTOR, symbol_short!("triggers"), symbol_short!("updated"), sub_topic),
        data
    );
}

#[cfg(test)]
mod tests;
