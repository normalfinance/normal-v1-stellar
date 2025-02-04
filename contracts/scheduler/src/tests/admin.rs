use super::setup::{
    deploy_index_token_factory_contract, deploy_market_factory_contract, deploy_scheduler_contract,
};
use crate::{contract::Scheduler, tests::setup::install_and_deploy_token_contract};

use soroban_sdk::{
    testutils::{arbitrary::std, Address as _},
    vec, Address, Env, String,
};

#[test]
fn scheduler_successfully_inits_itself() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let mut market_factory = deploy_market_factory_contract(&env, &admin);
    let mut index_token_factory = deploy_index_token_factory_contract(&env, &admin);
    let keepers = [Address::generate(&env)];

    let scheduler = deploy_scheduler_contract(
        &env,
        Some(admin.clone()),
        &market_factory.address,
        &index_token_factory.address,
        500,
        200,
    );

    assert_eq!(scheduler.get_admin(), admin);
}

#[test]
#[should_panic(expected = "Insurance: Initialize: initializing contract twice is not allowed")]
fn test_deploying_insurance_twice_should_fail() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let governor = Address::generate(&env);
    let deposit_token = deploy_token_contract(&env, &admin);

    let first = deploy_insurance_contract(
        &env,
        admin.clone(),
        governor.clone(),
        &deposit_token.address,
    );

    first.initialize(
        &admin,
        &governor,
        &deposit_token.address,
        &100i128,
        &10u32,
        &String::from_str(&env, "Normal Insurance Fund Stake"),
        &String::from_str(&env, "NIFS"),
    );
}

#[test]
fn scheduler_successfully_updates_config() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let mut market_factory = deploy_market_factory_contract(&env, &admin);
    let mut index_token_factory = deploy_index_token_factory_contract(&env, &admin);
    let keepers = [Address::generate(&env)];

    let scheduler = deploy_scheduler_contract(
        &env,
        Some(admin.clone()),
        &market_factory.address,
        &index_token_factory.address,
        500,
        200,
    );

    assert_eq!(scheduler.get_admin(), admin);
}

#[test]
#[should_panic(expected = "Scheduler: You are not authorized!")]
fn scheduler_fails_to_update_config_if_not_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let mut market_factory = deploy_market_factory_contract(&env, &admin);
    let mut index_token_factory = deploy_index_token_factory_contract(&env, &admin);

    let scheduler = deploy_scheduler_contract(
        &env,
        Some(admin.clone()),
        &market_factory.address,
        &index_token_factory.address,
        500,
        200,
    );

    scheduler.update_config(user.clone(), "", "", "", "");
}

#[test]
fn scheduler_successfully_updates_keepers() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let mut market_factory = deploy_market_factory_contract(&env, &admin);
    let mut index_token_factory = deploy_index_token_factory_contract(&env, &admin);

    let scheduler = deploy_scheduler_contract(
        &env,
        Some(admin.clone()),
        &market_factory.address,
        &index_token_factory.address,
        500,
        200,
    );

    let keepers_to_add = [Address::generate(&env)];
    let keepers_to_remove = [Address::generate(&env)];

    scheduler.update_keepers(admin.clone(), keepers_to_add, keepers_to_remove);

    let config = scheduler.query_config();

    assert_eq!(config.config.keepers, []);
}

#[test]
#[should_panic(expected = "Scheduler: You are not authorized!")]
fn scheduler_fails_to_update_keepers_if_not_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let mut market_factory = deploy_market_factory_contract(&env, &admin);
    let mut index_token_factory = deploy_index_token_factory_contract(&env, &admin);

    let scheduler = deploy_scheduler_contract(
        &env,
        Some(admin.clone()),
        &market_factory.address,
        &index_token_factory.address,
        500,
        200,
    );

    scheduler.update_keepers(user.clone(), [], []);
}

#[test]
fn scheduler_successfully_distributes_protocol_fees() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let mut market_factory = deploy_market_factory_contract(&env, &admin);
    let mut index_token_factory = deploy_index_token_factory_contract(&env, &admin);
    let keepers = [Address::generate(&env)];

    let scheduler = deploy_scheduler_contract(
        &env,
        Some(admin.clone()),
        &market_factory.address,
        &index_token_factory.address,
        500,
        200,
    );

    assert_eq!(scheduler.get_admin(), admin);
}
