use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

use crate::{
    contract::{Insurance, InsuranceClient},
    token_contract,
};

pub fn deploy_token_contract<'a>(env: &Env, admin: &Address) -> token_contract::Client<'a> {
    token_contract::Client::new(
        env,
        &env.register_stellar_asset_contract_v2(admin.clone())
            .address(),
    )
}

#[allow(clippy::too_many_arguments)]
mod insurance_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_insurance.wasm"
    );
}

#[allow(dead_code)]
fn install_insurance_contract_wasm(env: &Env) -> BytesN<32> {
    env.deployer()
        .upload_contract_wasm(insurance_contract::WASM)
}

pub const ONE_WEEK: u64 = 604800;
pub const ONE_DAY: u64 = 86400;
pub const SIXTY_DAYS: u64 = 60 * ONE_DAY;

pub fn deploy_insurance_contract<'a>(
    env: &Env,
    admin: impl Into<Option<Address>>,
    lp_token: &Address,
    manager: &Address,
    owner: &Address,
    max_complexity: &u32,
) -> InsuranceClient<'a> {
    let admin = admin.into().unwrap_or(Address::generate(env));
    let insurance = InsuranceClient::new(env, &env.register(Insurance, ()));

    insurance.initialize(
        &admin,
        lp_token,
        &MIN_BOND,
        &MIN_REWARD,
        manager,
        owner,
        max_complexity,
    );
    insurance
}
