use crate::{
    storage::{
        get_config,
        get_index_vec,
        is_initialized,
        save_config,
        save_index_vec,
        set_initialized,
        Asset,
        Config,
        IndexInfo,
        Operation,
        ADMIN,
    },
    utils::deploy_index_contract,
};
use normal::{
    ttl::{ INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD },
    validate_bps,
    error::{ ErrorCode },
};
use soroban_sdk::{
    contract,
    contractimpl,
    contractmeta,
    log,
    panic_with_error,
    vec,
    Address,
    BytesN,
    Env,
    String,
    Symbol,
    Val,
    Vec,
};

contractmeta!(key = "Description", val = "Factory for creating new Indexes");

#[contract]
pub struct IndexFactory;

#[allow(dead_code)]
pub trait IndexFactoryTrait {
    fn initialize(
        env: Env,
        admin: Address,
        index_wasm_hash: BytesN<32>,
        index_token_wasm_hash: BytesN<32>,
        paused_operations: Vec<Operation>,
        max_manager_fee_bps: i64,
        protocol_fee_bps: i64,
        default_oracle: Address
    );

    #[allow(clippy::too_many_arguments)]
    fn create_index(
        env: Env,
        sender: Address,
        index_params: IndexParams,
        index_token_name: String,
        index_token_symbol: String
    ) -> Address;

    fn update_wasm_hashes(
        env: Env,
        index_wasm_hash: Option<BytesN<32>>,
        index_token_wasm_hash: Option<BytesN<32>>
    );

    // Allows admin address set during initialization to change some parameters of the
    // configuration
    #[allow(clippy::too_many_arguments)]
    fn update_config(
        env: Env,
        new_admin: Option<Address>,
        paused_operations: Option<Vec<Operation>>,
        max_manager_fee_bps: Option<i64>,
        protocol_fee_bps: Option<i64>,
        default_oracle: Option<Address>
    );

    fn query_indexes(env: Env) -> Vec<Address>;

    fn query_index_details(env: Env, index_address: Address) -> IndexInfo;

    fn query_all_indexes_details(env: Env) -> Vec<IndexInfo>;

    // For indexes to query AMMs via the Synth Market Factory
    fn query_for_amm_by_market(env: Env, marekt: Address) -> Address;

    // fn query_for_pool_by_token_pair(env: Env, token_a: Address, token_b: Address) -> Address;

    fn get_admin(env: Env) -> Address;

    fn get_config(env: Env) -> Config;

    // fn query_user_portfolio(env: Env, sender: Address, staking: bool) -> UserPortfolio;

    fn migrate_admin_key(env: Env) -> Result<(), ErrorCode>;
}

