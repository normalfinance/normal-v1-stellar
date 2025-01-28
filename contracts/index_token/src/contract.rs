use crate::allowance::{read_allowance, spend_allowance, write_allowance};
use crate::balance::{read_balance, receive_balance, spend_balance};
use crate::metadata::{read_decimal, read_name, read_symbol, write_metadata};
use crate::msg::IndexResponse;
use crate::storage::{get_index, get_last_transfer, Swap, TransferWithFees, USD, XLM};

use normal::error::{ErrorCode, NormalResult};
use normal::math::casting::Cast;
use normal::math::safe_math::SafeMath;
use normal::types::{IndexAsset, IndexParams};
use soroban_sdk::token::{self, Interface as _};
use soroban_sdk::{
    contract, contractimpl, contractmeta, log, panic_with_error, vec, Address, Env, Map, String,
    Symbol, Vec,
};
use soroban_token_sdk::metadata::TokenMetadata;
use soroban_token_sdk::TokenUtils;

use crate::{
    events::IndexTokenEvents,
    // index_factory_contract,
    index_token::IndexTokenTrait,
    storage::{save_index, utils, Index, IndexOperation},
};
use normal::oracle::get_oracle_price;

use normal::{
    constants::{INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD, SECONDS_IN_A_YEAR},
    validate_bps,
};

contractmeta!(
    key = "Description",
    val = "Diversified exposure to a basket of cryptocurrencies"
);

#[contract]
pub struct IndexToken;

#[contractimpl]
impl IndexTokenTrait for IndexToken {
    fn initialize(
        env: Env,
        admin: Address,
        factory: Address,
        initial_deposit: i128,
        params: IndexParams,
    ) -> Result<(), ErrorCode> {
        if params.decimal > 18 {
            panic!("Decimal must not be greater than 18");
        }

        if utils::is_initialized(&env) {
            log!(
                &env,
                "Index Token: Initialize: initializing contract twice is not allowed"
            );
            panic_with_error!(&env, ErrorCode::AlreadyInitialized);
        }

        let now = env.ledger().timestamp();
        let oracle = params.oracle;
        let oracle_source = params.oracle_source;

        // Verify oracle is readable
        let oracle_price_data = get_oracle_price(&env, &oracle_source, &oracle, (XLM, USD), now)?;

        validate_bps!(params.manager_fee_bps);

        utils::set_initialized(&env);
        utils::save_admin(&env, &admin.clone());
        utils::save_factory(&env, &factory);
        write_metadata(
            &env,
            TokenMetadata {
                decimal: params.decimal,
                name: params.name.clone(),
                symbol: params.symbol.clone(),
            },
        );

        let base_nav =
            initial_deposit.safe_mul(oracle_price_data.price.cast::<i128>(&env)?, &env)?;

        let index = Index {
            quote_token: params.quote_token.clone(),
            oracle: oracle.clone(),
            oracle_source,
            is_public: params.is_public,
            paused_operations: Vec::new(&env),
            manager_fee_bps: params.manager_fee_bps,
            whitelist: params.whitelist,
            blacklist: params.blacklist,
            base_nav,
            initial_price: params.initial_price,
            component_balances: Map::new(&env),
            component_balance_update_ts: now,
            component_assets: params.component_assets,
            rebalance_threshold: params.rebalance_threshold,
            rebalance_ts: now,
            last_updated_ts: now,
            total_fees: 0,
            total_mints: 0,
            total_redemptions: 0,
        };

        save_index(&env, index);

        let initial_mint_amount = base_nav.safe_div(params.initial_price, &env)?;
        let _ = Self::mint(env, admin, initial_mint_amount);

        // IndexTokenEvents::initialize(&env, admin, name, symbol);

        Ok(())
    }

    fn update_manager_fee(env: Env, sender: Address, manager_fee_bps: i64) {
        sender.require_auth();

        validate_bps!(manager_fee_bps);

        utils::is_admin(&env, sender);

        let mut index = get_index(&env);

        index.manager_fee_bps = manager_fee_bps;

        save_index(&env, index);
        // save_index(&env, Index {
        //     manager_fee_bps,
        //     ..index
        // });
    }

    fn update_paused_operations(
        env: Env,
        sender: Address,
        to_add: Vec<IndexOperation>,
        to_remove: Vec<IndexOperation>,
    ) {
        sender.require_auth();
        utils::is_admin(&env, sender);

        let index = get_index(&env);
        let mut paused_operations = index.paused_operations;

        to_add.into_iter().for_each(|op| {
            if !paused_operations.contains(op) {
                paused_operations.push_back(op);
            }
        });

        to_remove.into_iter().for_each(|op| {
            if let Some(id) = paused_operations.iter().position(|x| x == op) {
                paused_operations.remove(id as u32);
            }
        });

        save_index(
            &env,
            Index {
                paused_operations,
                ..index
            },
        );
    }

