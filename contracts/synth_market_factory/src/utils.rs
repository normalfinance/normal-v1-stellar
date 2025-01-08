use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, Env, IntoVal, Symbol, Val, Vec};

pub fn deploy_synth_market_contract(
    env: &Env,
    wasm_hash: BytesN<32>,
    token_a: &Address,
    token_b: &Address,
) -> Address {
    let mut salt = Bytes::new(env);
    salt.append(&token_a.to_xdr(env));
    salt.append(&token_b.to_xdr(env));
    let salt = env.crypto().sha256(&salt);

    env.deployer()
        .with_current_contract(salt)
        .deploy_v2(wasm_hash, ())
}