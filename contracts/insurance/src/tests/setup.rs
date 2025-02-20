use soroban_sdk::{Address, BytesN, Env, String};

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
mod insurnace_latest {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_insurance.wasm"
    );
}

pub fn install_token_wasm(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(token_contract::WASM)
}

#[allow(dead_code)]
fn install_insurnace_latest_wasm(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(insurnace_latest::WASM)
}

pub const ONE_WEEK: u64 = 604800;
pub const ONE_DAY: u64 = 86400;
pub const SIXTY_DAYS: u64 = 60 * ONE_DAY;

pub fn deploy_insurance_contract<'a>(
    env: &Env,
    admin: &Address,
    deposit_token: &Address,
) -> InsuranceClient<'a> {
    let insurance = InsuranceClient::new(env, &env.register(Insurance, ()));

    let token_wasm_hash = install_token_wasm(env);

    insurance.initialize(
        &admin,
        deposit_token,
        &token_wasm_hash,
        &10u32,
        &String::from_str(&env, "Normal Insurance Fund Stake"),
        &String::from_str(&env, "NIFS"),
        &1_000_000i128,
    );
    insurance
}

#[cfg(feature = "upgrade")]
use soroban_sdk::{testutils::Ledger, vec};

#[test]
#[cfg(feature = "upgrade")]
fn upgrade_insurance_contract() {
    let env = Env::default();
    env.mock_all_auths();
    env.budget().reset_unlimited();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let token_client = deploy_token_contract(&env, &admin);
    token_client.mint(&user, &1_000);

    let insurance_addr = env.register_contract_wasm(None, insurance_v_1_0_0::WASM);

    let insurance_v_1_0_0_client = insurance_v_1_0_0::Client::new(&env, &insurance_addr);

    let manager = Address::generate(&env);
    let owner = Address::generate(&env);

    insurance_v_1_0_0_client.initialize(
        &admin,
        &token_client.address,
        &10,
        &10,
        &manager,
        &owner,
        &10,
    );

    assert_eq!(insurance_v_1_0_0_client.query_admin(), admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 100;
    });
    insurance_v_1_0_0_client.bond(&user, &1_000);
    assert_eq!(
        insurance_v_1_0_0_client.query_if_stake(&user),
        insurance_v_1_0_0::Stake {
            stakes: vec![
                &env,
                insurance_v_1_0_0::Stake {
                    stake: 1_000i128,
                    stake_timestamp: 100,
                }
            ],
        }
    );

    env.ledger().with_mut(|li| {
        li.timestamp = 10_000;
    });

    let new_insurance_wasm = install_insurnace_latest_wasm(&env);
    insurance_v_1_0_0_client.update(&new_insurance_wasm);
    insurance_v_1_0_0_client.update(&new_insurance_wasm);

    let upgraded_insurance_client = insurance_latest::Client::new(&env, &insurance_addr);

    assert_eq!(upgraded_insurance_client.query_admin(), admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 20_000;
    });

    upgraded_insurance_client.unbond(&user, &1_000, &100);
    assert_eq!(
        upgraded_insurance_client.query_if_stake(&user),
        insurance_latest::Stake {
            stakes: vec![&env],
            total_stake: 0i128,
        }
    );

    // upgraded_insurance_client.create_distribution_flow(&owner, &token_client.address);
    // token_client.mint(&owner, &1_000);
    // upgraded_insurance_client.distribute_rewards(&owner, &1_000, &token_client.address);
}
