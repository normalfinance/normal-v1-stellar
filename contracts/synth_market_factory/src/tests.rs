use soroban_sdk::{testutils::Address as _, Address, Env};

use self::setup::{deploy_synth_market_factory_contract, install_synth_market_wasm};

mod config;
mod setup;

mod queries;
#[test]
#[should_panic(
    expected = "Synth Market Factory: Initialize: initializing contract twice is not allowed"
)]
fn test_deploy_factory_twice_should_fail() {
    let env = Env::default();
    env.mock_all_auths();
    env.cost_estimate().budget().reset_unlimited();

    let admin = Address::generate(&env);
    let governor = Address::generate(&env);

    let synth_market_wasm_hash = install_synth_market_wasm(&env);

    let factory = deploy_synth_market_factory_contract(&env, admin.clone(), governor.clone());

    factory.initialize(&admin, &synth_market_wasm_hash);
    // factory.initialize(&admin, &index_wasm_hash, &index_token_wasm_hash, &[], 500, 300, &oracle);
}
