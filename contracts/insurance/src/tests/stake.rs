extern crate std;

use pretty_assertions::assert_eq;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger},
    vec, Address, Env, IntoVal, Symbol, Vec,
};

use super::setup::{deploy_insurance_contract, deploy_token_contract};

use crate::{
    contract::{Insurance, InsuranceClient},
    msg::{ConfigResponse, StakedResponse},
    storage::{Config, Stake},
    tests::setup::{ONE_DAY, ONE_WEEK},
};

const DEFAULT_COMPLEXITY: u32 = 7;

#[test]
fn initialize_insurance_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let lp_token = deploy_token_contract(&env, &admin);
    let manager = Address::generate(&env);
    let owner = Address::generate(&env);

    let insurance = deploy_insurance_contract(
        &env,
        admin.clone(),
        &lp_token.address,
        &manager,
        &owner,
        &DEFAULT_COMPLEXITY,
    );

    let response = insurance.query_config();
    assert_eq!(
        response,
        ConfigResponse {
            config: Config {
                lp_token: lp_token.address,
            },
        }
    );

    let response = staking.query_admin();
    assert_eq!(response, admin);
}

#[test]
#[should_panic(expected = "Insurance: Initialize: initializing contract twice is not allowed")]
fn test_deploying_insurance_twice_should_fail() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let governor = Address::generate(&env);
    let lp_token = deploy_token_contract(&env, &admin);

    let first = deploy_insurance_contract(&env, admin.clone(), governor.clone(), &lp_token.address);

    first.initialize(
        &admin,
        &governor,
        &lp_token.address,
        &100i128,
        18,
        "TEST",
        "TEEST",
    );
}

#[test]
fn add_stake_simple() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let governor = Address::generate(&env);
    let user = Address::generate(&env);
    let lp_token = deploy_token_contract(&env, &admin);

    let insurance =
        deploy_insurance_contract(&env, admin.clone(), governor.clone(), &lp_token.address);

    env.ledger().with_mut(|li| {
        li.timestamp = ONE_WEEK;
    });

    lp_token.mint(&user, &10_000);

    insurance.add_stake(&user, &10_000);

    assert_eq!(
        env.auths(),
        [(
            user.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    insurance.address.clone(),
                    Symbol::new(&env, "bond"),
                    (&user.clone(), 10_000i128).into_val(&env),
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        lp_token.address.clone(),
                        symbol_short!("transfer"),
                        (&user, &insurance.address.clone(), 10_000i128).into_val(&env),
                    )),
                    sub_invocations: std::vec![],
                }],
            },
        ),]
    );

    let stake = insurance.query_stake(&user);
    assert_eq!(
        stake,
        vec![
            &env,
            Stake {
                stake: 10_000,
                stake_timestamp: ONE_WEEK,
            }
        ]
    );
    // assert_eq!(insurance.query_total_staked(), 10_000);

    assert_eq!(lp_token.balance(&user), 0);
    assert_eq!(lp_token.balance(&insurance.address), 10_000);
}

#[test]
fn request_remove_simple() {}

#[test]
fn cancel_request_remove_simple() {}

#[test]
fn remove_stake_simple() {
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
        li.timestamp += ONE_DAY;
    });
    staking.bond(&user, &10_000);
    env.ledger().with_mut(|li| {
        li.timestamp += ONE_DAY;
    });
    staking.bond(&user, &10_000);
    staking.bond(&user2, &10_000);
    env.ledger().with_mut(|li| {
        li.timestamp += ONE_DAY;
    });
    staking.bond(&user, &15_000);

    assert_eq!(staking.query_staked(&user).stakes.len(), 3);
    assert_eq!(lp_token.balance(&user), 0);
    assert_eq!(lp_token.balance(&staking.address), 45_000);

    staking.unbond(&user, &10_000, &(ONE_DAY + ONE_DAY));

    assert_eq!(
        env.auths(),
        [(
            user.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    staking.address.clone(),
                    Symbol::new(&env, "unbond"),
                    (&user.clone(), 10_000i128, ONE_DAY + ONE_DAY).into_val(&env),
                )),
                sub_invocations: std::vec![],
            },
        ),]
    );

    let bonds = staking.query_staked(&user).stakes;
    assert_eq!(
        bonds,
        vec![
            &env,
            Stake {
                stake: 10_000,
                stake_timestamp: ONE_DAY,
            },
            Stake {
                stake: 15_000,
                stake_timestamp: 3 * ONE_DAY,
            }
        ]
    );
    assert_eq!(staking.query_total_staked(), 35_000);

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

