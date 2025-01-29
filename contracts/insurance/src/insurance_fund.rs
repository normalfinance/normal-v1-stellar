use normal::error::NormalResult;
use soroban_sdk::{ Address, BytesN, Env, String };

use crate::storage::InsuranceFund;

pub trait InsuranceFundTrait {
    // ################################################################
    //                             Admin
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
    //                             User
    // ################################################################

    fn add_if_stake(env: Env, sender: Address, amount: i128) -> NormalResult;

    fn request_remove_if_stake(env: Env, sender: Address, amount: i128) -> NormalResult;

    fn cancel_request_remove_if_stake(env: Env, sender: Address) -> NormalResult;

    fn remove_if_stake(env: Env, sender: Address) -> NormalResult;

    // ################################################################
    //                             Queries
    // ################################################################

    fn query_insurance_fund(env: Env) -> InsuranceFund;

    // fn query_if_stake(env: Env, address: Address) -> Stake;
}
