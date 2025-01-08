use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

use crate::{
    errors::ErrorCode,
    storage::{ get_config, is_initialized, save_config, set_initialized, Asset, Config, ADMIN },
    scheduler::SchedulerTrait,
    token_contract,
};

contractmeta!(
    key = "Description",
    val = "On-chain dollar cost average order scheduler for repetitive investments"
);

#[contract]
pub struct Scheduler;

#[contractimpl]
impl SchedulerTrait for Scheduler {
    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        env: Env,
        admin: Address,
        synth_market_factory_address: Address,
        index_factory_address: Address,
        keeper_accounts: Vec<Address>,
        protocol_fee_bps: u64,
        keeper_fee_bps: u64
    ) {
        if is_initialized(&env) {
            log!(&env, "Scheduler: Initialize: initializing contract twice is not allowed");
            panic_with_error!(&env, ErrorCode::AlreadyInitialized);
        }

        if keeper_accounts.is_empty() {
            log!(
                &env,
                "Scheduler: Initialize: there must be at least one keeper account able to execute schedule orders."
            );
            panic_with_error!(&env, ErrorCode::KeeperAccountsEmpty);
        }

        set_initialized(&env);

        save_config(&env, Config {
            admin: admin.clone(),
            synth_market_factory_address,
            index_factory_address,
            keeper_accounts,
            protocol_fee_bps,
            keeper_fee_bps,
        });

        SchedulerEvents::initialize(&env, admin);
    }

    #[allow(clippy::too_many_arguments)]
    fn update_config(
        env: Env,
        new_admin: Option<Address>,
        synth_market_factory_address: Option<Address>,
        index_factory_address: Option<Address>,
        protocol_fee_bps: Option<u64>,
        keeper_fee_bps: Option<u64>
    ) {
        let admin: Address = utils::get_admin_old(&env);
        admin.require_auth();
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // TODO: do we need manual admin check here?

        let mut config = get_config(&env);

        if let Some(new_admin) = new_admin {
            utils::save_admin_old(&env, new_admin);
        }
        if let Some(synth_market_factory_address) = synth_market_factory_address {
            config.synth_market_factory_address = synth_market_factory_address;
        }
        if let Some(index_factory_address) = index_factory_address {
            config.index_factory_address = index_factory_address;
        }
        if let Some(protocol_fee_bps) = protocol_fee_bps {
            validate_bps!(protocol_fee_bps);
            config.protocol_fee_bps = protocol_fee_bps;
        }
        if let Some(keeper_fee_bps) = keeper_fee_bps {
            validate_bps!(keeper_fee_bps);
            config.keeper_fee_bps = keeper_fee_bps;
        }

        save_config(&env, config);
    }

    fn update_keeper_accounts(
        env: Env,
        sender: Address,
        to_add: Vec<Address>,
        to_remove: Vec<Address>
    ) {
        sender.require_auth();
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let config = get_config(&env);

        if config.admin != sender {
            log!(&env, "Scheduler: Update keeper accounts: You are not authorized!");
            panic_with_error!(&env, ErrorCode::NotAuthorized);
        }

        let mut keeper_accounts = config.keeper_accounts;

        to_add.into_iter().for_each(|addr| {
            if !keeper_accounts.contains(addr.clone()) {
                keeper_accounts.push_back(addr);
            }
        });

        to_remove.into_iter().for_each(|addr| {
            if let Some(id) = keeper_accounts.iter().position(|x| x == addr) {
                keeper_accounts.remove(id as u32);
            }
        });

        save_config(&env, Config {
            keeper_accounts,
            ..config
        })
    }

    fn collect_protocol_fees(env: Env, sender: Address, to: Address) {
        sender.require_auth();

        let config = get_config(&env);

        if config.admin != sender {
            log!(&env, "Scheduler: Collect protocol fees: You are not authorized!");
            panic_with_error!(&env, ErrorCode::NotAuthorized);
        }

        utils::transfer_tokens(
            &env,
            _,
            &env.current_contract_address(),
            &to,
            &config.fees_to_collect
        );

        config.fees_to_collect = 0;
    }

    // User

    pub fn deposit(env: Env, user: Address, asset: Asset) -> u128 {
        if asset.amount <= 0 {
            panic!("Amount must be positive");
        }

        user.require_auth();

        match asset.address {
            // Handle XLM deposits
            None => {
                env.pay(&user, &env.current_contract_address(), asset.amount); // Transfer XLM to the contract
            }
            // Handle token deposits
            Some(token_address) => {
                let token_client = token_contract::Client::new(&env, &token_address);
                token_client.transfer(&user, &env.current_contract_address(), &asset.amount);
            }
        }

        // Update the user's balance for the given asset
        let key = DataKey::Balance(user.clone(), asset.address.clone());
        let current_balance: i128 = env.storage().get(&key).unwrap_or(0);
        env.storage().set(&key, current_balance + asset.amount);

        SchedulerEvents::deposit(&env, user, asset.address, asset.amount);
    }

    pub fn withdraw(e: Env, user: Address, asset: Option<Address>, amount: u128) {
        if amount <= 0 {
            panic!("Amount must be positive");
        }

        user.require_auth();

        // Check user balance
        let key = DataKey::Balance(user.clone(), asset.clone());
        let current_balance: i128 = env.storage().get(&key).unwrap_or(0);

        if amount > current_balance {
            return Err(ErrorCode::InsufficientFunds);
        }

        match asset {
            // Handle XLM withdrawals
            None => {
                env.pay(&env.current_contract_address(), &user, amount); // Transfer XLM to the user
            }
            // Handle token withdrawals
            Some(token_address) => {
                let token_client = token_contract::Client::new(&env, &token_address);
                token_client.transfer(&env.current_contract_address(), &user, &amount);
            }
        }

        // Update the user's balance for the given asset
        env.storage().set(&key, current_balance - amount);

        ScheduleEvents::withdraw(&env, user, asset, amount);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_schedule(
        env: Env,
        user: Address,
        schedule_type: ScheduleType,
        target_contract_address: Address,
        base_asset_amount_per_interval: u64,
        direction: OrderDirection,
        active: bool,
        interval_seconds: u64,
        min_price: Option<u16>,
        max_price: Option<u16>
    ) {
        user.require_auth();

        // Make sure target_contract_address exists
        validate_target_info(&schedule_type, &target_contract_address);

        let mut schedules = get_schedules(&env, &user);

        let schedule = Schedule {
            schedule_type,
            target_contract_address: target_contract_address.clone(),
            base_asset_amount_per_interval,
            direction,
            active,
            interval_seconds,
            min_price,
            max_price,
            schedule_timestamp: env.ledger().timestamp(),
        };
        schedules.push_back(schedule);

        save_schedules(&env, &user, &schedules);

        ScheduleEvents::create_schedule(&env, user, schedule);
    }

    #[allow(clippy::too_many_arguments)]
    fn update_schedule(
        env: Env,
        user: Address,
        schedule_timestamp: u64,
        base_asset_amount_per_interval: Option<u64>,
        direction: Option<OrderDirection>,
        active: Option<bool>,
        interval_seconds: Option<u64>,
        total_orders: Option<u16>,
        min_price: Option<u16>,
        max_price: Option<u16>
    ) {
        user.require_auth();
        // env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // TODO: confirm users owns schedule on schedule_timestamp
        // ...
        if user != schedule.creator {
            return Err(ErrorCode::InvalidScheduleOwner);
        }

        let mut schedule = get_schedule(&env, user, schedule_timestamp);

        if let Some(base_asset_amount_per_interval) = base_asset_amount_per_interval {
            schedule.base_asset_amount_per_interval = base_asset_amount_per_interval;
        }
        if let Some(direction) = direction {
            schedule.direction = direction;
        }
        if let Some(active) = active {
            schedule.active = active;
        }
        if let Some(interval_seconds) = interval_seconds {
            schedule.interval_seconds = interval_seconds;
        }
        if let Some(total_orders) = total_orders {
            schedule.total_orders = total_orders;
        }
        if let Some(min_price) = min_price {
            schedule.min_price = min_price;
        }
        if let Some(max_price) = max_price {
            schedule.max_price = max_price;
        }
        schedule.last_updated_ts = env.ledger().timestamp();

        save_schedule(&env, schedule);
    }

    pub fn delete_schedule(e: Env, user: Address, schedule_timestamp: u64) {
        user.require_auth();

        let mut schedule = get_schedule(&env, user, schedule_timestamp);

        if user != schedule.creator {
            return Err(ErrorCode::InvalidScheduleOwner);
        }

        // TODO: delete schedule...

        ScheduleEvents::delete_schedule(&env, user, schedule_timestamp);
    }

    // KEEPER

    pub fn execute_schedule(env: Env, sender: Address, user: Address, schedule_timestamp: u64) {
        sender.require_auth();

        if !get_config(&env).keeper_accounts.contains(sender) {
            log!(
                &env,
                "Scheduler: Execute Schedule: You are not authorized to execute schedule orders!"
            );
            panic_with_error!(&env, ErrorCode::NotAuthorized);
        }

        // TODO: how do we error if no schedule is found
        let mut schedule = get_schedule_by_timestamp(&env, &user, &schedule_timestamp).ok_or(
            "Schedule not found"
        )?;

        // TODO: Validate the schedule needs to be executed

        // Calculate order amount
        let price = 0; // TODO: get the price
        let order_quote_asset_amount = schedule.base_asset_amount_per_interval * price;

        // Validate available balance compared to order amount
        let key = DataKey::Balance(user.clone(), schedule.asset.clone());
        let current_balance: i128 = env.storage().get(&key).unwrap_or(0);

        if order_quote_asset_amount > current_balance {
            return Err(ErrorCode::InsufficientFunds);
        }

        // Execute the order
        match schedule.schedule_type {
            ScheduleType::Asset => {
                let amm_response: SwapResponse = env.invoke_contract(
                    &schedule.target_contract_address,
                    &Symbol::new(&env, "swap"),
                    vec![
                        &env,
                        user.into_val(&env),
                        amount,
                        other_amount_threshold,
                        sqrt_price_limit,
                        amount_specified_is_input,
                        a_to_b
                    ]
                );
                assert!(
                    amm_response.ask_amount.is_some(),
                    "Scheduler: Create Schedule: Invalid AMM response"
                );
            }
            ScheduleType::Index => {
                let index_response: MintResponse = env.invoke_contract(
                    &schedule.target_contract_address,
                    &Symbol::new(&env, "mint"),
                    vec![&env, user.into_val(&env), amount]
                );
                assert!(
                    index_response.mint_amount.is_some(),
                    "Scheduler: Create Schedule: Invalid Index response"
                );
            }
        }

        // Collect protocol and keeper fees
        let token_client = token_contract::Client::new(&env, &token_address);

        // token_client.transfer(&env.current_contract_address(), &contract, &protocol_fee_amount);
        token_client.transfer(&env.current_contract_address(), &keeper, &keeper_fee_amount);

        // Update the Schedule
        // TODO: do we need checked/safe add here?
        schedule.executed_orders += 1;
        schedule.total_executed += order_quote_asset_amount;
        schedule.total_fees_paid += protocol_fee_amount + keeper_fee_amount;
        schedule.last_order_ts = env.ledger().timestamp();

        ScheduleEvents::order_execution(&env, sender, user, schedule_timestamp);
    }

    pub fn collect_keeper_fees(env: Env, keeper: Address, to: Option<Address>) {
        keeper.require_auth();

        let mut keeper_info = get_keeper_info(&env, &keeper);

        let recipient_address = match to {
            Some(to_address) => to_address, // Use the provided `to` address
            None => keeper, // Otherwise use the keeper address
        };

        for asset in keeper_info.fees_owed {
            utils::transfer_tokens(
                &env,
                &asset.address,
                &env.current_contract_address(),
                &recipient_address,
                &asset.amount
            );
            asset.amount = 0; // TODO: is this the correct way to zero this?
        }

        // Update keeper fees
        keeper_info.last_fee_collection_time = env.ledger().timestamp();

        save_keeper_info(&env, &keeper, &keeper_info);
    }

    // Queries

    fn query_schedules(env: Env) -> Vec<Address> {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        // get_lp_vec(&env)
    }

    fn query_pool_details(env: Env, pool_address: Address) -> LiquidityPoolInfo {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        // let pool_response: LiquidityPoolInfo = env.invoke_contract(
        //     &pool_address,
        //     &Symbol::new(&env, "query_pool_info_for_factory"),
        //     Vec::new(&env),
        // );
        // pool_response
    }
}

fn validate_target_info(schedule_type: &ScheduleType, target_contract_address: Address) {
    match schedule_type {
        ScheduleType::Asset => {
            let amm_response: SimulateSwapResponse = env.invoke_contract(
                &target_contract_address,
                &Symbol::new(&env, "simulate_swap"),
                Vec::new(&env) // TODO: update args OR use health_ping call instead
            );
            assert!(
                amm_response.ask_amount.is_some(),
                "Scheduler: Create Schedule: Invalid AMM response"
            )
        }
        ScheduleType::Index => {
            let index_response: SimulateMintResponse = env.invoke_contract(
                &target_contract_address,
                &Symbol::new(&env, "simulate_mint"),
                Vec::new(&env) // TODO: update args OR use health_ping call instead
            );
            assert!(
                index_response.mint_amount.is_some(),
                "Scheduler: Create Schedule: Invalid Index response"
            )
        }
    }
}
