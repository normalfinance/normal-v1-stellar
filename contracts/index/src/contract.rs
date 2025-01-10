use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

use crate::{
    errors,
    storage::{ DataKey, get_admin },
    events::IndexEvents,
    index::IndexTrait,
    index_token_contract,
};

contractmeta!(key = "Description", val = "Diversified exposure to a basket of cryptocurrencies");

#[contract]
pub struct Index;

#[contractimpl]
impl IndexTrait for Index {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        name: String,
        symbol: String,
        is_public: bool,
        delegate: Option<Address>,
        fee_authority: Option<Address>,
        access_authority: Option<Address>,
        rebalance_authority: Option<Address>,
        assets: Vec<IndexAssetInfo>,
        manager_fee_bps: i64,
        revenue_share_bps: i64,
        whitelist: Option<Vec<Pubkey>>,
        blacklist: Option<Vec<Pubkey>>
    ) {
        if protocol_fee > MAX_FEE_BASIS_POINTS {
            return Err(ErrorCode::InvalidFee);
        }
        if manager_fee > MAX_FEE_BASIS_POINTS {
            return Err(ErrorCode::InvalidFee);
        }

        // Deploy the IndexToken contract
        let index_token_address = e.deploy_contract(
            &token_wasm, // WASM bytecode for the IndexToken contract
            &e.current_contract_address() // Pass Index contract address to the IndexToken init function
        );

        IndexEvents::initialize(&env, admin, index_id, name, symbol);
    }

    fn update_fees(
        env: Env,
        sender: Address,
        manager_fee_bps: Option<i64>,
        revenue_share_bps: Option<i64>
    ) {
        if index.fee_authority != sender {
            log!(&env, "Index: Update fees: You are not authorized!");
            panic_with_error!(&env, ContractError::NotAuthorized);
        }

        let mut index = get_index(&env);

        if let Some(manager_fee_bps) = manager_fee_bps {
            if expense_ratio > MAX_FEE_RATE {
                return Err(ErrorCode::OperationPaused);
            }

            validate_bps!(manager_fee_bps);
            index.manager_fee_bps = manager_fee_bps;
        }

        if let Some(revenue_share_bps) = revenue_share_bps {
            validate_bps!(revenue_share_bps);
            index.revenue_share_bps = revenue_share_bps;
        }

        save_index(&env, index);
    }

    fn update_paused_operations(e: Env, paused_operations: Vec<Operation>) {
        let mut index = get_index(&env);

        is_fund_admin(&env, index.admin);

        set_paused_operations(&e, paused_operations);
    }

    fn update_is_public(env: Env, sender: Address, is_public: bool) {
        let mut index = get_index(&env);

        is_fund_admin(&env, index.admin);

        if index.is_public == true {
            return Err(ErrorCode::Idk);
        }
        index.is_public = is_public;

        save_index(&env, index);
    }

    fn update_whitelist(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>) {
        sender.require_auth();
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let mut index = get_index(&env);

        if index.access_authority != sender {
            log!(&env, "Index: Update whitelist accounts: You are not authorized!");
            panic_with_error!(&env, ContractError::NotAuthorized);
        }

        let mut whitelist = index.whitelist;

        to_add.into_iter().for_each(|addr| {
            if !whitelist.contains(addr.clone()) {
                whitelist.push_back(addr);
            }
        });

        to_remove.into_iter().for_each(|addr| {
            if let Some(id) = whitelist.iter().position(|x| x == addr) {
                whitelist.remove(id as u32);
            }
        });

        save_index(&env, Index {
            whitelist,
            ..index
        });

        whitelist
    }

    fn update_blacklist(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>) {
        sender.require_auth();
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let mut index = get_index(&env);

        if index.access_authority != sender {
            log!(&env, "Index: Update blacklist accounts: You are not authorized!");
            panic_with_error!(&env, ContractError::NotAuthorized);
        }

        let mut blacklist = index.blacklist;

        to_add.into_iter().for_each(|addr| {
            if !blacklist.contains(addr.clone()) {
                blacklist.push_back(addr);
            }
        });

        to_remove.into_iter().for_each(|addr| {
            if let Some(id) = blacklist.iter().position(|x| x == addr) {
                blacklist.remove(id as u32);
            }
        });

        save_index(&env, Index {
            blacklist,
            ..index
        });

        blacklist
    }

    fn rebalance(env: Env, sender: Address) {
        let mut index = get_index(&env);

        if index.rebalance_authority != sender {
            log!(&env, "Index: Rebalance: You are not authorized!");
            panic_with_error!(&env, ContractError::NotAuthorized);
        }

        // ...
    }

    fn collect_fees(env: Env, sender: Address, to: Option<Address>) {
        sender.require_auth();

        let mut index = get_index(&env);

        if index.fee_authority != sender {
            log!(&env, "Index: Collect fees: You are not authorized!");
            panic_with_error!(&env, ContractError::NotAuthorized);
        }

        // fetch available to withdraw
        let can_withdraw = 0;

        // find send address
        let recipient_address = match to {
            Some(to_address) => to_address, // Use the provided `to` address
            None => sender, // Otherwise use the sender address
        };

        // transfer token
        let token_client = token::Client::new(&env, &x);
        token_client.transfer(&recipient_address, &env.current_contract_address(), &can_withdraw);

        // update balances
        // index.
    }

    // User

    fn mint(env: Env, sender: Address, to: Option<Address>, amount: i128) {
        sender.require_auth();

        // Transfer quote asset to Index
        let token_quote_client = token::Client::new(&e, &get_token_quote(&e));
        token_quote_client.transfer(&to, &e.current_contract_address(), &amount);

        // Compute asset amounts
        // ...

        // Execute swaps
        let amm = amm::Client::new(&e, &get_amm(&e));

        for acc_a in swaps_a.iter() {
            let swap_client = atomic_swap::Client::new(&env, &swap_contract);
            for i in 0..swaps_b.len() {
                let acc_b = swaps_b.get(i).unwrap();

                if acc_a.amount >= acc_b.min_recv && acc_a.min_recv <= acc_b.amount {
                    // As this is a simple 'batching' contract, there is no need
                    // for all swaps to succeed, hence we handle the failures
                    // gracefully to try and clear as many swaps as possible.
                    if
                        swap_client
                            .try_swap(
                                &acc_a.address,
                                &acc_b.address,
                                &token_a,
                                &token_b,
                                &acc_a.amount,
                                &acc_a.min_recv,
                                &acc_b.amount,
                                &acc_b.min_recv
                            )
                            .is_ok()
                    {
                        swaps_b.remove(i);
                        break;
                    }
                }
            }
        }

        // Compute appropriate # of index tokens
        let index_tokens_to_mint = 0;

        // Mint index tokens
        let token_contract: Address = env.storage().get("token_contract").unwrap();
        env.invoke_contract(&token_contract, &symbol_short!("mint"), (to.clone(), amount));

        // let client = MintClient::new(&env, &contract);
        // client.mint(&to, &index_tokens_to_mint);

        IndexEvents::index_minted(&e, index_id, to, amount);
    }

    fn redeem(env: Env, sender: Address, amount: i128) {
        sender.require_auth();

        // Burn tokens

        // Perform swaps

        // Transfer quote token back to user
        let token_contract: Address = env.storage().get("token_contract").unwrap();
        env.invoke_contract(&token_contract, &symbol_short!("transfer"), (
            from.clone(),
            env.current_contract_address(),
            amount,
        ));

        let token_quote_client = token::Client::new(&e, &get_token_quote(&e));
        token_quote_client.transfer(&from, &e.current_contract_address(), &amount);

        IndexEvents::index_redeemed(&e, index_id, from, amount);
    }

    fn collect_revenue_share(env: Env, sender: Address, to: Option<Address>) {
        sender.require_auth();

        let mut index = get_index(&env);

        // fetch available to withdraw
        let can_withdraw = 0;

        // find send address
        let recipient_address = match to {
            Some(to_address) => to_address, // Use the provided `to` address
            None => sender, // Otherwise use the sender address
        };

        // transfer token
        let token_client = token::Client::new(&env, &env);
        token_client.transfer(&recipient_address, &env.current_contract_address(), &can_withdraw);

        // update balances
        // index.
    }
}