// #[test]
// #[should_panic(expected = "Stake: Remove stake: Stake not found")]
// fn unbond_wrong_user_stake_not_found() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let admin = Address::generate(&env);
//     let user = Address::generate(&env);
//     let user2 = Address::generate(&env);
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

//     lp_token.mint(&user, &35_000);
//     lp_token.mint(&user2, &10_000);

//     env.ledger().with_mut(|li| {
//         li.timestamp = ONE_DAY;
//     });
//     staking.bond(&user, &10_000);
//     env.ledger().with_mut(|li| {
//         li.timestamp += ONE_DAY;
//     });
//     staking.bond(&user, &10_000);
//     staking.bond(&user2, &10_000);

//     assert_eq!(lp_token.balance(&user), 15_000);
//     assert_eq!(lp_token.balance(&user2), 0);
//     assert_eq!(lp_token.balance(&staking.address), 30_000);

//     let non_existing_timestamp = ONE_DAY / 2;
//     staking.unbond(&user2, &10_000, &non_existing_timestamp);
// }

// #[test]
// fn pay_rewards_during_unbond() {
//     let env = Env::default();
//     env.mock_all_auths();
//     env.cost_estimate().budget().reset_unlimited();

//     let full_bonding_multiplier = ONE_DAY * 60;

//     let admin = Address::generate(&env);
//     let user = Address::generate(&env);
//     let manager = Address::generate(&env);

//     let lp_token = deploy_token_contract(&env, &admin);
//     let reward_token = deploy_token_contract(&env, &admin);
//     let staking = deploy_staking_contract(
//         &env,
//         admin.clone(),
//         &lp_token.address,
//         &manager,
//         &admin,
//         &DEFAULT_COMPLEXITY
//     );

//     lp_token.mint(&user, &10_000);
//     reward_token.mint(&admin, &20_000);

//     let staked = 1_000;
//     staking.bond(&user, &staked);

//     // Move so that user would have 100% APR from bonding after 60 days
//     env.ledger().with_mut(|li| {
//         li.timestamp = full_bonding_multiplier;
//     });

//     staking.create_distribution_flow(&admin, &reward_token.address);

//     // simulate passing 20 days and distributing 1000 tokens each day
//     for _ in 0..20 {
//         staking.distribute_rewards(&admin, &1_000, &reward_token.address);
//         env.ledger().with_mut(|li| {
//             li.timestamp += 3600 * 24;
//         });
//     }

//     assert_eq!(
//         staking
//             .query_withdrawable_rewards(&user)
//             .rewards.iter()
//             .map(|reward| reward.reward_amount)
//             .sum::<u128>(),
//         20_000
//     );
//     assert_eq!(reward_token.balance(&user), 0);

//     // we first have to withdraw_rewards _before_ unbonding
//     // as this messes up with the reward calculation
//     // if we unbond first then we get no rewards
//     staking.withdraw_rewards(&user);
//     assert_eq!(reward_token.balance(&user), 20_000);

//     // user bonded at timestamp 0
//     staking.unbond(&user, &staked, &0);
//     assert_eq!(lp_token.balance(&staking.address), 0);
//     assert_eq!(lp_token.balance(&user), 9000 + staked);
//     assert_eq!(staking.query_staked(&user), StakedResponse {
//         stakes: Vec::new(&env),
//         total_stake: 0i128,
//         last_reward_time: 6_912_000,
//     });
// }
