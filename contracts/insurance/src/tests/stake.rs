extern crate std;

use normal::constants::{ONE_MILLION_QUOTE, THIRTEEN_DAY};
use pretty_assertions::assert_eq;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger},
    vec, Address, Env, IntoVal, String, Symbol, Vec,
};

use super::setup::{deploy_insurance_contract, deploy_token_contract, install_token_wasm};

use crate::{
    math::insurance::vault_amount_to_if_shares,
    storage::{InsuranceFund, Stake},
    tests::setup::ONE_DAY,
};

#[test]
fn initialize_insurance_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let governor_contract = Address::generate(&env);
    let deposit_token = deploy_token_contract(&env, &admin);

    let insurance =
        deploy_insurance_contract(&env, &admin, &governor_contract, &deposit_token.address);

    let insurance_fund = insurance.query_insurance_fund();

    assert_eq!(
        insurance_fund,
        InsuranceFund {
            deposit_token: deposit_token.address,
            stake_token: insurance_fund.stake_token.clone(), // unsure how to test this since it's created in the contract
            unstaking_period: THIRTEEN_DAY,
            revenue_settle_period: THIRTEEN_DAY,
            max_insurance: ONE_MILLION_QUOTE,
            paused_operations: Vec::new(&env),
            total_shares: 0,
            user_shares: 0,
            shares_base: 0,
            last_revenue_settle_ts: 0,
            total_factor: 0,
            user_factor: 0,
        }
    );

    let response = insurance.query_admin();
    assert_eq!(response, admin);
}

#[test]
#[should_panic(expected = "Insurance: Initialize: initializing contract twice is not allowed")]
fn test_deploying_insurance_twice_should_fail() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let governor_contract = Address::generate(&env);
    let deposit_token = deploy_token_contract(&env, &admin);

    let first = deploy_insurance_contract(&env, &admin, &governor_contract, &deposit_token.address);

    let token_wasm_hash = install_token_wasm(&env);

    first.initialize(
        &admin,
        &governor_contract,
        &deposit_token.address,
        &token_wasm_hash,
        &10u32,
        &String::from_str(&env, "Normal Insurance Fund Stake"),
        &String::from_str(&env, "NIFS"),
        &1_000_000i128,
    );
}

#[test]
fn add_stake() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let governor_contract = Address::generate(&env);
    let user = Address::generate(&env);

    let deposit_token = deploy_token_contract(&env, &admin);

    let insurance =
        deploy_insurance_contract(&env, &admin, &governor_contract, &deposit_token.address);

    // env.ledger().with_mut(|li| {
    //     li.timestamp = ONE_WEEK;
    // });

    deposit_token.mint(&user, &10_000);

    let insurance_fund_before = insurance.query_insurance_fund();

    insurance.add_if_stake(&user, &10_000);

    assert_eq!(
        env.auths(),
        [(
            user.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    insurance.address.clone(),
                    Symbol::new(&env, "add_if_stake"),
                    (&user.clone(), 10_000i128).into_val(&env),
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        deposit_token.address.clone(),
                        symbol_short!("transfer"),
                        (&user, &insurance.address.clone(), 10_000i128).into_val(&env),
                    )),
                    sub_invocations: std::vec![],
                }],
            },
        ),]
    );

    // TODO: this will fail
    let stake = insurance.query_if_stake(&user);
    assert_eq!(
        stake,
        Stake {
            if_shares: 0,
            last_withdraw_request_shares: 0,
            if_base: 0,
            last_valid_ts: 0,
            last_withdraw_request_value: 0,
            last_withdraw_request_ts: 0,
            cost_basis: 0,
        }
    );

    let insurance_fund = insurance.query_insurance_fund();

    let n_shares = vault_amount_to_if_shares(
        &env,
        10_000,
        insurance_fund.total_shares,
        deposit_token.balance(&insurance.address),
    );

    assert_eq!(
        insurance_fund.total_shares,
        insurance_fund_before.total_shares + n_shares
    );
    assert_eq!(
        insurance_fund.user_shares,
        insurance_fund_before.user_shares + n_shares
    );

    assert_eq!(deposit_token.balance(&user), 0);
    assert_eq!(deposit_token.balance(&insurance.address), 10_000);
}

