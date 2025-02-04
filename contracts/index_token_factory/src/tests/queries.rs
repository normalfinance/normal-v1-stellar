use super::setup::{
    deploy_index_token_factory_contract,
    deploy_index_token_factory_contract,
    generate_index_token_init_info,
};
use crate::token_contract;
use normal::types::IndexParams;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::vec;
use soroban_sdk::{
    contracttype,
    testutils::{ arbitrary::std, Address as _ },
    Address,
    Env,
    String,
    Symbol,
    Vec,
};

#[test]
fn test_deploy_multiple_index_tokens() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let mut token1 = Address::generate(&env);
    let mut token2 = Address::generate(&env);
    let mut token3 = Address::generate(&env);
    let mut token4 = Address::generate(&env);
    let mut token5 = Address::generate(&env);
    let mut token6 = Address::generate(&env);

    env.mock_all_auths();
    env.cost_estimate().budget().reset_unlimited();

    if token2 < token1 {
        std::mem::swap(&mut token1, &mut token2);
    }

    if token4 < token3 {
        std::mem::swap(&mut token3, &mut token4);
    }

    if token6 < token5 {
        std::mem::swap(&mut token5, &mut token6);
    }

    let factory = deploy_factory_contract(&env, Some(admin.clone()));

    let first_token_init_info = TokenInitInfo {
        token_a: token1.clone(),
        token_b: token2.clone(),
    };
    let first_stake_init_info = StakeInitInfo {
        min_bond: 10i128,
        min_reward: 5i128,
        manager: Address::generate(&env),
        max_complexity: 10u32,
    };

    let second_token_init_info = TokenInitInfo {
        token_a: token3.clone(),
        token_b: token4.clone(),
    };
    let second_stake_init_info = StakeInitInfo {
        min_bond: 5i128,
        min_reward: 2i128,
        manager: Address::generate(&env),
        max_complexity: 10u32,
    };

    let third_token_init_info = TokenInitInfo {
        token_a: token5.clone(),
        token_b: token6.clone(),
    };
    let third_stake_init_info = StakeInitInfo {
        min_bond: 6i128,
        min_reward: 3i128,
        manager: Address::generate(&env),
        max_complexity: 10u32,
    };

    let first_lp_init_info = LiquidityPoolInitInfo {
        admin: admin.clone(),
        fee_recipient: user.clone(),
        max_allowed_slippage_bps: 5_000,
        default_slippage_bps: 2_500,
        max_allowed_spread_bps: 500,
        swap_fee_bps: 0,
        max_referral_bps: 5_000,
        token_init_info: first_token_init_info.clone(),
        stake_init_info: first_stake_init_info,
    };

    let second_lp_init_info = LiquidityPoolInitInfo {
        admin: admin.clone(),
        fee_recipient: user.clone(),
        max_allowed_slippage_bps: 4_000,
        default_slippage_bps: 2_500,
        max_allowed_spread_bps: 400,
        swap_fee_bps: 0,
        max_referral_bps: 5_000,
        token_init_info: second_token_init_info,
        stake_init_info: second_stake_init_info,
    };

    let third_lp_init_info = LiquidityPoolInitInfo {
        admin: admin.clone(),
        fee_recipient: user.clone(),
        max_allowed_slippage_bps: 4_000,
        default_slippage_bps: 2_500,
        max_allowed_spread_bps: 400,
        swap_fee_bps: 0,
        max_referral_bps: 5_000,
        token_init_info: third_token_init_info,
        stake_init_info: third_stake_init_info,
    };

    let lp_contract_addr = factory.create_liquidity_pool(
        &admin.clone(),
        &first_lp_init_info,
        &String::from_str(&env, "Pool"),
        &String::from_str(&env, "PHO/BTC"),
        &PoolType::Xyk,
        &None::<u64>,
        &100i64,
        &1_000
    );
    let second_lp_contract_addr = factory.create_liquidity_pool(
        &admin.clone(),
        &second_lp_init_info,
        &String::from_str(&env, "Pool #2"),
        &String::from_str(&env, "PHO/ETH"),
        &PoolType::Xyk,
        &None::<u64>,
        &100i64,
        &1_000
    );
    let third_lp_contract_addr = factory.create_liquidity_pool(
        &admin.clone(),
        &third_lp_init_info,
        &String::from_str(&env, "Pool #3"),
        &String::from_str(&env, "PHO/XLM"),
        &PoolType::Xyk,
        &None::<u64>,
        &100i64,
        &1_000
    );

    let first_result = factory.query_pool_details(&lp_contract_addr);
    let share_token_addr: Address = env.invoke_contract(
        &lp_contract_addr,
        &Symbol::new(&env, "query_share_token_address"),
        Vec::new(&env)
    );
    let first_lp_config: LiquidityPoolConfig = env.invoke_contract(
        &lp_contract_addr,
        &Symbol::new(&env, "query_config"),
        Vec::new(&env)
    );

    assert_eq!(first_lp_init_info.max_allowed_spread_bps, first_lp_config.max_allowed_spread_bps);

    assert_eq!(token1, first_result.pool_response.asset_a.address);
    assert_eq!(token2, first_result.pool_response.asset_b.address);
    assert_eq!(share_token_addr, first_result.pool_response.asset_lp_share.address);
    assert_eq!(lp_contract_addr, first_result.pool_address);

    let second_result = factory.query_pool_details(&second_lp_contract_addr);
    let second_share_token_addr: Address = env.invoke_contract(
        &second_lp_contract_addr,
        &Symbol::new(&env, "query_share_token_address"),
        Vec::new(&env)
    );
    let second_lp_config: LiquidityPoolConfig = env.invoke_contract(
        &second_lp_contract_addr,
        &Symbol::new(&env, "query_config"),
        Vec::new(&env)
    );

    assert_eq!(second_lp_init_info.max_allowed_spread_bps, second_lp_config.max_allowed_spread_bps);

    assert_eq!(token3, second_result.pool_response.asset_a.address);
    assert_eq!(token4, second_result.pool_response.asset_b.address);
    assert_eq!(second_share_token_addr, second_result.pool_response.asset_lp_share.address);
    assert_eq!(second_lp_contract_addr, second_result.pool_address);

    let third_result = factory.query_pool_details(&third_lp_contract_addr);
    let third_share_token_addr: Address = env.invoke_contract(
        &third_lp_contract_addr,
        &Symbol::new(&env, "query_share_token_address"),
        Vec::new(&env)
    );
    let third_lp_config: LiquidityPoolConfig = env.invoke_contract(
        &third_lp_contract_addr,
        &Symbol::new(&env, "query_config"),
        Vec::new(&env)
    );

    assert_eq!(third_lp_init_info.max_allowed_spread_bps, third_lp_config.max_allowed_spread_bps);

    assert_eq!(token5, third_result.pool_response.asset_a.address);
    assert_eq!(token6, third_result.pool_response.asset_b.address);
    assert_eq!(third_share_token_addr, third_result.pool_response.asset_lp_share.address);
    assert_eq!(third_lp_contract_addr, third_result.pool_address);

    let all_pools = factory.query_all_pools_details();
    assert_eq!(all_pools.len(), 3);
    all_pools.iter().for_each(|pool| {
        assert!(all_pools.contains(pool));
    });

    let first_lp_address_by_tuple = factory.query_for_pool_by_token_pair(&token1, &token2);
    assert_eq!(first_lp_address_by_tuple, lp_contract_addr);
}