    fn update_whitelist(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>) {
        sender.require_auth();
        utils::is_admin(&env, sender);

        let index: Index = get_index(&env);
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

        save_index(&env, Index { whitelist, ..index });
    }

    fn update_blacklist(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>) {
        sender.require_auth();
        utils::is_admin(&env, sender);

        let index: Index = get_index(&env);
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

        save_index(&env, Index { blacklist, ..index });
    }

    fn update_rebalance_threshold(env: Env, sender: Address, rebalance_threshold: u64) {
        sender.require_auth();

        let mut index = get_index(&env);

        if index.is_public {
            utils::is_governor(&env, sender);
        } else {
            utils::is_admin(&env, sender);
        }

        index.rebalance_threshold = rebalance_threshold;
        save_index(&env, index);
        // save_index(&env, Index {
        //     rebalance_threshold,
        //     ..index
        // });
    }

    // ################################################################
    //                             KEEPER
    // ################################################################

    fn rebalance(env: Env, sender: Address, updated_assets: Vec<IndexAsset>) {
        sender.require_auth();

        let index = get_index(&env);

        if index.is_public {
            utils::is_governor(&env, sender.clone());
            // TODO: weight change guardrails to avoid massive spikes in price
        } else {
            utils::is_admin(&env, sender.clone());
        }

        let now = env.ledger().timestamp();
        if !index.can_rebalance(now) {
            panic_with_error!(&env, ErrorCode::TooSoonToRebalance);
        }

        // TODO: validate updated asset markets

        let position_increases: Vec<Swap> = Vec::new(&env);
        let position_reductions: Vec<Swap> = Vec::new(&env);

        // updated_assets.iter().for_each(|updated_asset| {
        //     let current_asset = index.component_assets
        //         .iter()
        //         .find(|current_asset| current_asset.market == updated_asset.market)
        //         .cloned();

        //     match current_asset {
        //         Some(current_asset) => {
        //             let percent_delta = updated_asset.weight - current_asset.weight;
        //             let amount_delta = 0;

        //             if delta > 0 {
        //                 position_increases.push(Swap {
        //                     ask_asset: updated_asset.market, // Buy the asset
        //                     offer_asset: index.quote_asset,
        //                     ask_asset_min_amount: amount_delta,
        //                 })
        //             } else {
        //                 position_reductions.push(Swap {
        //                     ask_asset: index.quote_asset,
        //                     offer_asset: updated_asset.market, // Sell the asset
        //                     ask_asset_min_amount: amount_delta,
        //                 })
        //             }
        //         }
        //         None => {
        //             let amount_delta = 0;

        //             position_increases.push(Swap {
        //                 ask_asset: updated_asset.market, // Buy the asset
        //                 offer_asset: index.quote_asset,
        //                 ask_asset_min_amount: amount_delta,
        //             })
        //         }
        //     }
        // });

        swap_and_update_component_balances(&env, position_increases, &index);
        swap_and_update_component_balances(&env, position_reductions, &index);

        save_index(
            &env,
            Index {
                component_balances: Map::new(&env), // TODO:
                component_assets: Vec::new(&env),   // TODO:
                rebalance_ts: now,
                last_updated_ts: now,
                ..index
            },
        );

        IndexTokenEvents::rebalance(&env, sender, updated_assets);
    }

    // ################################################################
    //                             USER
    // ################################################################

    fn mint(env: Env, sender: Address, index_token_amount: i128) -> NormalResult {
        utils::check_nonnegative_amount(index_token_amount);
        sender.require_auth();

        let now = env.ledger().timestamp();

        // Get index and price
        let index = get_index(&env);
        if !index.can_invest(sender.clone()) {
            panic_with_error!(&env, ErrorCode::AdminNotSet);
        }

        let index_price = get_index_price(&env, &index)?;

        // Compute amount of quote asset needed
        let (quote_token_amount, _price) = convert_index_token_amount_to_quote_amount(
            &env,
            &index,
            index_token_amount,
            index_price,
            now,
        )?;

        // Deposit initial investment
        utils::transfer_token(
            &env,
            &index.quote_token,
            &sender,
            &env.current_contract_address(),
            quote_token_amount,
        );

        // Compute asset amounts / swaps
        let _operations: Vec<Swap> = Vec::new(&env);

        // TODO:
        // for (k, v) in index.component_assets.iter() {
        //     //
        //     let amount = 0;

        //     let swap = Swap {
        //         ask_asset: &asset.market_address,
        //         offer_asset: "XLM",
        //         ask_asset_min_amount: &amount,
        //     };

        //     operations.push_back(swap);
        // };

        // swap_and_update_component_balances(&env, operations, index);

        // Mint index tokens
        receive_balance(&env, sender.clone(), index_token_amount);
        TokenUtils::new(&env).events().mint(
            utils::get_admin(&env),
            sender.clone(),
            index_token_amount,
        );

        IndexTokenEvents::mint(&env, sender, index_token_amount);

        Ok(())
    }

