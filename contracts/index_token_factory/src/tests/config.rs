use super::setup::deploy_index_factory_contract;
use crate::contract::{IndexFactory, IndexFactoryClient};

use soroban_sdk::{
    testutils::{arbitrary::std, Address as _},
    vec, Address, Env, String,
};

#[test]
fn factory_successfully_inits_itself() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    let factory = deploy_index_factory_contract(&env, Some(admin.clone()), oracle);

    assert_eq!(factory.get_admin(), admin);
}

#[test]
fn factory_successfully_inits_index_token() {
    let env = Env::default();
    let admin = Address::generate(&env);
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

    let factory = deploy_factory_contract(&env, Some(admin.clone()));
    assert_eq!(factory.get_admin(), admin);

    let lp_init_info = generate_lp_init_info(
        token1.clone(),
        token2.clone(),
        Address::generate(&env),
        admin.clone(),
        user.clone(),
    );

    factory.create_liquidity_pool(
        &admin,
        &lp_init_info,
        &String::from_str(&env, "Pool"),
        &String::from_str(&env, "PHO/BTC"),
        &PoolType::Xyk,
        &None::<u64>,
        &100i64,
        &1_000,
    );
    let lp_contract_addr = factory.query_pools().get(0).unwrap();

    let first_lp_contract = lp_contract::Client::new(&env, &lp_contract_addr);
    let share_token_address = first_lp_contract.query_share_token_address();
    let stake_token_address = first_lp_contract.query_stake_contract_address();

    assert_eq!(
        first_lp_contract.query_config(),
        lp_contract::Config {
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
        }
    );
}

#[test]
#[should_panic(
    expected = "Factory: Create Liquidity Pool: You are not authorized to create liquidity pool!"
)]
fn factory_fails_to_init_index_token_when_operation_paused() {}

#[test]
#[should_panic(
    expected = "Factory: Create Liquidity Pool: You are not authorized to create liquidity pool!"
)]
fn factory_fails_to_init_index_token_when_invalid_fees() {}

#[test]
fn factory_index_token_creation_should_fail_without_valid_oracle() {}
