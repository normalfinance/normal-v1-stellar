use soroban_sdk::{ Address, BytesN, Env, String };

use crate::storage::{ Config, InsuranceFund, Stake };

pub trait InsuranceFundTrait {
    // ################################################################
    //                             ADMIN
    // ################################################################

    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        governor: Address,
        governance_token: Address,
        stake_asset: Address,
        token_wasm_hash: BytesN<32>,
        share_token_decimals: u32,
        share_token_name: String,
        share_token_symbol: String,
        max_buffer_balance: i128
    );

    // ################################################################
    //                             USER
    // ################################################################

    fn add_if_stake(env: Env, sender: Address, amount: u64);

    fn request_remove_if_stake(env: Env, sender: Address, amount: u64);

    fn cancel_request_remove_if_stake(env: Env, sender: Address);

    fn remove_if_stake(env: Env, sender: Address);

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_config(env: Env) -> Config;

    fn query_if(env: Env) -> InsuranceFund;

    fn query_if_stake(env: Env, address: Address) -> Stake;
}
