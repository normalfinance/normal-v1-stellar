use crate::{ contract::{ Scheduler, SchedulerClient }, token_contract };
use soroban_sdk::{ testutils::Address as _, Vec, Address, BytesN, Env, String };
pub const ONE_DAY: u64 = 86400;
const TOKEN_WASM: &[u8] = include_bytes!(
    "../../../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
);

pub fn install_token_wasm(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(token_contract::WASM)
}

pub fn deploy_scheduler_contract<'a>(
    env: &Env,
    admin: impl Into<Option<Address>>,
    synth_market_factory: Address,
    index_factory: Address,
    keeper_accounts: impl Into<Option<Vec<Address>>>,
    protocol_fee_bps: impl Into<Option<i64>>,
    keeper_fee_bps: impl Into<Option<i64>>
) -> SchedulerClient<'a> {
    let admin = admin.into().unwrap_or(Address::generate(env));
    let scheduler = SchedulerClient::new(env, &env.register(Scheduler, ()));

    scheduler.initialize(
        &admin,
        &synth_market_factory,
        &index_factory,
        keeper_accounts,
        protocol_fee_bps,
        keeper_fee_bps
    );

    scheduler
}

// ...

pub fn install_and_deploy_token_contract<'a>(
    env: &Env,
    admin: Address,
    decimal: u32,
    name: String,
    symbol: String
) -> token_contract::Client<'a> {
    let token_addr = env.register(TOKEN_WASM, (admin, decimal, name, symbol));
    let token_client = token_contract::Client::new(env, &token_addr);

    token_client
}
