#![no_std]

pub mod constants;
mod contract;
mod errors;
mod interfaces;
mod storage;
mod storage_types;

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
