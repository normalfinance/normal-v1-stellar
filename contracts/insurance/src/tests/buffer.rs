extern crate std;

use normal::constants::{ONE_MILLION_QUOTE, THIRTEEN_DAY};
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
    storage::{InsuranceFund, Stake},
    tests::setup::{ONE_DAY, ONE_WEEK},
};

#[test]
fn buyback() {
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

    insurance.execute_buffer_buyback(&user, &10_000);

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

    // ...
}

#[test]
fn auction() {
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

    lp_token.mint(&user, &10_000);

    insurance.add_if_stake(&user, &10_000);

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

    let stake = insurance.query_if_stake(&user);
    assert_eq!(
        stake,
        vec![
            &env,
            Stake {
                if_shares: 0,
                if_base: 0,
            }
        ]
    );
    // assert_eq!(insurance.query_total_staked(), 10_000);

    assert_eq!(lp_token.balance(&user), 0);
    assert_eq!(lp_token.balance(&insurance.address), 10_000);
}
