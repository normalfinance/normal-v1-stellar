use crate::contract::{SynthMarketFactory, SynthMarketFactoryClient};
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env, String};
pub const ONE_DAY: u64 = 86400;
const TOKEN_WASM: &[u8] =
    include_bytes!("../../../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm");

#[allow(clippy::too_many_arguments)]
pub fn install_synth_market_wasm(env: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_synth_market.wasm"
    );
    env.deployer().upload_contract_wasm(WASM)
}

pub fn deploy_synth_market_factory_contract<'a>(
    env: &Env,
    admin: impl Into<Option<Address>>,
    governor: impl Into<Option<Address>>,
) -> SynthMarketFactoryClient<'a> {
    let admin = admin.into().unwrap_or(Address::generate(env));
    let factory = SynthMarketFactoryClient::new(env, &env.register(SynthMarketFactory, ()));

    let paused_operations = vec![];

    let synth_market_wasm_hash = install_synth_market_wasm(env);

    factory.initialize(&admin, &governor, &synth_market_wasm_hash);

    factory
}
