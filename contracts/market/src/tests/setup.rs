use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String};

use crate::{
    contract::{SynthPool, SynthPoolClient},
    token_contract,
};

use normal::utils::{SynthPoolInitInfo, TokenInitInfo};

pub fn deploy_token_contract<'a>(env: &Env, admin: &Address) -> token_contract::Client<'a> {
    token_contract::Client::new(
        env,
        &env.register_stellar_asset_contract_v2(admin.clone())
            .address(),
    )
}

pub fn install_token_wasm(env: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
    );
    env.deployer().upload_contract_wasm(WASM)
}

// #[allow(clippy::too_many_arguments)]
// pub fn install_stake_wasm(env: &Env) -> BytesN<32> {
//     soroban_sdk::contractimport!(
//         file = "../../target/wasm32-unknown-unknown/release/phoenix_stake.wasm"
//     );
//     env.deployer().upload_contract_wasm(WASM)
// }

#[allow(clippy::too_many_arguments)]
pub fn install_new_lp_wasm(env: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/phoenix_pool.wasm"
    );
    env.deployer().upload_contract_wasm(WASM)
}

#[allow(clippy::too_many_arguments)]
pub fn deploy_synth_pool_contract<'a>(
    env: &Env,
    admin: impl Into<Option<Address>>,
    token_a_b: (&Address, &Address),
    swap_fees: i64,
    fee_recipient: impl Into<Option<Address>>,
    max_allowed_slippage_bps: impl Into<Option<i64>>,
    max_allowed_spread_bps: impl Into<Option<i64>>,
    stake_manager: Address,
    stake_owner: Address,
) -> SynthPoolClient<'a> {
    let admin = admin.into().unwrap_or(Address::generate(env));
    let pool = SynthPoolClient::new(env, &env.register(SynthPool, ()));
    // let fee_recipient = fee_recipient.into().unwrap_or_else(|| Address::generate(env));

    let token_init_info = TokenInitInfo {
        token_a: token_a_b.0.clone(),
        token_b: token_a_b.1.clone(),
    };

    // let stake_wasm_hash = install_stake_wasm(env);
    let token_wasm_hash = install_token_wasm(env);

    let pool_params = SynthPoolInitInfo {
        admin,
        swap_fee_bps: swap_fees,
        fee_recipient,
        max_allowed_slippage_bps: max_allowed_slippage_bps.into().unwrap_or(5_000),
        default_slippage_bps: 2_500,
        max_allowed_spread_bps: max_allowed_spread_bps.into().unwrap_or(1_000),
        max_referral_bps: 5_000,
        token_init_info,
    };

    pool.initialize(
        &stake_wasm_hash,
        &token_wasm_hash,
        &pool_params,
        &stake_owner,
        &10u32,
        &String::from_str(env, "Pool"),
        &String::from_str(env, "PHOBTC"),
        &100i64,
        &1_000,
    );
    pool
}
