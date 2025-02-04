use super::setup::{ install_index_token_contract, install_token_wasm };
use crate::{
    tests::setup::{
        deploy_index_token_factory_contract,
        generate_index_token_init_info,
        index_token_contract,
    },
};

use soroban_sdk::{ testutils::{ arbitrary::std, Address as _ }, vec, Address, Env, String };

#[test]
fn factory_successfully_inits_itself() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let governor = Address::generate(&env);

    let factory = deploy_index_token_factory_contract(
        &env,
        Some(admin.clone()),
        Some(governor.clone())
    );

    assert_eq!(factory.get_admin(), admin);
}

#[test]
#[should_panic(expected = "Factory: Initialize: initializing contract twice is not allowed")]
fn test_deploy_factory_twice_should_fail() {
    let env = Env::default();
    env.mock_all_auths();
    env.cost_estimate().budget().reset_unlimited();

    let admin = Address::generate(&env);
    let governor = Address::generate(&env);

    let auth_user = Address::generate(&env);
    let index_token_wasm_hash = install_index_token_contract(&env);

    let factory = deploy_index_token_factory_contract(&env, admin.clone(), governor.clone());

    factory.initialize(&admin, &index_token_wasm_hash);
    factory.initialize(&admin, &index_token_wasm_hash);
}

#[test]
fn factory_successfully_inits_market() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let governor = Address::generate(&env);
    let mut token1_admin = Address::generate(&env);
    let mut token2_admin = Address::generate(&env);
    let user = Address::generate(&env);

    let mut token1 = Address::generate(&env);
    let mut token2 = Address::generate(&env);

    env.mock_all_auths();
    env.cost_estimate().budget().reset_unlimited();

    if token2 < token1 {
        std::mem::swap(&mut token1, &mut token2);
        std::mem::swap(&mut token1_admin, &mut token2_admin);
    }

    let factory = deploy_index_token_factory_contract(
        &env,
        Some(admin.clone()),
        Some(governor.clone())
    );
    assert_eq!(factory.get_admin(), admin);

    let index_params = generate_index_token_init_info(
        token1.clone(),
        token2.clone(),
        Address::generate(&env),
        admin.clone(),
        user.clone()
    );

    factory.create_index_token(&admin, &100, &index_params);
    let index_token_contract_addr = factory.query_markets().get(0).unwrap();

    let first_index_token_contract = index_token_contract::Client::new(
        &env,
        &index_token_contract_addr
    );

    assert_eq!(first_index_token_contract.query_index(), index_token_contract::Index {
        fee_recipient: user,
        max_allowed_slippage_bps: 5_000,
        // ...
    });
}

#[test]
fn factory_successfully_updates_config() {}

#[test]
fn factory_successfully_updates_wasm_hashes() {}