    fn redeem(env: Env, sender: Address, index_token_amount: i128) -> NormalResult {
        utils::check_nonnegative_amount(index_token_amount);
        sender.require_auth();

        let now = env.ledger().timestamp();

        // Get index and price
        let index = get_index(&env);
        let index_price = get_index_price(&env, &index)?;

        // Compute amount of quote asset needed
        let (quote_token_amount, _price) = convert_index_token_amount_to_quote_amount(
            &env,
            &index,
            index_token_amount,
            index_price,
            now,
        )?;

        // Ensure sufficient quote funds
        let balance = utils::get_token_balance(&env, &index.quote_token, &sender);
        if balance < quote_token_amount {
            panic_with_error!(&env, ErrorCode::InsufficientFunds);
        }

        // Compute asset amounts / swaps
        let _operations: Vec<Swap> = Vec::new(&env);

        // index.component_assets.iter().for_each(|asset| {
        //     let amount = 0;

        //     let swap = Swap {
        //         ask_asset: &asset.market_address,
        //         offer_asset: "XLM",
        //         ask_asset_min_amount: &amount,
        //     };

        //     operations.push_back(swap);
        // });

        // swap_and_update_component_balances(&env, operations, index);

        utils::transfer_token(
            &env,
            &index.quote_token,
            &env.current_contract_address(),
            &sender,
            quote_token_amount,
        );

        // IndexTokenEvents::redeem(&env, sender, index_token_amount);

        // Burn index tokens
        Self::burn(env, sender.clone(), index_token_amount);

        Ok(())
    }

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_index(env: Env) -> IndexResponse {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        IndexResponse {
            index: get_index(&env),
        }
    }

    // fn query_price(env: Env) -> i128 {
    //     env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

    //     let index = get_index(&env);

    //     get_index_price(&env, index);
    // }

    // fn query_nav(env: Env) -> i128 {
    //     env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

    //     let index = get_index(&env);

    //     calculate_current_nav(&env, index);
    // }
}

fn annualize_fee_amount(env: &Env, amount: i128, fee: i64, time_held: u64) -> NormalResult<i128> {
    let fee = amount
        .safe_mul(fee.cast::<i128>(env)?, env)?
        .safe_mul(time_held.cast::<i128>(env)?, env)?
        .safe_div(SECONDS_IN_A_YEAR.cast::<i128>(env)?, env)?;

    Ok(fee)
}

#[contractimpl]
impl token::Interface for IndexToken {
    fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_allowance(&env, from, spender).amount
    }

    fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        utils::check_nonnegative_amount(amount);
        from.require_auth();

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        write_allowance(
            &env,
            from.clone(),
            spender.clone(),
            amount,
            expiration_ledger,
        );
        TokenUtils::new(&env)
            .events()
            .approve(from, spender, amount, expiration_ledger);
    }

    fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_balance(&env, id)
    }

    fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        utils::check_nonnegative_amount(amount);

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let _ = transfer_with_index_fee(&env, from, to, amount);
    }

    fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();

        utils::check_nonnegative_amount(amount);

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        spend_allowance(&env, from.clone(), spender, amount);
        let _ = transfer_with_index_fee(&env, from, to, amount);
    }

    fn burn(env: Env, from: Address, amount: i128) {
        from.require_auth();

        utils::check_nonnegative_amount(amount);

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        spend_balance(&env, from.clone(), amount);
        TokenUtils::new(&env).events().burn(from, amount);
    }

    fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();

        utils::check_nonnegative_amount(amount);

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        spend_allowance(&env, from.clone(), spender, amount);
        spend_balance(&env, from.clone(), amount);
        TokenUtils::new(&env).events().burn(from, amount)
    }

    fn decimals(env: Env) -> u32 {
        read_decimal(&env)
    }

    fn name(env: Env) -> String {
        read_name(&env)
    }

    fn symbol(env: Env) -> String {
        read_symbol(&env)
    }
}

fn check_fee_exemption(env: &Env, user: &Address) -> NormalResult<bool> {
    Ok(user.eq(&utils::get_admin(env)) || user.eq(&env.current_contract_address()))
}

fn transfer_with_index_fee(env: &Env, from: Address, to: Address, amount: i128) -> NormalResult {
    let transfer_info = calculate_fees(env, &from, amount)?;

    let factory_addr = utils::get_factory(env);
    let manager_addr = utils::get_admin(env); // Assumes admin is manager

    spend_balance(env, from.clone(), amount);
    receive_balance(env, to.clone(), transfer_info.net_amount);
    TokenUtils::new(env)
        .events()
        .transfer(from, to, transfer_info.net_amount);

    if transfer_info.protocol_fee_amount > 0 {
        receive_balance(env, factory_addr.clone(), transfer_info.protocol_fee_amount);
    }

    if transfer_info.manager_fee_amount > 0 {
        receive_balance(env, manager_addr.clone(), transfer_info.manager_fee_amount);
    }

    Ok(())
}