#[test]
#[should_panic(expected = "")]
fn add_stake_over_max_insurance_should_fail() {
    let env = Env::default();
    env.mock_all_auths();
}

#[test]
fn request_remove_stake() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let governor_contract = Address::generate(&env);
    let user = Address::generate(&env);
    let user2 = Address::generate(&env);

    let deposit_token = deploy_token_contract(&env, &admin);

    let insurance =
        deploy_insurance_contract(&env, &admin, &governor_contract, &deposit_token.address);

    // env.ledger().with_mut(|li| {
    //     li.timestamp = ONE_WEEK;
    // });

    deposit_token.mint(&user, &10_000);

    insurance.add_if_stake(&user, &10_000);

    env.ledger().with_mut(|li| {
        li.timestamp += ONE_DAY;
    });

    assert_eq!(deposit_token.balance(&user), 0);
    assert_eq!(deposit_token.balance(&insurance.address), 10_000);

    insurance.request_remove_if_stake(&user, &10_000);

    assert_eq!(
        env.auths(),
        [(
            user.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    insurance.address.clone(),
                    Symbol::new(&env, "request_remove_if_stake"),
                    (&user.clone(), 10_000i128).into_val(&env),
                )),
                sub_invocations: std::vec![],
            },
        ),]
    );

    let stake = insurance.query_if_stake(&user);
    assert_eq!(
        stake,
        Stake {
            last_withdraw_request_shares: 0,
            last_withdraw_request_value: 0,
            last_withdraw_request_ts: 0,
        }
    );

    assert_eq!(deposit_token.balance(&user), 10_000);
    assert_eq!(deposit_token.balance(&user2), 0);
    assert_eq!(deposit_token.balance(&insurance.address), 35_000);
}

#[test]
fn cancel_request_remove_stake() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let governor = Address::generate(&env);
    let user = Address::generate(&env);
    let gov_token = deploy_token_contract(&env, &admin);

    let insurance =
        deploy_insurance_contract(&env, admin.clone(), governor.clone(), &gov_token.address);

    env.ledger().with_mut(|li| {
        li.timestamp = ONE_WEEK;
    });

    // lp_token.mint(&user, &10_000);

    insurance.cancel_request_remove_if_stake(&user);

    assert_eq!(
        env.auths(),
        [(
            user.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    insurance.address.clone(),
                    Symbol::new(&env, "cancel_request_remove_if_stake"),
                    (&user.clone()).into_val(&env),
                )),
                sub_invocations: std::vec![],
            },
        ),]
    );

    let stake = insurance.query_if_stake(&user);
    // assert_eq!(
    //     stake,
    //     vec![
    //         &env,
    //         Stake {
    //             last_withdraw_request_shares: 0,
    //             last_withdraw_request_value: 0,
    //             last_withdraw_request_ts: 0,
    //         }
    //     ]
    // );
}

#[test]
fn remove_stake() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let user2 = Address::generate(&env);
    let manager = Address::generate(&env);
    let owner = Address::generate(&env);
    let lp_token = deploy_token_contract(&env, &admin);

    let insurance =
        deploy_insurance_contract(&env, admin.clone(), governor.clone(), &gov_token.address);

    lp_token.mint(&user, &35_000);
    lp_token.mint(&user2, &10_000);

    env.ledger().with_mut(|li| {
        li.timestamp += ONE_DAY;
    });
    insurance.bond(&user, &10_000);
    env.ledger().with_mut(|li| {
        li.timestamp += ONE_DAY;
    });
    insurance.bond(&user, &10_000);
    insurance.bond(&user2, &10_000);
    env.ledger().with_mut(|li| {
        li.timestamp += ONE_DAY;
    });
    insurance.bond(&user, &15_000);

    assert_eq!(insurance.query_staked(&user).stakes.len(), 3);
    assert_eq!(lp_token.balance(&user), 0);
    assert_eq!(lp_token.balance(&insurance.address), 45_000);

    insurance.remove_if_stake(&user);

    assert_eq!(
        env.auths(),
        [(
            user.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    insurance.address.clone(),
                    Symbol::new(&env, "remove_if_stake"),
                    (&user.clone()).into_val(&env),
                )),
                sub_invocations: std::vec![],
            },
        ),]
    );

    let stake = insurance.query_if_stake(&user);
    assert_eq!(
        stake,
        vec![
            &env,
            Stake {
                stake: 10_000,
                stake_timestamp: ONE_DAY,
            }
        ]
    );
    // assert_eq!(staking.query_total_staked(), 35_000);

    assert_eq!(lp_token.balance(&user), 10_000);
    assert_eq!(lp_token.balance(&user2), 0);
    assert_eq!(lp_token.balance(&staking.address), 35_000);
}

