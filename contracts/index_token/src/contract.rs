use crate::allowance::{read_allowance, spend_allowance, write_allowance};
use crate::balance::{read_balance, receive_balance, spend_balance};
use crate::metadata::{read_decimal, read_name, read_symbol, write_metadata};
use crate::storage::{
    get_last_transfer, is_initialized, read_administrator, read_factory, set_initialized,
    write_administrator, write_factory, Swap, TransferWithFees,
};

use normal::error::ErrorCode;
use normal::types::{IndexAsset, IndexTokenInitInfo};
use soroban_sdk::token::{self, Interface as _};
use soroban_sdk::{
    assert_with_error, contract, contractimpl, contractmeta, log, panic_with_error, symbol_short,
    Address, Env, Map, String, Symbol, Vec,
};
use soroban_token_sdk::metadata::TokenMetadata;
use soroban_token_sdk::TokenUtils;

use crate::{
    amm_contract,
    events::IndexTokenEvents,
    index_factory_contract, index_factory_contract,
    index_token::IndexTokenTrait,
    storage::{get_index, save_index, DataKey, Index, IndexOperation},
    token_contract,
};
use normal::math::oracle::{is_oracle_valid_for_action, oracle_validity, NormalAction};
use normal::oracle::get_oracle_price;

use normal::{
    constants::{INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD, SECONDS_IN_A_YEAR},
    validate, validate_bps,
};

fn check_nonnegative_amount(amount: i128) {
    if amount < 0 {
        panic!("negative amount is not allowed: {}", amount)
    }
}

fn is_governor(env: &Env, sender: Address) {
    let factory_client = index_factory_contract::Client::new(&env, &read_factory(&env));
    let config = factory_client.query_config();

    if config.governor != sender {
        log!(&env, "Index Token: You are not authorized!");
        panic_with_error!(&env, ErrorCode::NotAuthorized);
    }
}

fn is_admin(env: &Env, sender: Address) {
    let admin = read_administrator(&env);
    if admin != sender {
        log!(&env, "Index Token: You are not authorized!");
        panic_with_error!(&env, ErrorCode::NotAuthorized);
    }
}

contractmeta!(
    key = "Description",
    val = "Diversified exposure to a basket of cryptocurrencies"
);

#[contract]
pub struct IndexToken;

