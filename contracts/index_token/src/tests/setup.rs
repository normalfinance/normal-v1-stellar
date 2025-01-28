use crate::contract::{IndexFactory, IndexFactoryClient};
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env, String};
pub const ONE_DAY: u64 = 86400;
const TOKEN_WASM: &[u8] =
    include_bytes!("../../../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm");

#[allow(clippy::too_many_arguments)]
pub fn install_index_wasm(env: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_index.wasm"
    );
    env.deployer().upload_contract_wasm(WASM)
}

#[allow(clippy::too_many_arguments)]
pub fn install_index_token_wasm(env: &Env) -> BytesN<32> {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/normal_index_token.wasm"
    );
    env.deployer().upload_contract_wasm(WASM)
}

pub fn deploy_index_token_contract<'a>(
    env: &Env,
    admin: impl Into<Option<Address>>,
    oracle: impl Into<Option<Address>>,
) -> IndexFactoryClient<'a> {
    let admin = admin.into().unwrap_or(Address::generate(env));
    let factory = IndexFactoryClient::new(env, &env.register(IndexFactory, ()));

    let paused_operations = vec![];

    let index_wasm_hash = install_index_wasm(env);
    let index_token_wasm_hash = install_index_token_wasm(env);

    factory.initialize(
        &admin,
        &index_wasm_hash,
        &index_token_wasm_hash,
        &paused_operations,
        500,
        300,
        &oracle,
    );

    factory
}
