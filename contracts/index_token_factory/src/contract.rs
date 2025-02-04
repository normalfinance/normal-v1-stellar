use crate::{
    factory::IndexTokenFactoryTrait,
    storage::{
        get_config, get_index_vec, is_initialized, save_config, save_index_vec,
        save_index_vec_with_tuple_as_key, set_initialized, Config, IndexInfo, IndexTupleKey,
        Operation, ADMIN,
    },
    utils::deploy_index_token_contract,
};
use normal::{
    constants::{
        INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT,
        PERSISTENT_LIFETIME_THRESHOLD,
    },
    error::ErrorCode,
    types::IndexParams,
    validate_bps,
};
use soroban_sdk::{
    contract, contractimpl, contractmeta, log, panic_with_error, Address, BytesN, Env, IntoVal,
    String, Symbol, Val, Vec,
};

contractmeta!(
    key = "Description",
    val = "Factory for creating new Indexes"
);

#[contract]
pub struct IndexTokenFactory;

#[contractimpl]
impl IndexTokenFactoryTrait for IndexTokenFactory {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        index_token_wasm_hash: BytesN<32>,
        quote_token_whitelist: Vec<Address>,
        paused_operations: Vec<Operation>,
        max_manager_fee_bps: i64,
        protocol_fee_bps: i64,
        default_oracle: Address,
    ) {
        if is_initialized(&env) {
            log!(
                &env,
                "Index Token Factory: Initialize: initializing contract twice is not allowed"
            );
            panic_with_error!(&env, ErrorCode::AlreadyInitialized);
        }

        set_initialized(&env);

        save_config(
            &env,
            Config {
                admin: admin.clone(),
                index_token_wasm_hash,
                quote_token_whitelist,
                paused_operations,
                max_manager_fee_bps,
                protocol_fee_bps,
                default_oracle,
            },
        );

        save_index_vec(&env, Vec::new(&env));

        env.events()
            .publish(("initialize", "Index token factory contract"), admin);
    }

    fn create_index_token(
        env: Env,
        sender: Address,
        initial_deposit: i128,
        params: IndexParams,
    ) -> Address {
        sender.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let config = get_config(&env);

        if config.paused_operations.contains(Operation::CreateIndex) {
            panic_with_error!(&env, ErrorCode::IndexFactoryOperationPaused);
        }

        if params.manager_fee_bps >= config.max_manager_fee_bps {
            log!(
                env,
                "Factory: validate_token_info: Minimum amount of lp share tokens to bond can not be smaller or equal to 0"
            );
            panic_with_error!(&env, ErrorCode::AdminNotSet);
        }

        let index_token_hash = config.index_token_wasm_hash;

        let index_token_contract_address =
            deploy_index_token_contract(&env, index_token_hash, &params.name, &params.symbol);

        validate_bps!(params.manager_fee_bps);

        let factory_addr = env.current_contract_address();
        let init_fn: Symbol = Symbol::new(&env, "initialize");
        let init_fn_args: Vec<Val> = (
            sender.clone(),
            params.clone(),
            initial_deposit,
            factory_addr,
        )
            .into_val(&env);

        env.invoke_contract::<Val>(&index_token_contract_address, &init_fn, init_fn_args);

        let mut index_vec = get_index_vec(&env);

        index_vec.push_back(index_token_contract_address.clone());

        save_index_vec(&env, index_vec);
        let symbol = &params.symbol;
        let name = &params.name;
        save_index_vec_with_tuple_as_key(&env, (symbol, name), &index_token_contract_address);

        env.events()
            .publish(("create", "index"), &index_token_contract_address);

        index_token_contract_address
    }

    fn update_wasm_hashes(env: Env, index_token_wasm_hash: BytesN<32>) {
        let config = get_config(&env);

        config.admin.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        save_config(
            &env,
            Config {
                index_token_wasm_hash,
                ..config
            },
        );
    }

    fn update_config(
        env: Env,
        paused_operations: Option<Vec<Operation>>,
        max_manager_fee_bps: Option<i64>,
        protocol_fee_bps: Option<i64>,
        default_oracle: Option<Address>,
    ) {
        let mut config = get_config(&env);

        config.admin.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        if let Some(paused_operations) = paused_operations {
            config.paused_operations = paused_operations;
        }

        if let Some(max_manager_fee_bps) = max_manager_fee_bps {
            validate_bps!(max_manager_fee_bps);
            config.max_manager_fee_bps = max_manager_fee_bps;
        }
        if let Some(protocol_fee_bps) = protocol_fee_bps {
            validate_bps!(protocol_fee_bps);
            config.protocol_fee_bps = protocol_fee_bps;
        }

        if let Some(default_oracle) = default_oracle {
            config.default_oracle = default_oracle;
        }

        save_config(&env, config);
    }

    // ################################################################
    //                             Queries
    // ################################################################

    fn query_indexes(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_index_vec(&env)
    }

    fn query_index_details(env: Env, index_address: Address) -> IndexInfo {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let index_response: IndexInfo = env.invoke_contract(
            &index_address,
            &Symbol::new(&env, "query_index_info_for_factory"),
            Vec::new(&env),
        );
        index_response
    }

    fn query_all_indexes_details(env: Env) -> Vec<IndexInfo> {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let all_index_vec_addresses = get_index_vec(&env);
        let mut result = Vec::new(&env);
        for address in all_index_vec_addresses {
            let index_response: IndexInfo = env.invoke_contract(
                &address,
                &Symbol::new(&env, "query_index_info_for_factory"),
                Vec::new(&env),
            );

            result.push_back(index_response);
        }

        result
    }

    fn query_for_index_by_tuple(env: Env, name: String, symbol: String) -> Address {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let index_result: Option<Address> = env.storage().persistent().get(
            &(IndexTupleKey {
                symbol: symbol.clone(),
                name: name.clone(),
            }),
        );

        env.storage()
            .persistent()
            .has(
                &(IndexTupleKey {
                    symbol: symbol.clone(),
                    name: name.clone(),
                }),
            )
            .then(|| {
                env.storage().persistent().extend_ttl(
                    &(IndexTupleKey {
                        symbol: symbol.clone(),
                        name: name.clone(),
                    }),
                    PERSISTENT_LIFETIME_THRESHOLD,
                    PERSISTENT_BUMP_AMOUNT,
                );
            });

        if let Some(addr) = index_result {
            return addr;
        }

        let reverted_index_result: Option<Address> = env.storage().persistent().get(
            &(IndexTupleKey {
                symbol: name.clone(),
                name: symbol.clone(),
            }),
        );

        env.storage()
            .persistent()
            .has(
                &(IndexTupleKey {
                    symbol: name.clone(),
                    name: symbol.clone(),
                }),
            )
            .then(|| {
                env.storage().persistent().extend_ttl(
                    &(IndexTupleKey {
                        symbol: name,
                        name: symbol,
                    }),
                    PERSISTENT_LIFETIME_THRESHOLD,
                    PERSISTENT_BUMP_AMOUNT,
                );
            });

        if let Some(addr) = reverted_index_result {
            return addr;
        }

        log!(
            &env,
            "Factory: query_for_index_by_name_and_symbol failed: No index token found"
        );
        panic_with_error!(&env, ErrorCode::IndexTokenNotFound);
    }

    // fn query_for_amm_by_market(env: Env, market: Address) -> Address {
    //     // TODO: return the AMM address for a market via the Synth Market Factory
    //     // ...
    // }

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

    // // ...

    fn migrate_admin_key(env: Env) -> Result<(), ErrorCode> {
        let admin = get_config(&env).admin;
        env.storage().instance().set(&ADMIN, &admin);

        Ok(())
    }
}

#[contractimpl]
impl IndexTokenFactory {
    #[allow(dead_code)]
    pub fn update(env: Env, new_wasm_hash: BytesN<32>) {
        let admin = get_config(&env).admin;
        admin.require_auth();

        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
}
