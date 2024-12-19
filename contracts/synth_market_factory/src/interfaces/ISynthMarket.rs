use soroban_sdk::{ symbol_short, vec, Address, BytesN, Env, Val, Vec };

pub mod ISynthMarket {
    soroban_sdk::contractimport!(file = "../../target/wasm32-unknown-unknown/release/pair.wasm");
}

pub fn deploy_synth_market(e: Env, name: String) -> (Address, Val) {
    let wasm_hash = e.deployer().upload_contract_wasm(ISynthMarket::WASM);

    let salt = BytesN::from_array(&e, &[0; 32]);

    let init_fn = symbol_short!("init");
    let init_fn_args: Vec<Val> = vec![&e, token0.to_val(), token1.to_val(), factory.to_val()];

    // Deploy the contract using the uploaded Wasm with given hash.
    let deployed_address = e
        .deployer()
        .with_address(e.current_contract_address(), salt)
        .deploy(wasm_hash);

    // Invoke the init function with the given arguments.
    let res: Val = e.invoke_contract(&deployed_address, &init_fn, init_fn_args);

    // Return the contract ID of the deployed contract and the result of
    // invoking the init result.
    (deployed_address, res)
}
