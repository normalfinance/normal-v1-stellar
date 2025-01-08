use soroban_sdk::{ xdr::ToXdr, Address, Bytes, BytesN, Env, IntoVal, Symbol, Val, Vec };

pub fn deploy_index_contract(
    env: &Env,
    wasm_hash: BytesN<32>,
    index_token_symbol: &Symbol,
) -> Address {
    let mut salt = Bytes::new(env);
    // salt.append(&token_a.to_xdr(env));
    // salt.append(&token_b.to_xdr(env));
    salt.append(&index_token_symbol.to_xdr(env));
    let salt = env.crypto().sha256(&salt);

    env.deployer().with_current_contract(salt).deploy_v2(wasm_hash, ())
}
