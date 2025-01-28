use crate::contract::{IndexTokenFactory, IndexTokenFactoryClient};
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env, String};
pub const ONE_DAY: u64 = 86400;
const TOKEN_WASM: &[u8] =
    include_bytes!("../../../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm");

#[allow(clippy::too_many_arguments)]
pub mod index_token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_index_token.wasm"
    );
}

pub fn install_index_token_contract(env: &Env) -> BytesN<32> {
    env.deployer()
        .upload_contract_wasm(index_token_contract::WASM)
}

pub fn deploy_index_factory_contract<'a>(
    env: &Env,
    admin: impl Into<Option<Address>>,
    oracle: impl Into<Option<Address>>,
) -> IndexTokenFactoryClient<'a> {
    let admin = admin.into().unwrap_or(Address::generate(env));
    let factory = IndexTokenFactoryClient::new(env, &env.register(IndexTokenFactory, ()));

    let paused_operations = vec![];

    let index_token_wasm_hash = install_index_token_contract(env);

    factory.initialize(
        &admin,
        &index_token_wasm_hash,
        &paused_operations,
        500,
        300,
        &oracle,
    );

    factory
}

pub fn generate_index_token_init_info(
    token_a: Address,
    token_b: Address,
    manager: Address,
    admin: Address,
    fee_recipient: Address,
) -> LiquidityPoolInitInfo {
    let token_init_info = TokenInitInfo { token_a, token_b };

    let stake_init_info = StakeInitInfo {
        min_bond: 10,
        min_reward: 10,
        manager,
        max_complexity: 10u32,
    };

    LiquidityPoolInitInfo {
        admin: admin.clone(),
        fee_recipient: fee_recipient.clone(),
        max_allowed_slippage_bps: 5000,
        max_allowed_spread_bps: 500,
        default_slippage_bps: 2_500,
        swap_fee_bps: 0,
        max_referral_bps: 5000,
        token_init_info,
        stake_init_info,
    }
}
