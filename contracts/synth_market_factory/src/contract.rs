use crate::{
    error::ContractError,
    stake_contract::StakedResponse,
    storage::{
        get_config, get_lp_vec, get_stable_wasm_hash, is_initialized, save_config, save_lp_vec,
        save_lp_vec_with_tuple_as_key, save_stable_wasm_hash, set_initialized, Asset, Config,
        LiquidityPoolInfo, LpPortfolio, PairTupleKey, StakePortfolio, UserPortfolio, ADMIN,
    },
    utils::{deploy_and_initialize_multihop_contract, deploy_lp_contract},
    ConvertVec,
};
use normal::{
    constants::{INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD},
    validate_bps,
};
use normal::{
    constants::{PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD},
    error::ErrorCode,
    oracle::OracleSource,
    utils::{LiquidityPoolInitInfo, PoolType, StakeInitInfo, TokenInitInfo},
};
use soroban_sdk::{
    contract, contractimpl, contractmeta, log, panic_with_error, vec, Address, BytesN, Env,
    IntoVal, String, Symbol, Val, Vec,
};

contractmeta!(
    key = "Description",
    val = "Factory for creating new Synth Markets"
);

#[contract]
pub struct SynthMarketFactory;

pub trait SynthMarketFactoryTrait {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        admin: Address,
        governor: Address,
        synth_market_wasm_hash: BytesN<32>,
        quote_token_whitelist: Vec<Address>,
    );

    #[allow(clippy::too_many_arguments)]
    fn create_synth_market(env: Env, sender: Address, params: SynthMarketParams) -> Address;

    // TODO: should this be here or on the Market?
    fn initialize_synth_market_shutdown() {}

    fn delete_initialized_synth_market(env: Env, sender: Address, market: Address);

    fn freeze_oracle(env: Env, keeper: Address, market: Address);

    fn unfreeze_oracle(env: Env, keeper: Address, market: Address);

    fn update_oracle_guard_rails(
        env: Env,
        admin: Address,
        oracle_guard_rails: OracleGuardRails,
    ) -> OracleGuardRails;

    fn update_emergency_oracles(
        env: Env,
        sender: Address,
        to_add: Vec<Address>,
        to_remove: Vec<Address>,
    ) -> Vec<Address>;

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
        governor: Address,
        synth_market_wasm_hash: BytesN<32>,
        quote_token_whitelist: Vec<Address>,
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
                synth_market_wasm_hash,
                quote_token_whitelist,
                emergency_oracle_accounts: [],
                oracle_guard_rails: OracleGuardRails::default(),
            },
        );

        save_lp_vec(&env, Vec::new(&env));

        env.events()
            .publish(("initialize", "LP factory contract"), admin);
    }

    #[allow(clippy::too_many_arguments)]
    fn create_synth_market(
        env: Env,
        sender: Address,
        params: SynthMarketParams, // // Market
                                   // name: String,
                                   // token_name: String,
                                   // token_symbol: String,
                                   // active_status: bool,
                                   // synthetic_tier: SyntheticTier,

                                   // // Oracle
                                   // oracle_source: OracleSource,
                                   // oracle: Address,

                                   // // Margin
                                   // margin_ratio_initial: u32,
                                   // margin_ratio_maintenance: u32,
                                   // imf_factor: u32,

                                   // // Liquidation
                                   // liquidation_penalty: u32,
                                   // liquidator_fee: u32,
                                   // insurance_fund_liquidation_fee: u32,
                                   // debt_ceiling: u128,
                                   // debt_floor: u32
    ) -> Address {
        sender.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        // validate_pool_info(&pool_type, &amp);

        // validate_token_info(&env, &lp_init_info.token_init_info, &lp_init_info.stake_init_info);

        let config = get_config(&env);

        if !Self::is_token_allowed(&env, &params.quote_token) {
            return Err(ErrorCode::TokenNotAllowed);
        }

        let token_wasm_hash = config.token_wasm_hash;

        let synth_market_hash = config.synth_market_wasm_hash;

        let market_contract_address = deploy_synth_market_contract(
            &env,
            synth_market_hash,
            &params.token_init_info.token_a,
            &params.token_init_info.token_b,
        );

        validate_bps!(
            params.swap_fee_bps,
            params.max_allowed_slippage_bps,
            params.max_allowed_spread_bps,
            params.max_referral_bps,
            default_slippage_bps,
            max_allowed_fee_bps
        );

        let factory_addr = env.current_contract_address();
        let init_fn: Symbol = Symbol::new(&env, "initialize");
        let mut init_fn_args: Vec<Val> = (
            token_wasm_hash,
            params.clone(),
            factory_addr,
            config.lp_token_decimals,
            share_token_name,
            share_token_symbol,
        )
            .into_val(&env);

        init_fn_args.push_back(default_slippage_bps.into_val(&env));

        init_fn_args.push_back(max_allowed_fee_bps.into_val(&env));

        env.invoke_contract::<Val>(&market_contract_address, &init_fn, init_fn_args);

        let mut market_vec = get_market_vec(&env);

        market_vec.push_back(market_contract_address.clone());

        save_market_vec(&env, market_vec);
        let token_a = &market_init_info.token_init_info.token_a;
        let token_b = &market_init_info.token_init_info.token_b;
        save_market_vec_with_tuple_as_key(&env, (token_a, token_b), &market_contract_address);

        env.events()
            .publish(("create", "liquidity_pool"), &market_contract_address);

        market_contract_address
    }

    fn delete_initialized_synth_market(env: Env, sender: Address, market: Address) {
        let mut market = query_market_details();

        log!(&env, "market {}", market.name);
        // let config = get_config(&env);

        // to preserve all protocol invariants, can only remove the last market if it hasn't been "activated"

        validate!(
            state.number_of_markets - 1 == market_index,
            ErrorCode::InvalidMarketAccountforDeletion,
            "state.number_of_markets={} != market_index={}",
            state.number_of_markets,
            market_index
        )?;
        validate!(
            market.status == MarketStatus::Initialized,
            ErrorCode::InvalidMarketAccountforDeletion,
            "market.status != Initialized"
        )?;
        validate!(
            market.number_of_users == 0,
            ErrorCode::InvalidMarketAccountforDeletion,
            "market.number_of_users={} != 0",
            market.number_of_users
        )?;
        validate!(
            market.market_index == market_index,
            ErrorCode::InvalidMarketAccountforDeletion,
            "market_index={} != market.market_index={}",
            market_index,
            market.market_index
        )?;

        safe_decrement!(state.number_of_markets, 1);
        // ...
    }

    fn freeze_oracle(env: Env, keeper: Address, market_address: Address) {}

    fn unfreeze_oracle(env: Env, keeper: Address, market_address: Address) {}

    fn update_oracle_guard_rails(env: Env) -> OracleGuardRails {}

    fn update_emergency_oracles(
        env: Env,
        sender: Address,
        to_add: Vec<Address>,
        to_remove: Vec<Address>,
    ) -> Vec<Address> {
        sender.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let config = get_config(&env);

        // TODO: do we want to limit this to the admin or the DAO?
        if config.admin != sender {
            log!(
                &env,
                "Synth Marekt Factory: Update emergency oracle accounts: You are not authorized!"
            );
            panic_with_error!(&env, ContractError::NotAuthorized);
        }

        let mut emergency_oracle_accounts = config.emergency_oracle_accounts;

        to_add.into_iter().for_each(|addr| {
            if !emergency_oracle_accounts.contains(addr.clone()) {
                emergency_oracle_accounts.push_back(addr);
            }
        });

        to_remove.into_iter().for_each(|addr| {
            if let Some(id) = emergency_oracle_accounts.iter().position(|x| x == addr) {
                emergency_oracle_accounts.remove(id as u32);
            }
        });

        save_config(
            &env,
            Config {
                emergency_oracle_accounts,
                ..config
            },
        );

        emergency_oracle_accounts
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
