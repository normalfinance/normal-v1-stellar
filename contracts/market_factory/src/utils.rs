use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, Env, String};

pub fn deploy_market_contract(
    env: &Env,
    wasm_hash: BytesN<32>,
    oracle: &Address,
    name: &String,
    symbol: &String,
) -> Address {
    let mut salt = Bytes::new(env);
    salt.append(&oracle.clone().to_xdr(env));
    salt.append(&name.clone().to_xdr(env));
    salt.append(&symbol.clone().to_xdr(env));
    let salt = env.crypto().sha256(&salt);

    env.deployer()
        .with_current_contract(salt)
        .deploy_v2(wasm_hash, ())
}

#[allow(clippy::too_many_arguments)]
pub fn deploy_synthetic_token_contract(
    env: &Env,
    token_wasm_hash: BytesN<32>,
    token_quote: &Address,
    admin: Address,
    decimals: u32,
    name: String,
    symbol: String,
) -> Address {
    let mut salt = Bytes::new(env);
    salt.append(&token_quote.clone().to_xdr(env));
    let salt = env.crypto().sha256(&salt);

    env.deployer()
        .with_current_contract(salt)
        .deploy_v2(token_wasm_hash, (admin, decimals, name, symbol))
}

#[allow(clippy::too_many_arguments)]
pub fn deploy_lp_token_contract(
    env: &Env,
    token_wasm_hash: BytesN<32>,
    token_a: &Address,
    token_b: &Address,
    admin: Address,
    decimals: u32,
    name: String,
    symbol: String,
) -> Address {
    let mut salt = Bytes::new(env);
    salt.append(&token_a.clone().to_xdr(env));
    salt.append(&token_b.clone().to_xdr(env));
    let salt = env.crypto().sha256(&salt);

    env.deployer()
        .with_current_contract(salt)
        .deploy_v2(token_wasm_hash, (admin, decimals, name, symbol))
}