// #[test]
// fn initializing_contract_sets_total_staked_var() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let admin = Address::generate(&env);
//     let manager = Address::generate(&env);
//     let owner = Address::generate(&env);
//     let lp_token = deploy_token_contract(&env, &admin);

//     let staking = deploy_staking_contract(
//         &env,
//         admin.clone(),
//         &lp_token.address,
//         &manager,
//         &owner,
//         &DEFAULT_COMPLEXITY
//     );

//     assert_eq!(staking.query_total_staked(), 0);
// }

#[test]
#[should_panic(expected = "Stake: Remove stake: Stake not found")]
fn remove_stake_wrong_user_stake_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let user2 = Address::generate(&env);
    let manager = Address::generate(&env);
    let owner = Address::generate(&env);
    let lp_token = deploy_token_contract(&env, &admin);

    let staking = deploy_staking_contract(
        &env,
        admin.clone(),
        &lp_token.address,
        &manager,
        &owner,
        &DEFAULT_COMPLEXITY,
    );

    lp_token.mint(&user, &35_000);
    lp_token.mint(&user2, &10_000);

    env.ledger().with_mut(|li| {
        li.timestamp = ONE_DAY;
    });
    staking.bond(&user, &10_000);
    env.ledger().with_mut(|li| {
        li.timestamp += ONE_DAY;
    });
    staking.bond(&user, &10_000);
    staking.bond(&user2, &10_000);

    assert_eq!(lp_token.balance(&user), 15_000);
    assert_eq!(lp_token.balance(&user2), 0);
    assert_eq!(lp_token.balance(&staking.address), 30_000);

    let non_existing_timestamp = ONE_DAY / 2;
    staking.unbond(&user2, &10_000, &non_existing_timestamp);
}

#[test]
#[should_panic(expected = "No withdraw request in progress")]
fn cancel_request_remove_stake_without_request_in_progress() {
    let env = Env::default();
    env.mock_all_auths();
}

#[test]
#[should_panic(expected = "Must submit withdraw request and wait the escrow period")]
fn remove_stake_before_unstaking_period() {
    let env = Env::default();
    env.mock_all_auths();
}

#[test]
fn pay_rewards_during_remove_stake() {
    let env = Env::default();
    env.mock_all_auths();
    env.cost_estimate().budget().reset_unlimited();

    let full_bonding_multiplier = ONE_DAY * 60;

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let manager = Address::generate(&env);

    let stake_asset = deploy_token_contract(&env, &admin);
    let reward_token = deploy_token_contract(&env, &admin);
    let insurance = deploy_insurance_contract(&env, admin.clone());

    stake_asset.mint(&user, &10_000);
    // reward_token.mint(&admin, &20_000);

    let staked = 1_000;
    insurance.add_if_stake(&user, &staked);

    insurance.request_remove_if_stake(&user, &staked);

    // Move so that user would have 100% APR from bonding after 60 days
    env.ledger().with_mut(|li| {
        li.timestamp = full_bonding_multiplier;
    });

    // user bonded at timestamp 0
    // insurance.request_remove_if_stake(&user, &staked, &0);
    assert_eq!(stake_asset.balance(&insurance.address), 0);
    assert_eq!(stake_asset.balance(&user), 9000 + staked);
    assert_eq!(
        insurance.query_if_stake(&user),
        Stake {
            stakes: Vec::new(&env),
            total_stake: 0i128,
            last_reward_time: 6_912_000,
        }
    );
}
