use crate::{
    contract::{MarketFactory, MarketFactoryClient},
    token_contract,
};
// use phoenix::utils::{LiquidityPoolInitInfo, StakeInitInfo, TokenInitInfo};
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env, String};
pub const ONE_DAY: u64 = 86400;
const TOKEN_WASM: &[u8] =
    include_bytes!("../../../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm");

#[allow(clippy::too_many_arguments)]
pub mod market_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_market.wasm"
    );
}

pub fn install_market_contract(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(market_contract::WASM)
}

pub fn install_token_wasm(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(token_contract::WASM)
}

pub fn deploy_market_factory_contract<'a>(
    env: &Env,
    admin: impl Into<Option<Address>>,
    governor: impl Into<Option<Address>>,
) -> MarketFactoryClient<'a> {
    let admin = admin.into().unwrap_or(Address::generate(env));
    let governor: Address = governor.into().unwrap_or(Address::generate(env));
    let factory = MarketFactoryClient::new(env, &env.register(MarketFactory, ()));

    let market_wasm_hash = install_market_contract(env);
    let token_wasm_hash = install_token_wasm(env);

    factory.initialize(&admin, &governor, &market_wasm_hash, &token_wasm_hash);

    factory
}

pub fn generate_market_init_info(
    token_a: Address,
    token_b: Address,
    manager: Address,
    admin: Address,
    fee_recipient: Address,
) -> MarketParams {
    let pool_params = PoolParams {
        min_bond: 10,
        min_reward: 10,
        manager,
        max_complexity: 10u32,
    };

    MarketParams {
        admin: admin.clone(),
        fee_recipient: fee_recipient.clone(),
        max_allowed_slippage_bps: 5000,
        max_allowed_spread_bps: 500,
        default_slippage_bps: 2_500,
        swap_fee_bps: 0,
        max_referral_bps: 5000,
        pool_params,
    }
}

pub fn install_and_deploy_token_contract<'a>(
    env: &Env,
    admin: Address,
    decimal: u32,
    name: String,
    symbol: String,
) -> token_contract::Client<'a> {
    let token_addr = env.register(TOKEN_WASM, (admin, decimal, name, symbol));
    let token_client = token_contract::Client::new(env, &token_addr);

    token_client
}
