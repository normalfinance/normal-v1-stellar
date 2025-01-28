use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, Env, Symbol};

pub fn deploy_index_token_contract(
    env: &Env,
    wasm_hash: BytesN<32>,
    name: &Symbol,
    symbol: &Symbol,
) -> Address {
    let mut salt = Bytes::new(env);
    salt.append(&name.to_xdr(env));
    salt.append(&symbol.to_xdr(env));
    let salt = env.crypto().sha256(&salt);

    env.deployer()
        .with_current_contract(salt)
        .deploy_v2(wasm_hash, ())
}
