extern crate std;

use pretty_assertions::assert_eq;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger},
    vec, Address, Env, IntoVal, String, Symbol, Vec,
};

use super::setup::{deploy_scheduler_contract, deploy_token_contract};

#[test]
fn execute_schedule() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let stake_asset = deploy_token_contract(&env, &admin);

    let scheduler = deploy_scheduler_contract(
        &env,
        admin.clone(),
        governor.clone(),
        &gov_token.address,
        &stake_asset.address,
    );
}

#[test]
#[should_panic(expected = "Insurance: Initialize: initializing contract twice is not allowed")]
fn execute_invalid_schedule_should_fail() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let stake_asset = deploy_token_contract(&env, &admin);

    let scheduler = deploy_scheduler_contract(
        &env,
        admin.clone(),
        governor.clone(),
        &gov_token.address,
        &stake_asset.address,
    );

    // ...
}

#[test]
fn collect_fees() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let stake_asset = deploy_token_contract(&env, &admin);

    let scheduler = deploy_scheduler_contract(
        &env,
        admin.clone(),
        governor.clone(),
        &gov_token.address,
        &stake_asset.address,
    );
}