#[contractimpl]
impl IndexTokenTrait for IndexToken {
    // ################################################################
    //                             ADMIN
    // ################################################################

    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        factory: Address,
        quote_token: Address,
        rebalance_threshold: i64,
        params: IndexTokenInitInfo,
    ) {
        if params.decimal > 18 {
            panic!("Decimal must not be greater than 18");
        }

        if is_initialized(&env) {
            log!(
                &env,
                "Index Token: Initialize: initializing contract twice is not allowed"
            );
            panic_with_error!(&env, ErrorCode::AlreadyInitialized);
        }

        validate_bps!(params.manager_fee_bps);

        set_initialized(&env);

        write_administrator(&env, &admin);
        write_factory(&env, &factory);
        write_metadata(
            &env,
            TokenMetadata {
                decimal: params.decimal,
                name: params.name,
                symbol: params.symbol,
            },
        );

        let now = env.ledger().timestamp();
        let index = Index {
            quote_token,
            quote_oracle: params.oracle,
            quote_oracle_source: params.oracle_source,
            is_public: params.is_public,
            paused_operations: Vec::new(&env),
            manager_fee_bps: params.manager_fee_bps,
            whitelist: Vec::new(&env),
            blacklist: Vec::new(&env),
            base_nav: 0,
            initial_price,
            component_balances: Vec::new(&env),
            component_balance_update_ts: now,
            component_assets: params.component_assets,
            rebalance_threshold,
            rebalance_ts: now,
            last_updated_ts: now,
            total_fees: 0,
            total_mints: 0,
            total_redemptions: 0,
        };

        save_index(&env, index);

        IndexTokenEvents::initialize(&env, admin, name, symbol);

        // Mint initial tokens
        let initial_mint_amount = base_nav / initial_price;
        Self::mint(env, sender, index_token_amount, to);
    }

    fn update_manager_fee(env: Env, sender: Address, manager_fee_bps: i64) {
        sender.require_auth();
        is_admin(&env, sender);

        validate_bps!(manager_fee_bps);

        let mut index = get_index(&env);

        save_index(
            &env,
            Index {
                manager_fee_bps,
                ..index
            },
        );
    }

    fn update_paused_operations(
        env: Env,
        sender: Address,
        to_add: Vec<IndexOperation>,
        to_remove: Vec<IndexOperation>,
    ) {
        sender.require_auth();
        is_admin(&env, sender);

        let mut index = get_index(&env);
        let mut paused_operations = index.paused_operations;

        to_add.into_iter().for_each(|op| {
            if !paused_operations.contains(op.clone()) {
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
        is_admin(&env, sender);

        let mut index: Index = get_index(&env);
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
        is_admin(&env, sender);

        let mut index: Index = get_index(&env);
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

    fn update_rebalance_threshold(env: Env, sender: Address, rebalance_threshold: i64) {
        sender.require_auth();

        let mut index = get_index(&env);

        if index.is_public {
            is_governor(&env, sender);
        } else {
            is_admin(&env, sender);
            // sender.require_auth();
        }

        save_index(
            &env,
            Index {
                rebalance_threshold,
                ..index
            },
        );
    }

    // ################################################################
    //                             KEEPER
    // ################################################################

    fn rebalance(env: Env, sender: Address, updated_assets: Vec<IndexAsset>) {
        let mut index = get_index(&env);

        if index.is_public {
            // is_governor(&env, sender);
            // TODO: weight change guardrails to avoid massive spikes in price
        } else {
            is_admin(&env, sender);
            sender.require_auth();
        }

        let now = env.ledger().timestamp();
        if !index.can_rebalance(now) {
            return Err(ErrorCode::TooSoonToRebalance);
        }

        // TODO: validate updated asset markets

        let mut position_increases: Vec<Swap> = [];
        let mut position_reductions: Vec<Swap> = [];

        updated_assets.iter().for_each(|updated_asset| {
            let current_asset = index
                .component_assets
                .iter()
                .find(|current_asset| current_asset.market_address == updated_asset.market_address)
                .cloned();

            match current_asset {
                Some(current_asset) => {
                    let percent_delta = updated_asset.weight - current_asset.weight;
                    let amount_delta = 0;

                    if delta > 0 {
                        position_increases.push(Swap {
                            ask_asset: updated_asset.market_address, // Buy the asset
                            offer_asset: index.quote_asset,
                            ask_asset_min_amount: amount_delta,
                        })
                    } else {
                        position_reductions.push(Swap {
                            ask_asset: index.quote_asset,
                            offer_asset: updated_asset.market_address, // Sell the asset
                            ask_asset_min_amount: amount_delta,
                        })
                    }
                }
                None => {
                    let amount_delta = 0;

                    position_increases.push(Swap {
                        ask_asset: updated_asset.market_address, // Buy the asset
                        offer_asset: index.quote_asset,
                        ask_asset_min_amount: amount_delta,
                    })
                }
            }
        });

        swap_and_update_component_balances(&env, position_increases, index);
        swap_and_update_component_balances(&env, position_reductions, index);

        save_index(
            &env,
            Index {
                component_balances: [], // TODO:
                component_assets: [],   // TODO:
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

    fn mint(env: Env, sender: Address, index_token_amount: i128, to: Option<Address>) {
        check_nonnegative_amount(index_token_amount);
        sender.require_auth();

        // Check if token is allowed
        // if !Self::is_token_allowed(&env, &token) {
        //     return Err(Error::TokenNotAllowed);
        // }

        // Get index and price
        let index = get_index(&env);
        if !index.can_invest(&env, sender) {
            return Err(ErrorCode::idk);
        }

        let index_price = get_index_price(&env, index);

        // Compute amount of quote asset needed
        let quote_token_amount =
            convert_index_token_amount_to_quote_amount(&env, index_token_amount, index_price);

        // Deposit initial investment
        let quote_token_client = token_contract::Client::new(&env, &index.quote_token);
        quote_token_client.transfer(
            &sender,
            &env.current_contract_address(),
            &quote_token_amount,
        );

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
            None => sender,                 // Otherwise use the sender address
        };

        receive_balance(&env, recipient_address.clone(), amount);
        TokenUtils::new(&env)
            .events()
            .mint(admin, recipient_address, amount);

        IndexTokenEvents::mint(&env, sender, index_token_amount, recipient_address);
    }

    fn redeem(env: Env, sender: Address, index_token_amount: i128, to: Option<Address>) {
        check_nonnegative_amount(index_token_amount);
        sender.require_auth();

        let now = env.ledger().timestamp();

        // Get index and price
        let index = get_index(&env);
        let index_price = get_index_price(&env, index);

        // Compute amount of quote asset needed
        let quote_token_amount =
            convert_index_token_amount_to_quote_amount(&env, index_token_amount, index_price, now);

        // Ensure sufficient quote funds

        match index.quote_token {
            Some(token) => {
                let token_client = token_contract::Client::new(&env, &token);
                let balance = token_client.balance(&sender);
                if balance < quote_token_amount {
                    return Err(ErrorCode::InsufficientFunds);
                }
            }
            None => {
                env.transfer(&env.current_contract_address(), &params.initial_deposit);
            }
        }

        Self::burn(env, from, amount);

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

        swap_and_update_component_balances(&env, operations, index);

        // Transfer quote token back to user
        let recipient_address = match to {
            Some(to_address) => to_address, // Use the provided `to` address
            None => sender,                 // Otherwise use the sender address
        };

        quote_token_client.transfer(&env.current_contract_address(), &recipient_address, &amount);

        IndexTokenEvents::redeem(&env, sender, index_token_amount);
    }

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_index(env: Env) -> Index {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        get_index(&env)
    }

    fn query_price(env: Env) -> i128 {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let index = get_index(&env);

        get_index_price(&env, index);
    }

    fn query_nav(env: Env) -> i128 {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let index = get_index(&env);

        calculate_current_nav(&env, index);
    }

    fn query_fee_exemption(env: Env, user: Address) -> bool {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let index = get_index(&env);

        let exempt = from == admin || from == protocol_address || from == index_contract_addr;

        exempt
    }

    fn query_fees_for_transfer(env: &Env, from: Address, amount: i128) -> TransferWithFees {
        if Self::query_fee_exemption(env, from) {
            // No fees applied, transfer the full amount
            return TransferWithFees {
                protocol_fee_amount: 0,
                manager_fee_amount: 0,
                total_fees: 0,
                net_amount: amount,
            };
        }

        let last_transfer = get_last_transfer(&env, &from);

        if last_transfer.balance == 0 {
            // No fee if there was no prior balance
            return TransferWithFees {
                protocol_fee_amount: 0,
                manager_fee_amount: 0,
                total_fees: 0,
                net_amount: amount,
            };
        }

        let index = get_index(&env);

        // Get the protocol fee
        let protocol_fee: i64 = env.invoke_contract(
            &read_factory(&env),
            &Symbol::new(&env, "query_protocol_fee"),
            vec![],
        );

        // Calculate weighted holding time
        let time_held = env.ledger().timestamp() - last_transfer.ts;

        // Prorated fee calculation
        let protocol_fee_amount =
            (amount * (protocol_fee as i128) * (time_held as i128)) / SECONDS_IN_A_YEAR;
        let manager_fee_amount =
            (amount * (index.manager_fee_bps as i128) * (time_held as i128)) / SECONDS_IN_A_YEAR;

        let total_fees = protocol_fee_amount + manager_fee_amount;

        let net_amount = amount - total_fees;

        if net_amount <= 0 {
            panic_with_error!(&env, ErrorCode::TransferAmountTooSmallAfterFees);
        }

        TransferWithFees {
            protocol_fee_amount,
            manager_fee_amount,
            total_fees,
            net_amount,
        }
    }

    // #[cfg(test)]
    // pub fn get_allowance(env: Env, from: Address, spender: Address) -> Option<AllowanceValue> {
    //     let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    //     let allowance = env.storage().temporary().get::<_, AllowanceValue>(&key);
    //     allowance
    // }
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
        check_nonnegative_amount(amount);
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

        check_nonnegative_amount(amount);

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        transfer_with_index_fee(&env, from, to, amount);
    }

    fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();

        check_nonnegative_amount(amount);

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        spend_allowance(&env, from.clone(), spender, amount);
        transfer_with_index_fee(&env, from, to, amount);
    }

    fn burn(env: Env, from: Address, amount: i128) {
        from.require_auth();

        check_nonnegative_amount(amount);

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        spend_balance(&env, from.clone(), amount);
        TokenUtils::new(&env).events().burn(from, amount);
    }

    fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();

        check_nonnegative_amount(amount);

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

fn transfer_with_index_fee(env: &Env, from: Address, to: Address, amount: i128) {
    let transfer_info = IndexToken::query_fees_for_transfer(&env, from, amount);

    let factory_addr = read_factory(&env);
    let manager_addr = read_administrator(env); // Assumes admin is manager

    spend_balance(&env, from.clone(), amount);
    receive_balance(&env, to.clone(), transfer_info.net_amount);
    TokenUtils::new(&env)
        .events()
        .transfer(from, to, transfer_info.net_amount);

    if transfer_info.protocol_fee_amount > 0 {
        receive_balance(
            &env,
            factory_addr.clone(),
            transfer_info.protocol_fee_amount,
        );
    }

    if transfer_info.manager_fee_amount > 0 {
        receive_balance(&env, manager_addr.clone(), transfer_info.manager_fee_amount);
    }
}

fn convert_index_token_amount_to_quote_amount(
    env: &Env,
    index_token_amount: i128,
    index_price: i128,
    now: u64,
    action: Option<NormalAction>,
) -> (i128, i128) {
    // Get quote asset price
    let oracle_price_data = get_oracle_price(
        &env,
        index.oracle_source,
        index.oracle,
        index.quote_asset,
        "USD",
        now,
    );

    let oracle_validity = oracle_validity(
        risk_ema_price,
        &oracle_price_data,
        oracle_guard_rails().validity, // import from Oracle module
        2,
        false,
    )?;

    validate!(
        is_oracle_valid_for_action(oracle_validity, action)?,
        ErrorCode::InvalidOracle,
        "Invalid Oracle ({} vs ema={}) for index={} and action={}",
        oracle_price_data,
        risk_ema_price,
        market.name,
        action
    )?;

    // Compute amount of quote asset needed
    let quote_token_amount = (index_price * index_token_amount) / oracle_price_data.price;

    (quote_token_amount, oracle_price_data.price)
}

fn get_index_price(env: &Env, index: Index) -> i128 {
    let current_nav = calculate_current_nav(&env, index.component_balances);

    let price = (current_nav / index.base_nav) * index.initial_price;

    price
}

fn calculate_current_nav(env: Env, component_balances: Map<Address, u128>) -> u128 {
    let mut nav = 0;

    component_balances
        .iter()
        .for_each(|(token_address, token_balance)| {
            // TODO: Fetch the asset price from the synth token AMM
            let price = 0;

            // Add total value to NAV
            nav += token_balance * price;
        });

    nav
}

fn swap_and_update_component_balances(env: Env, operations: Vec<Swap>, index: Index) {
    let index_factory_client = index_factory_contract::Client::new(&env, &read_factory(&env));

    operations.iter().for_each(|op| {
        let amm_addr: Address = index_factory_client.query_for_amm_by_market(&op.clone().asset);

        let amm_client = amm_contract::Client::new(&env, &amm_addr);

        swap_response = amm_client.swap(
            &recipient,
            &op.offer_asset,
            &next_offer_amount,
            &op.ask_asset_min_amount,
            &max_spread_bps,
            &max_allowed_fee_bps,
        );

        let signed_amount = util(swap_response);

        index.component_balances[op.asset] += signed_amount;
    });
}
