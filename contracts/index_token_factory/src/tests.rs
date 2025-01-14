use soroban_sdk::{testutils::Address as _, Address, Env};

use self::setup::{deploy_index_factory_contract, install_index_token_wasm, install_index_wasm};

mod config;
mod setup;

mod queries;
#[test]
#[should_panic(expected = "Index Factory: Initialize: initializing contract twice is not allowed")]
fn test_deploy_factory_twice_should_fail() {
    let env = Env::default();
    env.mock_all_auths();
    env.cost_estimate().budget().reset_unlimited();

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    let auth_user = Address::generate(&env);
    let index_wasm_hash = install_index_wasm(&env);
    let index_token_wasm_hash = install_index_token_wasm(&env);

    let factory = deploy_index_factory_contract(&env, admin.clone(), oracle.clone());

    factory.initialize(
        &admin,
        &index_wasm_hash,
        &index_token_wasm_hash,
        &[],
        500,
        300,
        &oracle,
    );
    factory.initialize(
        &admin,
        &index_wasm_hash,
        &index_token_wasm_hash,
        &[],
        500,
        300,
        &oracle,
    );
}