fn calculate_fees(env: &Env, from: &Address, amount: i128) -> NormalResult<TransferWithFees> {
    if check_fee_exemption(env, from)? {
        // No fees applied, transfer the full amount
        return Ok(TransferWithFees {
            protocol_fee_amount: 0,
            manager_fee_amount: 0,
            total_fees: 0,
            net_amount: amount,
        });
    }

    let last_transfer = get_last_transfer(env, from);

    if last_transfer.balance == 0 {
        // No fee if there was no prior balance
        return Ok(TransferWithFees {
            protocol_fee_amount: 0,
            manager_fee_amount: 0,
            total_fees: 0,
            net_amount: amount,
        });
    }

    let index = get_index(env);

    // Get the protocol fee
    let protocol_fee: i64 = env.invoke_contract(
        &utils::get_factory(env),
        &Symbol::new(env, "query_protocol_fee"),
        vec![&env],
    );

    // Calculate weighted holding time
    let time_held = env.ledger().timestamp() - last_transfer.ts;

    // Prorated fee calculation
    let protocol_fee_amount = annualize_fee_amount(env, amount, protocol_fee, time_held)?;
    let manager_fee_amount = annualize_fee_amount(env, amount, index.manager_fee_bps, time_held)?;

    let total_fees = protocol_fee_amount + manager_fee_amount;
    let net_amount = amount - total_fees;

    if net_amount <= 0 {
        panic_with_error!(env, ErrorCode::TransferAmountTooSmallAfterFees);
    }

    Ok(TransferWithFees {
        protocol_fee_amount,
        manager_fee_amount,
        total_fees,
        net_amount,
    })
}

fn convert_index_token_amount_to_quote_amount(
    env: &Env,
    index: &Index,
    index_token_amount: i128,
    index_price: i128,
    now: u64,
) -> NormalResult<(i128, i128)> {
    // Get quote asset price
    let oracle_price_data =
        get_oracle_price(env, &index.oracle_source, &index.oracle, (XLM, USD), now)?;

    // let oracle_validity = oracle_validity(
    //     env,
    //     String::from(env, "Hello, Soroban!"),
    //     pool.historical_oracle_data.last_oracle_price_twap,
    //     &oracle_price_data,
    //     oracle_guard_rails().validity, // import from Oracle module
    //     2,
    //     false
    // )?;

    // validate!(
    //     is_oracle_valid_for_action(oracle_validity, action)?,
    //     ErrorCode::InvalidOracle,
    //     "Invalid Oracle ({} vs ema={}) for index={} and action={}",
    //     oracle_price_data,
    //     risk_ema_price,
    //     market.name,
    //     action
    // )?;

    // Compute amount of quote asset needed
    let quote_token_amount = index_price
        .safe_mul(index_token_amount, env)?
        .safe_div(oracle_price_data.price.cast::<i128>(env)?, env)?;

    Ok((
        quote_token_amount,
        oracle_price_data.price.cast::<i128>(env)?,
    ))
}

fn get_index_price(env: &Env, index: &Index) -> NormalResult<i128> {
    let current_nav = calculate_current_nav(env, index.component_balances.clone())?;

    let price = current_nav
        .safe_div(index.base_nav, env)?
        .safe_mul(index.initial_price, env)?;

    Ok(price)
}

fn calculate_current_nav(_env: &Env, component_balances: Map<Address, i128>) -> NormalResult<i128> {
    let mut nav = 0;

    component_balances
        .iter()
        .for_each(|(_token_address, token_balance)| {
            // TODO: Fetch the asset price from the synth token AMM
            let price = 0;

            // Add total value to NAV
            nav += token_balance * price;
        });

    Ok(nav)
}

fn swap_and_update_component_balances(_env: &Env, _operations: Vec<Swap>, _index: &Index) {
    // let index_factory_client = index_factory_contract::Client::new(env, utils::get_factory(env));

    // operations.iter().for_each(|op| {
    //     let amm_addr: Address = index_factory_client.query_for_amm_by_market(&op.clone().asset);

    //     let amm_client = amm_contract::Client::new(env, &amm_addr);

    //     swap_response = amm_client.swap(
    //         &recipient,
    //         &op.offer_asset,
    //         &next_offer_amount,
    //         &op.ask_asset_min_amount,
    //         &max_spread_bps,
    //         &max_allowed_fee_bps
    //     );

    //     let signed_amount = util(swap_response);

    //     index.component_balances[op.asset] += signed_amount;
    // });
}
