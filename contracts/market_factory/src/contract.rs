use crate::{
    errors::Errors,
    factory::MarketFactoryTrait,
    storage::{
        get_config, get_market_vec, is_initialized, save_config, save_market_vec,
        save_market_vec_with_tuple_as_key, set_initialized, MarketTupleKey,
    },
    utils::{deploy_lp_token_contract, deploy_market_contract, deploy_synthetic_token_contract},
};
use normal::{
    constants::{
        INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT,
        PERSISTENT_LIFETIME_THRESHOLD,
    },
    oracle::OracleGuardRails,
    types::market::{MarketFactoryConfig, MarketInfo, MarketParams},
};
use soroban_sdk::{
    contract, contractimpl, contractmeta, log, panic_with_error, Address, BytesN, Env, FromVal,
    IntoVal, String, Symbol, Val, Vec,
};

contractmeta!(
    key = "Description",
    val = "Factory for creating new Synth Markets"
);

#[contract]
pub struct MarketFactory;

#[contractimpl]
impl MarketFactoryTrait for MarketFactory {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        insurance: Address,
        market_wasm_hash: BytesN<32>,
        token_wasm_hash: BytesN<32>,
    ) {
        if is_initialized(&env) {
            log!(
                &env,
                "Factory: Initialize: initializing contract twice is not allowed"
            );
            panic_with_error!(&env, Errors::AlreadyInitialized);
        }

        set_initialized(&env);

        save_config(
            &env,
            MarketFactoryConfig {
                admin: admin.clone(),
                insurance: insurance.clone(),
                market_wasm_hash,
                token_wasm_hash,
                super_keepers: Vec::new(&env),
                oracle_guard_rails: OracleGuardRails::default(),
            },
        );

        save_market_vec(&env, Vec::new(&env));

        env.events()
            .publish(("initialize", "Market factory contract"), admin);
    }

    #[allow(clippy::too_many_arguments)]
    fn create_market(env: Env, params: MarketParams) -> Address {
        let config = get_config(&env);

        config.admin.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let market_wasm_hash = config.market_wasm_hash;
        let token_wasm_hash = config.token_wasm_hash;

        let token_decimals = params.token_decimals;
        let synth_token_name = params.synth_token_name;
        let synth_token_symbol = params.synth_token_symbol;
        let lp_token_symbol = params.lp_token_symbol;
        let quote_token = &params.quote_token;
        let oracle = &params.oracle;

        // deploy and initialize the synth token contract
        let synth_token_address = deploy_synthetic_token_contract(
            &env,
            token_wasm_hash.clone(),
            quote_token,
            env.current_contract_address(),
            token_decimals,
            synth_token_name.clone(),
            synth_token_symbol.clone(),
        );

        // deploy and initialize the liquidity pool token contract
        let lp_token_address = deploy_lp_token_contract(
            &env,
            token_wasm_hash.clone(),
            &synth_token_address,
            quote_token,
            env.current_contract_address(),
            token_decimals,
            lp_token_symbol.clone(),
            lp_token_symbol,
        );

        let market_contract_address = deploy_market_contract(
            &env,
            market_wasm_hash,
            oracle,
            &synth_token_name,
            &synth_token_symbol,
        );

        // let args = params;
        let factory_addr = env.current_contract_address();

        let init_fn: Symbol = Symbol::new(&env, "initialize");
        let init_fn_args: Vec<Val> = (
            synth_token_address.clone(),
            lp_token_address,
            // params,
            factory_addr,
            config.insurance,
        )
            .into_val(&env);

        env.invoke_contract::<Val>(&market_contract_address, &init_fn, init_fn_args);

        let mut market_vec = get_market_vec(&env);

        market_vec.push_back(market_contract_address.clone());

        save_market_vec(&env, market_vec);
        // let token_b = quote_token;
        save_market_vec_with_tuple_as_key(
            &env,
            (&synth_token_address, quote_token),
            &market_contract_address,
        );

        env.events()
            .publish(("create", "market"), &market_contract_address);

        market_contract_address
    }

    fn update_super_keepers(env: Env, to_add: Vec<Address>, to_remove: Vec<Address>) {
        let config = get_config(&env);

        config.admin.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let mut super_keepers = config.super_keepers;

        to_add.into_iter().for_each(|addr| {
            if !super_keepers.contains(addr.clone()) {
                super_keepers.push_back(addr);
            }
        });

        to_remove.into_iter().for_each(|addr| {
            if let Some(id) = super_keepers.iter().position(|x| x == addr) {
                super_keepers.remove(id as u32);
            }
        });

        save_config(
            &env,
            MarketFactoryConfig {
                super_keepers,
                ..config
            },
        )
    }

    fn update_wasm_hashes(
        env: Env,
        market_wasm_hash: Option<BytesN<32>>,
        token_wasm_hash: Option<BytesN<32>>,
    ) {
        let config = get_config(&env);

        config.admin.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        save_config(
            &env,
            MarketFactoryConfig {
                market_wasm_hash: market_wasm_hash.unwrap_or(config.market_wasm_hash),
                token_wasm_hash: token_wasm_hash.unwrap_or(config.token_wasm_hash),
                ..config
            },
        );
    }

    fn update_oracle_guard_rails(env: Env, oracle_guard_rails: OracleGuardRails) {
        let config = get_config(&env);

        config.admin.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // TODO: validate guard rails

        save_config(
            &env,
            MarketFactoryConfig {
                oracle_guard_rails,
                ..config
            },
        );
    }

    // ################################################################
    //                             Queries
    // ################################################################

    fn query_markets(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_market_vec(&env)
    }

    fn query_market_details(env: Env, market_address: Address) -> MarketInfo {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let market_response: MarketInfo = env.invoke_contract(
            &market_address,
            &Symbol::new(&env, "query_market_info_for_factory"),
            Vec::new(&env),
        );
        market_response
    }

    fn query_all_markets_details(env: Env) -> Vec<MarketInfo> {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let all_market_vec_addresses = get_market_vec(&env);
        let mut result = Vec::new(&env);
        for address in all_market_vec_addresses {
            let market_response: MarketInfo = env.invoke_contract(
                &address,
                &Symbol::new(&env, "query_market_info_for_factory"),
                Vec::new(&env),
            );

            result.push_back(market_response);
        }

        result
    }

    fn query_for_market_by_token_pair(env: Env, token_a: Address, token_b: Address) -> Address {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let market_result: Option<Address> = env.storage().persistent().get(
            &(MarketTupleKey {
                token_a: token_a.clone(),
                token_b: token_b.clone(),
            }),
        );

        env.storage()
            .persistent()
            .has(
                &(MarketTupleKey {
                    token_a: token_a.clone(),
                    token_b: token_b.clone(),
                }),
            )
            .then(|| {
                env.storage().persistent().extend_ttl(
                    &(MarketTupleKey {
                        token_a: token_a.clone(),
                        token_b: token_b.clone(),
                    }),
                    PERSISTENT_LIFETIME_THRESHOLD,
                    PERSISTENT_BUMP_AMOUNT,
                );
            });

        if let Some(addr) = market_result {
            return addr;
        }

        let reverted_market_result: Option<Address> = env.storage().persistent().get(
            &(MarketTupleKey {
                token_a: token_b.clone(),
                token_b: token_a.clone(),
            }),
        );

        env.storage()
            .persistent()
            .has(
                &(MarketTupleKey {
                    token_a: token_b.clone(),
                    token_b: token_a.clone(),
                }),
            )
            .then(|| {
                env.storage().persistent().extend_ttl(
                    &(MarketTupleKey {
                        token_a: token_b,
                        token_b: token_a,
                    }),
                    PERSISTENT_LIFETIME_THRESHOLD,
                    PERSISTENT_BUMP_AMOUNT,
                );
            });

        if let Some(addr) = reverted_market_result {
            return addr;
        }

        log!(
            &env,
            "Factory: query_for_market_by_token_pair failed: No market found"
        );
        panic_with_error!(&env, Errors::MarketNotFound);
    }

    fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_config(&env).admin
    }

    fn get_config(env: Env) -> MarketFactoryConfig {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_config(&env)
    }

    // fn migrate_admin_key(env: Env) -> Result<(), ErrorCode> {
    //     let admin = get_config(&env).admin;
    //     env.storage().instance().set(&ADMIN, &admin);

    //     Ok(())
    // }
}

#[contractimpl]
impl MarketFactory {
    #[allow(dead_code)]
    pub fn update(env: Env, new_wasm_hash: BytesN<32>) {
        let admin = get_config(&env).admin;
        admin.require_auth();

        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
}
