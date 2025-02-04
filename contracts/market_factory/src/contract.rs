use crate::{
    errors::ContractError,
    factory::MarketFactoryTrait,
    storage::{
        get_config, get_market_vec, is_initialized, save_config, save_market_vec,
        save_market_vec_with_tuple_as_key, set_initialized, Config, MarketInfo, MarketTupleKey,
    },
    utils::deploy_market_contract,
};
use normal::{
    constants::{
        INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT,
        PERSISTENT_LIFETIME_THRESHOLD,
    },
    error::ErrorCode,
    oracle::OracleGuardRails,
};
use soroban_sdk::{
    contract, contractimpl, contractmeta, log, panic_with_error, Address, BytesN, Env, String,
    Symbol, Val, Vec,
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
        governor: Address,
        market_wasm_hash: BytesN<32>,
        token_wasm_hash: BytesN<32>,
    ) {
        if is_initialized(&env) {
            log!(
                &env,
                "Factory: Initialize: initializing contract twice is not allowed"
            );
            panic_with_error!(&env, ContractError::AlreadyInitialized);
        }

        set_initialized(&env);

        save_config(
            &env,
            Config {
                admin: admin.clone(),
                governor: governor.clone(),
                market_wasm_hash,
                token_wasm_hash,
                emergency_oracles: Vec::new(&env),
                oracle_guard_rails: OracleGuardRails::default(),
            },
        );

        save_market_vec(&env, Vec::new(&env));

        env.events()
            .publish(("initialize", "Market factory contract"), admin);
    }

    #[allow(clippy::too_many_arguments)]
    fn create_market(
        env: Env,
        sender: Address,
        asset: Symbol,
        params: MarketParams,
        token_wasm_hash: BytesN<32>,
        synth_token_name: String,
        synth_token_symbol: String,
    ) -> Address {
        sender.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let config = get_config(&env);
        let index_token_hash = config.market_wasm_hash;
        let token_wasm_hash = config.token_wasm_hash;

        let market_contract_address = deploy_market_contract(
            &env,
            token_wasm_hash,
            &params,
            index_token_hash,
            &synth_token_name,
            &synth_token_symbol,
        );

        let factory_addr = env.current_contract_address();
        let init_fn: Symbol = Symbol::new(&env, "initialize");
        let init_fn_args: Vec<Val> = (
            sender.clone(),
            params.clone(),
            initial_deposit,
            factory_addr,
        )
            .into_val(&env);

        env.invoke_contract::<Val>(&market_contract_address, &init_fn, init_fn_args);

        let mut market_vec = get_market_vec(&env);

        market_vec.push_back(market_contract_address.clone());

        save_market_vec(&env, market_vec);
        let token_a = &asset;
        let token_b = &params.collateral_token;
        save_market_vec_with_tuple_as_key(&env, (token_a, token_b), &market_contract_address);

        env.events()
            .publish(("create", "market"), &market_contract_address);

        market_contract_address
    }

    fn update_emergency_oracles(
        env: Env,
        sender: Address,
        to_add: Vec<Address>,
        to_remove: Vec<Address>,
    ) {
        sender.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let config = get_config(&env);

        if config.admin != sender {
            log!(
                &env,
                "Factory: Update whitelisted accounts: You are not authorized!"
            );
            panic_with_error!(&env, ContractError::NotAuthorized);
        }

        let mut emergency_oracles = config.emergency_oracles;

        to_add.into_iter().for_each(|addr| {
            if !emergency_oracles.contains(addr.clone()) {
                emergency_oracles.push_back(addr);
            }
        });

        to_remove.into_iter().for_each(|addr| {
            if let Some(id) = emergency_oracles.iter().position(|x| x == addr) {
                emergency_oracles.remove(id as u32);
            }
        });

        save_config(
            &env,
            Config {
                emergency_oracles,
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
            Config {
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
            Config {
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

    fn query_for_market_by_token_pair(env: Env, token_a: Symbol, token_b: Symbol) -> Address {
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
        panic_with_error!(&env, ContractError::MarketNotFound);
    }

    fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_config(&env).admin
    }

    fn get_config(env: Env) -> Config {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_config(&env)
    }

    fn query_emergency_oracle(env: Env, oracle: Address) -> bool {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_config(&env).emergency_oracles.contains(oracle)
    }

    fn migrate_admin_key(env: Env) -> Result<(), ErrorCode> {
        let admin = get_config(&env).admin;
        env.storage().instance().set(&ADMIN, &admin);

        Ok(())
    }
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
