use crate::{
    error::ContractError,
    stake_contract::StakedResponse,
    storage::{
        get_config,
        get_lp_vec,
        get_stable_wasm_hash,
        is_initialized,
        save_config,
        save_lp_vec,
        save_lp_vec_with_tuple_as_key,
        save_stable_wasm_hash,
        set_initialized,
        Asset,
        Config,
        LiquidityPoolInfo,
        LpPortfolio,
        PairTupleKey,
        StakePortfolio,
        UserPortfolio,
        ADMIN,
    },
    utils::{ deploy_and_initialize_multihop_contract, deploy_lp_contract },
    ConvertVec,
};
use phoenix::{ ttl::{ INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD }, validate_bps };
use phoenix::{
    ttl::{ PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD },
    utils::{ LiquidityPoolInitInfo, PoolType, StakeInitInfo, TokenInitInfo },
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
    IntoVal,
    String,
    Symbol,
    Val,
    Vec,
};

contractmeta!(key = "Description", val = "Factory for creating new Synth Markets");

#[contract]
pub struct SynthMarketFactory;

pub trait SynthMarketFactoryTrait {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        multihop_wasm_hash: BytesN<32>,
        lp_wasm_hash: BytesN<32>,
        stable_wasm_hash: BytesN<32>,
        stake_wasm_hash: BytesN<32>,
        token_wasm_hash: BytesN<32>,
        whitelisted_accounts: Vec<Address>,
        lp_token_decimals: u32
    );

    #[allow(clippy::too_many_arguments)]
    fn create_synth_market(
        env: Env,
        sender: Address,
        lp_init_info: LiquidityPoolInitInfo,
        share_token_name: String,
        share_token_symbol: String,
        pool_type: PoolType,
        amp: Option<u64>,
        default_slippage_bps: i64,
        max_allowed_fee_bps: i64
    ) -> Address;

    // ...

    fn query_markets(env: Env) -> Vec<Address>;

    fn query_market_details(env: Env, market_address: Address) -> LiquidityPoolInfo;

    fn query_all_markets_details(env: Env) -> Vec<LiquidityPoolInfo>;

    fn get_admin(env: Env) -> Address;

    fn get_config(env: Env) -> Config;
}

#[contractimpl]
impl SynthMarketFactoryTrait for SynthMarketFactory {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        market_wasm_hash: BytesN<32>,
        token_wasm_hash: BytesN<32>,
        whitelisted_accounts: Vec<Address>,
        lp_token_decimals: u32
    ) {
        if is_initialized(&env) {
            log!(&env, "Factory: Initialize: initializing contract twice is not allowed");
            panic_with_error!(&env, ContractError::AlreadyInitialized);
        }

        if whitelisted_accounts.is_empty() {
            log!(
                &env,
                "Factory: Initialize: there must be at least one whitelisted account able to create liquidity pools."
            );
            panic_with_error!(&env, ContractError::WhiteListeEmpty);
        }

        set_initialized(&env);

        save_config(&env, Config {
            admin: admin.clone(),
            multihop_address,
            lp_wasm_hash,
            stake_wasm_hash,
            token_wasm_hash,
            whitelisted_accounts,
            lp_token_decimals,
        });
        save_stable_wasm_hash(&env, stable_wasm_hash);

        save_lp_vec(&env, Vec::new(&env));

        env.events().publish(("initialize", "LP factory contract"), admin);
    }

    #[allow(clippy::too_many_arguments)]
    fn create_liquidity_pool(
        env: Env,
        sender: Address,
        lp_init_info: LiquidityPoolInitInfo,
        share_token_name: String,
        share_token_symbol: String,

        amp: Option<u64>,
        default_slippage_bps: i64,
        max_allowed_fee_bps: i64
    ) -> Address {
        sender.require_auth();
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        validate_pool_info(&pool_type, &amp);

        if !get_config(&env).whitelisted_accounts.contains(sender) {
            log!(
                &env,
                "Factory: Create Liquidity Pool: You are not authorized to create liquidity pool!"
            );
            panic_with_error!(&env, ContractError::NotAuthorized);
        }

        validate_token_info(&env, &lp_init_info.token_init_info, &lp_init_info.stake_init_info);

        let config = get_config(&env);
        let token_wasm_hash = config.token_wasm_hash;

        let pool_hash = match pool_type {
            PoolType::Xyk => config.lp_wasm_hash,
            PoolType::Stable => get_stable_wasm_hash(&env),
        };

        let market_contract_address = deploy_synth_market_contract(
            &env,
            pool_hash,
            &lp_init_info.token_init_info.token_a,
            &lp_init_info.token_init_info.token_b
        );

        validate_bps!(
            lp_init_info.swap_fee_bps,
            lp_init_info.max_allowed_slippage_bps,
            lp_init_info.max_allowed_spread_bps,
            lp_init_info.max_referral_bps,
            default_slippage_bps,
            max_allowed_fee_bps
        );

        let factory_addr = env.current_contract_address();
        let init_fn: Symbol = Symbol::new(&env, "initialize");
        let mut init_fn_args: Vec<Val> = (
            token_wasm_hash,
            lp_init_info.clone(),
            factory_addr,
            config.lp_token_decimals,
            share_token_name,
            share_token_symbol,
        ).into_val(&env);

        if let PoolType::Xyk = pool_type {
            init_fn_args.push_back(default_slippage_bps.into_val(&env));
        }

        if let PoolType::Stable = pool_type {
            init_fn_args.push_back(amp.unwrap().into_val(&env));
        }

        init_fn_args.push_back(max_allowed_fee_bps.into_val(&env));

        env.invoke_contract::<Val>(&lp_contract_address, &init_fn, init_fn_args);

        let mut lp_vec = get_lp_vec(&env);

        lp_vec.push_back(lp_contract_address.clone());

        save_lp_vec(&env, lp_vec);
        let token_a = &lp_init_info.token_init_info.token_a;
        let token_b = &lp_init_info.token_init_info.token_b;
        save_lp_vec_with_tuple_as_key(&env, (token_a, token_b), &lp_contract_address);

        env.events().publish(("create", "liquidity_pool"), &lp_contract_address);

        lp_contract_address
    }

    fn update_wasm_hashes(
        env: Env,
        lp_wasm_hash: Option<BytesN<32>>,
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
                lp_wasm_hash: lp_wasm_hash.unwrap_or(config.lp_wasm_hash),
                token_wasm_hash: token_wasm_hash.unwrap_or(config.token_wasm_hash),
                ..config
            },
        );
    }

    fn migrate_admin_key(env: Env) -> Result<(), ContractError> {
        let admin = get_config(&env).admin;
        env.storage().instance().set(&ADMIN, &admin);

        Ok(())
    }
}

#[contractimpl]
impl Factory {
    #[allow(dead_code)]
    pub fn update(env: Env, new_wasm_hash: BytesN<32>, new_stable_pool_hash: BytesN<32>) {
        let admin = get_config(&env).admin;
        admin.require_auth();

        env.deployer().update_current_contract_wasm(new_wasm_hash);
        save_stable_wasm_hash(&env, new_stable_pool_hash);
    }
}
