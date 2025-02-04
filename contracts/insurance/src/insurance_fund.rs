use soroban_sdk::{Address, BytesN, Env, String};

use crate::storage::{InsuranceFund, Stake};

pub trait InsuranceFundTrait {
    // ################################################################
    //                             Admin
    // ################################################################

    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        governor_contract: Address,
        deposit_token: Address,
        token_wasm_hash: BytesN<32>,
        stake_token_decimals: u32,
        stake_token_name: String,
        stake_token_symbol: String,
        max_buffer_balance: i128,
    );

    fn deposit_revenue(env: Env);

    // ################################################################
    //                             User
    // ################################################################

    fn add_if_stake(env: Env, sender: Address, amount: i128);

    fn request_remove_if_stake(env: Env, sender: Address, amount: i128);

    fn cancel_request_remove_if_stake(env: Env, sender: Address);

    fn remove_if_stake(env: Env, sender: Address);

    // ################################################################
    //                             Queries
    // ################################################################

    fn query_insurance_fund(env: Env) -> InsuranceFund;

    fn query_admin(env: Env) -> Address;

    fn query_if_stake(env: Env, address: Address) -> Stake;
}
