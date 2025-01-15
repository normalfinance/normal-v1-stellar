use soroban_sdk::{ Address, BytesN, Env, String };

use crate::storage::{ Config, InsuranceFund, Stake };

pub trait InsuranceFundTrait {
    // ################################################################
    //                             ADMIN
    // ################################################################

    fn initialize(
        env: Env,
        admin: Address,
        governor: Address,
        stake_asset: Address,
        token_wasm_hash: BytesN<32>,
        share_token_decimals: u32,
        share_token_name: String,
        share_token_symbol: String
    );

    // ################################################################
    //                             USER
    // ################################################################

    fn add_stake(env: Env, sender: Address, amount: u64);

    fn request_remove_stake(env: Env, sender: Address, amount: u64);

    fn cancel_request_remove_stake(env: Env, sender: Address);

    fn remove_stake(env: Env, sender: Address);

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_config(env: Env) -> Config;

    fn query_insurance_fund(env: Env) -> InsuranceFund;

    fn query_stake(env: Env, address: Address) -> Stake;
}
