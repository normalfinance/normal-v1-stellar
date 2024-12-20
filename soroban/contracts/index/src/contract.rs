use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

use crate::{
    errors,
    interfaces::{ IInsuranceFund::IInsuranceFund },
    storage::{ get_admin },
    storage_types::{ DataKey },
};

// workspace method
// use soroban_workspace_contract_a_interface::ContractAClient;

mod state {
    soroban_sdk::contractimport!(
        file = "../state/target/wasm32-unknown-unknown/release/state_contract.wasm"
    );
}

mod amm {
    soroban_sdk::contractimport!(
        file = "../amm/target/wasm32-unknown-unknown/release/amm_contract.wasm"
    );
}

#[contract]
pub struct Index;

#[contractimpl]
impl Index {
    fn init(
        e: Env,
        admin: Address,
        name: String,
        assets: u64,
        privacy: bool,
        expense_ratio: u64,
        revenue_share: u64,
        max_insurance: u64,
        whitelist: Vec<Address>,
        blacklist: Vec<Address>
    ) {
        // todo: already initiazed check
        //
        set_admin(&e, admin);
        set_max_insurance(&e, max_insurance);
        set_unstaking_period(&e, unstaking_period);
        set_paused_operations(&e, paused_operations);
    }

    // fn get_admin(e: Env) -> Address {
    //     get_admin(&e)
    // }

    // Getters

    // fn get_max_insurance(e: Env) -> u64 {
    //     get_max_insurance(&e)
    // }

    // Setters

    fn update_expense_ratio(e: Env, expense_ratio: u64) {
        is_fund_admin(&e);

        if expense_ratio > MAX_FEE_RATE {
            return Err(ErrorCode::OperationPaused);
        }

        set_expense_ratio(&e, expense_ratio);
    }

    fn update_revenue_share(e: Env, revenue_share: u64) {
        is_fund_admin(&e);
        set_revenue_share(&e, revenue_share);
    }

    fn update_expiry(e: Env, expiry: u64) {
        is_fund_admin(&e);
        set_expiry(&e, expiry);
    }

    fn update_paused_operations(e: Env, paused_operations: Vec<Operation>) {
        is_fund_admin(&e);
        set_paused_operations(&e, paused_operations);
    }

    fn update_privacy(e: Env, private: bool) {
        is_fund_admin(&e);

        let privacy = get_privacy(&e);
        if privacy == true {
            return Err(ErrorCode::Idk);
        }

        set_privacy(&e, private);
    }

    fn update_whitelist(e: Env, whitelist: Vec<Address>) {
        is_fund_admin(&e);
        set_max_insurance(&e, max_insurance);
    }

    fn update_blacklist(e: Env, blacklist: Vec<Address>) {
        is_fund_admin(&e);
        set_max_insurance(&e, max_insurance);
    }

    fn mint_index_tokens(e: Env, to: Address, amount: u64) {
        to.require_auth();

        // Transfer quote asset to Index
        let token_quote_client = token::Client::new(&e, &get_token_quote(&e));
        token_quote_client.transfer(&to, &e.current_contract_address(), &amount);

        // Compute asset amounts
        // ...

        // Execute swaps
        let amm = amm::Client::new(&e, &get_amm(&e));

        // Compute appropriate # of index tokens
        let index_tokens_to_mint = 0;

        // Mint index tokens
        let client = MintClient::new(&env, &contract);
        client.mint(&to, &index_tokens_to_mint);
    }

    fn redeem_index_tokens(e: Env, from: Address, amount: u64) {
        from.require_auth();

        // Burn tokens

        // Perform swaps

        // Transfer quote token back to user
        let token_quote_client = token::Client::new(&e, &get_token_quote(&e));
        token_quote_client.transfer(&from, &e.current_contract_address(), &amount);
    }

    fn rebalance_index(e: Env) {
        // let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        index.rebalance_authority.require_auth();

        let state = state::Client::new(&env, &contract);
        if num_assets > state.max_index_assets {
            return Err(ErrorCode::TooManyAssets);
        }
    }
}
