use super::setup::{ install_market_contract, install_token_wasm };
use crate::{
    tests::setup::{ deploy_market_factory_contract, generate_market_init_info, market_contract },
};

use soroban_sdk::{ testutils::{ arbitrary::std, Address as _ }, vec, Address, Env, String };

#[test]
fn factory_successfully_inits_itself() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let governor = Address::generate(&env);

    let factory = deploy_market_factory_contract(&env, Some(admin.clone()), Some(governor.clone()));

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
    let market_wasm_hash = install_market_contract(&env);
    let token_wasm_hash = install_token_wasm(&env);

    let factory = deploy_market_factory_contract(&env, admin.clone(), governor.clone());

    factory.initialize(&admin, &governor, &market_wasm_hash, &token_wasm_hash);
    factory.initialize(&admin, &governor, &market_wasm_hash, &token_wasm_hash);
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

    let factory = deploy_market_factory_contract(&env, Some(admin.clone()), Some(governor.clone()));
    assert_eq!(factory.get_admin(), admin);

    let market_params = generate_market_init_info(
        token1.clone(),
        token2.clone(),
        Address::generate(&env),
        admin.clone(),
        user.clone()
    );

    factory.create_market(
        &admin,
        &token1,
        &market_params,
        "",
        &String::from_str(&env, "Normal Bitcoin"),
        &String::from_str(&env, "nBTC")
    );
    let market_contract_addr = factory.query_markets().get(0).unwrap();

    let first_market_contract = market_contract::Client::new(&env, &market_contract_addr);
    let share_token_address = first_market_contract.query_share_token_address();
    let stake_token_address = first_market_contract.query_stake_contract_address();

    assert_eq!(first_market_contract.query_config(), market_contract::Config {
        fee_recipient: user,
        max_allowed_slippage_bps: 5_000,
        max_allowed_spread_bps: 500,
        max_referral_bps: 5_000,
        pool_type: lp_contract::PairType::Xyk,
        share_token: share_token_address,
        stake_contract: stake_token_address,
        token_a: token1,
        token_b: token2,
        total_fee_bps: 0,
    });
}


#[test]
fn factory_successfully_updates_emergency_oracles() {

}


#[test]
fn factory_successfully_updates_wasm_hashes() {

}


#[test]
fn factory_successfully_updates_oracle_guard_rails() {

}