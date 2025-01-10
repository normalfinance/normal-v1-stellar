use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

use crate::{
    errors,
    storage::{ DataKey, get_admin },
    events::IndexEvents,
    index::IndexTrait,
    token_contract,
    index_token_contract,
    index_factory_contract,
    amm_contract,
};

use normal::oracle::{ get_oracle_price, oracle_validity };

contractmeta!(key = "Description", val = "Diversified exposure to a basket of cryptocurrencies");

#[contract]
pub struct Index;

#[contractimpl]
impl IndexTrait for Index {
    // ################################################################
    //                             ADMIN
    // ################################################################

    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        factory_addr: Address,
        name: String,
        symbol: String,
        quote_asset: Address,
        initial_price: i32,
        initial_deposit: i128,
        is_public: bool,
        active_status: IndexStatus,
        delegate: Option<Address>,
        fee_authority: Option<Address>,
        access_authority: Option<Address>,
        rebalance_authority: Option<Address>,
        assets: Vec<IndexAssetInfo>,
        manager_fee_bps: i64,
        revenue_share_bps: i64,
        whitelist: Option<Vec<Address>>,
        blacklist: Option<Vec<Address>>
    ) {
        if is_initialized(&env) {
            log!(&env, "Index: Initialize: initializing contract twice is not allowed");
            panic_with_error!(&env, ContractError::AlreadyInitialized);
        }

        validate_bps!(manager_fee_bps, revenue_share_bps);

        if manager_fee_bps > MAX_FEE_BASIS_POINTS {
            return Err(ErrorCode::InvalidFee);
        }

        set_initialized(&env);

        // Deposit initial investment
        let token_contract_client = token_contract::Client::new(&env, &quote_asset);
        token_contract_client.transfer(&admin, &env.current_contract_address(), &initial_deposit);

        let index = Index {
            delegate,
            fee_authority,
            access_authority,
            rebalance_authority,
            name,
            symbol,
            token: None,
            is_public,
            assets: [],
            status: IndexStatus::Initialized,
            paused_operations: [],
            manager_fee_bps,
            revenue_share_bps,
            initial_price,
            base_nav: 0,
            whitelist,
            blacklist,
            expiry_ts: 0,
            expiry_price: 0,
        };

        utils::save_admin_old(&env, admin);

        // Deploy and initialize index token contract
        let index_token_address = utils::deploy_index_token_contract(
            &env,
            token_wasm_hash.clone(),
            &token_a,
            &token_b,
            env.current_contract_address(),
            index_token_decimals,
            name,
            symbol
        );
        index.index_token = index_token_address;

        let initial_mint_amount = base_nav / initial_price;

        let index_token_client = index_token_client::Client::new(&env, &index_token_address);
        env.invoke_contract(&index_token_client, &symbol_short!("mint"), (
            admin.clone(),
            &initial_mint_amount,
        ));

        swap_and_update_component_balances(&env, operations, index);

        save_index(&env, index);

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

    fn update_rebalance_threshold(env: Env, rebalance_threshold: i64) {
        let mut index = get_index(&env);

        // is_fund_admin(&env, index.admin);

        index.rebalance_threshold = rebalance_threshold;

        save_index(&env, index)
    }

    fn update_weights(env: Env, sender: Address, update: Vec<IndexAsset>) {
        let mut index = get_index(&env);

        // Public indexes can only be updated via a successful DAO vote
        if index.is_public {
            is_governor(&env, sender);
        }

        let now = env.ledger().timestamp();

        // check time against update threshold
        if now - index.rebalance_ts < index.rebalance_threshold {
            return Err(ErrorCode::TooSoonToRebalance);
        }

        // Compute weight deltas b/t current and updated weights
        // ..

        let buy_operations = [];
        let sell_operations = [];

        // Sell all negative deltas
        swap_and_update_component_balances(&env, buy_operations, index);

        // Buy all positive deltas
        swap_and_update_component_balances(&env, sell_operations, index);

        // Update index
        index.component_weights = weights;
        index.rebalance_ts = now;
        index.last_updated_ts = now;

        save_index(&env, index);

        IndexEvents::weight_update(&env, index.name, sender);
    }

    /**
     * Public indexes CAN be rebalanced, but...
     *
     * Weight reduced = sell via AMM
     * Weight increased = buy via AMM
     *
     * 1) how can we disable nefarious index managers from fucking people over (via price changes)?
     * 2) who can make public indexes?
     */

    fn rebalance(env: Env, sender: Address, update: Vec<IndexAsset>) {
        let mut index = get_index(&env);

        if index.rebalance_authority != sender {
            log!(&env, "Index: Rebalance: You are not authorized!");
            panic_with_error!(&env, ContractError::NotAuthorized);
        }

        // TODO: weight change guardrails to avoid massive spikes in price

        // ...

        // Update component balances
        // ...

        // Update index
        index.rebalance_ts = now;
        index.last_updated_ts = now;

        save_index(&env, index);

        IndexEvents::rebalance(&env, index.name, sender);
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
        let token_contractclient = token_contract::Client::new(&env, &x);
        token_client.transfer(&recipient_address, &env.current_contract_address(), &can_withdraw);

        // update balances
        // index.
    }

    // ################################################################
    //                             USER
    // ################################################################

    fn mint(env: Env, sender: Address, index_token_amount: i128, to: Option<Address>) {
        if index_token_amount <= 0 {
            return Err(ErrorCode::InsufficientDeposit);
        }

        sender.require_auth();

        // Get index and price
        let index = get_index(&env);
        let index_price = get_index_price(&env, index);

        // Compute amount of quote asset needed
        let quote_token_amount = convert_index_token_amount_to_quote_amount(
            &env,
            index_token_amount,
            index_price
        );

        // Deposit quote asset
        let quote_token_client = token_contract::Client::new(&env, &index.quote_asset);
        quote_token_client.transfer(&sender, &env.current_contract_address(), &quote_token_amount);

        // Compute asset amounts / swaps
        let operations: Vec<Swap> = [];

        index.assets.iter().for_each(|asset| {
            //
            let amount = 0;

            let swap = Swap {
                ask_asset: &asset.market_address,
                offer_asset: "XLM",
                ask_asset_min_amount: &amount,
            };

            operations.push_back(swap);
        });

        swap_and_update_component_balances(&env, operations, index);

        // Mint index tokens
        let recipient_address = match to {
            Some(to_address) => to_address, // Use the provided `to` address
            None => sender, // Otherwise use the sender address
        };

        let index_token_client = index_token_client::Client::new(&env, &index.index_token);
        env.invoke_contract(&index_token_client, &symbol_short!("mint"), (
            recipient_address.clone(),
            &index_token_amount,
        ));

        IndexEvents::mint(&env, index.name, sender, recipient_address, index_token_amount);
    }

    fn redeem(env: Env, sender: Address, index_token_amount: i128, to: Option<Address>) {
        if amount <= 0 {
            return Err(ErrorCode::InsufficientDeposit);
        }

        sender.require_auth();

        // Get index and price
        let index = get_index(&env);
        let index_price = get_index_price(&env, index);

        // Compute amount of quote asset needed
        let quote_token_amount = convert_index_token_amount_to_quote_amount(
            &env,
            index_token_amount,
            index_price
        );

        // Ensure sufficient quote funds
        let quote_token_client = token_contract::Client::new(&env, &index.quote_token);
        let quote_balance = quote_token_client.balance(&sender);
        if quote_balance < quote_token_amount {
            return Err(ErrorCode::InsufficientFunds);
        }

        // Burn tokens
        let index_token_client = index_token_client::Client::new(&env, &index.index_token);
        env.invoke_contract(&index_token_client, &symbol_short!("burn"), (to.clone(), amount));

        // Compute asset amounts / swaps
        let operations: Vec<Swap> = [];

        index.assets.iter().for_each(|asset| {
            let amount = 0;

            let swap = Swap {
                ask_asset: &asset.market_address,
                offer_asset: "XLM",
                ask_asset_min_amount: &amount,
            };

            operations.push_back(swap);
        });

        swap_and_update_component_balances(&env, operations);

        // Transfer quote token back to user
        let recipient_address = match to {
            Some(to_address) => to_address, // Use the provided `to` address
            None => sender, // Otherwise use the sender address
        };

        quote_token_client.transfer(&env.current_contract_address(), &recipient_address, &amount);

        IndexEvents::redeem(&env, index_id, from, amount);
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

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_index(env: Env) -> Index {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        get_index(&env)
    }

    fn query_price(env: Env) -> i128 {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let index = get_index(&env);

        get_index_price(&env, index);
    }

    fn query_nav(env: Env) -> i128 {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let index = get_index(&env);

        calculate_current_nav(&env, index);
    }
}

fn convert_index_token_amount_to_quote_amount(
    &env,
    index_token_amount: i128,
    index_price: i128
) -> (i128, i128) {
    // Get quote asset price
    let oracle_price_data = get_oracle_price(
        &env,
        index.oracle_source,
        index.oracle,
        index.quote_asset,
        "USD"
    );

    let oracle_validity = oracle_validity(
        market.name,
        risk_ema_price,
        oracle_price_data,
        oracle_guard_rails().validity, // import from Oracle module
        market.get_max_confidence_interval_multiplier()?,
        false
    )?;

    validate!(
        is_oracle_valid_for_action(oracle_validity, action)?,
        ErrorCode::InvalidOracle,
        "Invalid Oracle ({:?} vs ema={:?}) for perp market index={} and action={:?}",
        oracle_price_data,
        risk_ema_price,
        market.name,
        action
    )?;

    // Compute amount of quote asset needed
    let quote_token_amount = (index_price * index_token_amount) / oracle_price_data.price;

    (quote_token_amount, oracle_price_data.price)
}

fn get_index_price(&env, index: Index) -> u128 {
    let current_nav = calculate_current_nav(&env, index.component_balances);

    let price = (current_nav / index.base_nav) * index.initial_price;

    price
}

fn calculate_current_nav(env: Env, component_balances: Map<Address, u128>) -> u128 {
    let nav = 0;

    component_balances.iter().for_each(|(token_address, token_balance)| {
        // TODO: Fetch the asset price from the synth token AMM
        let price = 0;

        // Add total value to NAV
        nav += token_balance * price;
    });

    nav
}

fn swap_and_update_component_balances(env: Env, operations: Vec<Swap>, index: Index) {
    let index_factory_client = index_factory_contract::Client::new(&env, &get_factory(&env));

    operations.iter().for_each(|op| {
        let amm_addr: Address = index_factory_client.query_for_amm_by_market(&op.clone().asset);

        let amm_client = amm_contract::Client::new(&env, &amm_addr);

        swap_response = amm_client.swap(
            &recipient,
            &op.offer_asset,
            &next_offer_amount,
            &op.ask_asset_min_amount,
            &max_spread_bps,
            &max_allowed_fee_bps
        );

        let signed_amount = util(swap_response);

        index.component_balances[op.asset] += signed_amount;
    });
}
