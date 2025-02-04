extern crate std;

use normal::constants::{ONE_MILLION_QUOTE, THIRTEEN_DAY};
use pretty_assertions::assert_eq;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger},
    vec, Address, Env, IntoVal, String, Symbol, Vec,
};

use super::setup::{deploy_scheduler_contract, deploy_token_contract};

#[test]
fn deposit() {
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

    env.ledger().with_mut(|li| {
        li.timestamp = ONE_WEEK;
    });

    stake_asset.mint(&user, &10_000);

    scheduler.deposit(&user, &10_000);

    assert_eq!(
        env.auths(),
        [(
            user.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    scheduler.address.clone(),
                    Symbol::new(&env, "deposit"),
                    (&user.clone(), 10_000i128).into_val(&env),
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        stake_asset.address.clone(),
                        symbol_short!("transfer"),
                        (&user, &scheduler.address.clone(), 10_000i128).into_val(&env),
                    )),
                    sub_invocations: std::vec![],
                }],
            },
        ),]
    );

    let schedules = scheduler.query_scheduled(&user);
    assert_eq!(schedules.balances.get(stake_asset.address), 10_000);

    //    ...
}

#[test]
fn withdraw() {
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
#[should_panic(expected = "Insurance: Initialize: initializing contract twice is not allowed")]
fn withdraw_more_than_balance_should_fail() {
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
fn create_schedule() {
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
fn delete_schedule() {
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