#[test]
fn test_queries_by_tuple() {
    let env = Env::default();
    env.cost_estimate().budget().reset_unlimited();
    let admin = Address::generate(&env);
    let governor = Address::generate(&env);
    let user = Address::generate(&env);

    let mut quoteTokenAddress = Address::generate(&env);
    let mut quoteToken = Symbol::new(&env, "XLM");
    let mut token1 = Symbol::new(&env, "BTC");
    let mut token2 = Symbol::new(&env, "ETH");
    let mut token3 = Symbol::new(&env, "SOL");

    env.mock_all_auths();
    env.cost_estimate().budget().reset_unlimited();

    // if token2 < token1 {
    //     std::mem::swap(&mut token1, &mut token2);
    // }

    // if token4 < token3 {
    //     std::mem::swap(&mut token3, &mut token4);
    // }

    // if token6 < token5 {
    //     std::mem::swap(&mut token5, &mut token6);
    // }

    let factory = deploy_index_token_factory_contract(&env, Some(admin.clone()), Some(governor.clone()));

    let first_index_token_params = IndexParams {
        admin: admin.clone(),
        // ...
    };

    let second_index_token_params = IndexParams {
        admin: admin.clone(),
        // ...
    };

    let third_index_token_params = IndexParams {
        admin: admin.clone(),
        // ...
    };

    let index_token_contract_addr = factory.create_index_token(
        &admin.clone(),
        &quoteTokenAddress,
        &first_index_token_params,
        "",
        &String::from_str(&env, "Normal Bitcoin"),
        &String::from_str(&env, "nBTC")
    );
    let second_index_token_contract_addr = factory.create_index_token(
        &admin.clone(),
        &quoteTokenAddress,
        &second_index_token_params,
        "",
        &String::from_str(&env, "Normal Ethereum"),
        &String::from_str(&env, "nETH")
    );
    let third_index_token_contract_addr = factory.create_index_token(
        &admin.clone(),
        &quoteTokenAddress,
        &third_index_token_params,
        "",
        &String::from_str(&env, "Normal Solana"),
        &String::from_str(&env, "nSOL")
    );

    let first_result = factory.query_index_details(&index_token_contract_addr);

    assert_eq!(token1, first_result.market_response.asset_a.address);
    assert_eq!(token2, first_result.market_response.asset_b.address);
    assert_eq!(market_contract_addr, first_result.market_address);

    let second_result = factory.query_market_details(&second_market_contract_addr);
    let second_share_token_addr: Address = env.invoke_contract(
        &second_market_contract_addr,
        &Symbol::new(&env, "query_share_token_address"),
        Vec::new(&env)
    );

    let second_lp_config: LiquidityPoolConfig = env.invoke_contract(
        &second_market_contract_addr,
        &Symbol::new(&env, "query_config"),
        Vec::new(&env)
    );

    // assert_eq!(second_market_params.max_allowed_spread_bps, second_lp_config.max_allowed_spread_bps);

    assert_eq!(token3, second_result.market_response.asset_a.address);
    assert_eq!(token4, second_result.market_response.asset_b.address);
    assert_eq!(second_share_token_addr, second_result.market_response.asset_lp_share.address);
    assert_eq!(second_market_contract_addr, second_result.market_address);

    let first_market_address_by_tuple = factory.query_for_market_by_token_pair(&token1, &quoteToken);
    let second_market_address_by_tuple = factory.query_for_market_by_token_pair(&token2, &quoteToken);
    let third_market_address_by_tuple = factory.query_for_market_by_token_pair(&token3, &quoteToken);

    assert_eq!(first_market_address_by_tuple, market_contract_addr);
    assert_eq!(second_market_address_by_tuple, second_market_contract_addr);
    assert_eq!(third_market_address_by_tuple, third_market_contract_addr);
}

#[test]
#[should_panic(expected = "Factory: query_for_market_by_token_pair failed: No market found")]
fn test_queries_by_tuple_errors() {
    let env = Env::default();
    env.cost_estimate().budget().reset_unlimited();
    let admin = Address::generate(&env);
    let governor = Address::generate(&env);
    let factory: crate::contract::MarketFactoryClient<'_> = deploy_index_token_factory_contract(
        &env,
        Some(admin.clone()),
        Some(governor.clone())
    );

    factory.query_for_market_by_token_pair(&Symbol::new(&env, ""), &Symbol::new(&env, ""));
}
