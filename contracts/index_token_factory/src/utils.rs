use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, Env, String};

pub fn deploy_index_token_contract(
    env: &Env,
    wasm_hash: BytesN<32>,
    name: &String,
    symbol: &String,
) -> Address {
    let mut salt = Bytes::new(env);
    salt.append(&name.clone().to_xdr(env));
    salt.append(&symbol.clone().to_xdr(env));
    let salt = env.crypto().sha256(&salt);

    env.deployer()
        .with_current_contract(salt)
        .deploy_v2(wasm_hash, ())
}
