use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

use crate::{ errors, storage::{ get_admin }, storage_types::{ DataKey } };

contractmeta!(key = "Description", val = "Constant product AMM with a .3% swap fee");

#[contract]
struct SynthMarket;

#[contractimpl]
impl SynthMarket {
    pub fn __constructor(e: Env, reflector_contract_id: Address, token_wasm_hash: BytesN<32>) {
        if token_a >= token_b {
            panic!("token_a must be less than token_b");
        }

        // create the price oracle client instance
        let reflector_contract = PriceOracleClient::new(&env, &reflector_contract_id);

        // get oracle prcie precision
        let decimals = reflector_contract.decimals();

        // let share_contract = create_share_token(&e, token_wasm_hash, &token_a, &token_b);

        put_token_a(&e, token_a);
    }

    pub fn set_fee_rate(e: Env, fee_rate: u128) -> u128 {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_fee_rate(&e, fee_rate);

        publish_updated_event(&e, &symbol_short!("fee"), fee);
    }

    pub fn freeze_oracle(e: Env) -> u128 {}

    pub fn init_shutdown(e: Env) -> u128 {}

     pub fn update_debt_floor(e: Env) -> u128 {}

     pub fn update_debt_ceiling(e: Env) -> u128 {}

    pub fn liquidate(e: Env, fee_rate: u128) -> u128 {}

    pub fn liquidate(e: Env, fee_rate: u128) -> u128 {}
}