#[contractimpl]
impl IndexFactoryTrait for IndexFactory {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        index_wasm_hash: BytesN<32>,
        index_token_wasm_hash: BytesN<32>,
        paused_operations: Vec<Operation>,
        max_manager_fee_bps: i64,
        protocol_fee_bps: i64,
        default_oracle: Address
    ) {
        if is_initialized(&env) {
            log!(&env, "Index Factory: Initialize: initializing contract twice is not allowed");
            panic_with_error!(&env, ErrorCode::AlreadyInitialized);
        }

        set_initialized(&env);

        save_config(&env, Config {
            admin: admin.clone(),
            index_wasm_hash,
            index_token_wasm_hash,
            paused_operations,
            max_manager_fee_bps,
            protocol_fee_bps,
            default_oracle,
        });

        save_index_vec(&env, Vec::new(&env));

        env.events().publish(("initialize", "Index factory contract"), admin);
    }

    #[allow(clippy::too_many_arguments)]
    fn create_index(
        env: Env,
        sender: Address,
        index_params: IndexParams,
        index_token_name: String,
        index_token_symbol: String
    ) -> Address {
        sender.require_auth();
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // validate_token_info(&env, &lp_init_info.token_init_info, &lp_init_info.stake_init_info);

        let config = get_config(&env);
        let token_wasm_hash = config.token_wasm_hash;
        let index_hash = config.index_wasm_hash;

        let index_contract_address = deploy_index_contract(
            &env,
            index_hash,
            &index_params.token_init_info.token_a,
            &index_params.token_init_info.token_b
        );

        validate_bps!(
            index_params.manager_fee_bps,
            index_params.revenue_share_bps,
            max_manager_fee_bps,
            protocol_fee_bps
        );

        let factory_addr = env.current_contract_address();
        let init_fn: Symbol = Symbol::new(&env, "initialize");
        let mut init_fn_args: Vec<Val> = (
            index_token_wasm_hash,
            index_params.clone(),
            factory_addr,
            index_token_name,
            index_token_symbol,
        ).into_val(&env);

        // if let PoolType::Xyk = pool_type {
        //     init_fn_args.push_back(default_slippage_bps.into_val(&env));
        // }

        // init_fn_args.push_back(max_allowed_fee_bps.into_val(&env));

        env.invoke_contract::<Val>(&index_contract_address, &init_fn, init_fn_args);

        let mut index_vec = get_index_vec(&env);

        index_vec.push_back(index_contract_address.clone());

        save_index_vec(&env, index_vec);
        let token_a = &lp_init_info.token_init_info.token_a;
        let token_b = &lp_init_info.token_init_info.token_b;
        // save_index_vec_with_tuple_as_key(&env, (token_a, token_b), &index_contract_address);

        env.events().publish(("create", "index"), &index_contract_address);

        index_contract_address
    }

    fn update_wasm_hashes(
        env: Env,
        index_wasm_hash: Option<BytesN<32>>,
        index_token_wasm_hash: Option<BytesN<32>>
    ) {
        let config = get_config(&env);

        config.admin.require_auth();
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        save_config(&env, Config {
            index_wasm_hash: index_wasm_hash.unwrap_or(config.index_wasm_hash),
            index_token_wasm_hash: index_token_wasm_hash.unwrap_or(config.index_token_wasm_hash),
            ..config
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn update_config(
        env: Env,
        new_admin: Option<Address>,
        paused_operations: Option<Vec<Operation>>,
        max_manager_fee_bps: Option<i64>,
        protocol_fee_bps: Option<i64>,
        default_oracle: Option<Address>
    ) {
        let admin: Address = utils::get_admin_old(&env);
        admin.require_auth();
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let mut config = get_config(&env);

        if let Some(new_admin) = new_admin {
            utils::save_admin_old(&env, new_admin);
        }

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

    fn query_indexes(env: Env) -> Vec<Address> {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_index_vec(&env)
    }

    fn query_index_details(env: Env, index_address: Address) -> IndexInfo {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let index_response: IndexInfo = env.invoke_contract(
            &index_address,
            &Symbol::new(&env, "query_index_info_for_factory"),
            Vec::new(&env)
        );
        index_response
    }

    fn query_all_indexes_details(env: Env) -> Vec<IndexInfo> {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let all_index_vec_addresses = get_index_vec(&env);
        let mut result = Vec::new(&env);
        for address in all_index_vec_addresses {
            let pool_response: IndexInfo = env.invoke_contract(
                &address,
                &Symbol::new(&env, "query_index_info_for_factory"),
                Vec::new(&env)
            );

            result.push_back(pool_response);
        }

        result
    }

    // ....

    fn query_for_amm_by_market(env: Env, market: Address) -> Address {
        // TODO: return the AMM address for a market via the Synth Market Factory
        // ...
    }

    fn get_admin(env: Env) -> Address {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_config(&env).admin
    }

    fn get_config(env: Env) -> Config {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_config(&env)
    }

    // ...

    fn migrate_admin_key(env: Env) -> Result<(), ErrorCode> {
        let admin = get_config(&env).admin;
        env.storage().instance().set(&ADMIN, &admin);

        Ok(())
    }
}

#[contractimpl]
impl IndexFactory {
    #[allow(dead_code)]
    pub fn update(env: Env, new_wasm_hash: BytesN<32>) {
        let admin = get_config(&env).admin;
        admin.require_auth();

        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
}
