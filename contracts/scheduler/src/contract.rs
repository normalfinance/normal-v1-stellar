use normal::{
    constants::{INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD},
    error::{ErrorCode, NormalResult},
    math::{casting::Cast, safe_math::SafeMath},
    validate_bps,
};
use soroban_sdk::{
    contract, contractimpl, contractmeta, log, panic_with_error, Address, Env, Map, Vec,
};

use crate::{
    events::SchedulerEvents,
    msg::{ConfigResponse, ScheduledResponse},
    scheduler::SchedulerTrait,
    storage::{
        get_config, get_keeper, get_schedules, save_config, save_keeper, save_schedules, utils,
        Config, Schedule, ScheduleParams, ScheduleType,
    },
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
    fn initialize(
        env: Env,
        admin: Address,
        synth_market_factory_address: Address,
        index_factory_address: Address,
        protocol_fee_bps: i64,
        keeper_fee_bps: i64,
    ) {
        if utils::is_initialized(&env) {
            log!(
                &env,
                "Scheduler: Initialize: initializing contract twice is not allowed"
            );
            panic_with_error!(&env, ErrorCode::AlreadyInitialized);
        }

        utils::set_initialized(&env);

        validate_bps!(protocol_fee_bps, keeper_fee_bps);

        let config = Config {
            synth_market_factory_address,
            index_factory_address,
            keepers: Vec::new(&env),
            protocol_fee_bps,
            keeper_fee_bps,
            protocol_fees_to_collect: Map::new(&env),
        };
        save_config(&env, config);

        utils::save_admin(&env, &admin);

        SchedulerEvents::initialize(&env, admin);
    }

    #[allow(clippy::too_many_arguments)]
    fn update_config(
        env: Env,
        sender: Address,
        synth_market_factory_address: Option<Address>,
        index_factory_address: Option<Address>,
        protocol_fee_bps: Option<i64>,
        keeper_fee_bps: Option<i64>,
    ) {
        sender.require_auth();
        utils::is_admin(&env, sender);

        let mut config = get_config(&env);

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

    fn update_keepers(env: Env, sender: Address, to_add: Vec<Address>, to_remove: Vec<Address>) {
        sender.require_auth();
        utils::is_admin(&env, sender);

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let config = get_config(&env);

        let mut keepers = config.keepers;

        to_add.into_iter().for_each(|addr| {
            if !keepers.contains(addr.clone()) {
                keepers.push_back(addr);
            }
        });

        to_remove.into_iter().for_each(|addr| {
            if let Some(id) = keepers.iter().position(|x| x == addr) {
                keepers.remove(id as u32);
            }
        });

        save_config(&env, Config { keepers, ..config })
    }

    fn collect_protocol_fees(env: Env, sender: Address, to: Address) {
        sender.require_auth();
        utils::is_admin(&env, sender);

        let config = get_config(&env);

        for (address, amount) in config.protocol_fees_to_collect.iter() {
            utils::transfer_token(&env, &address, &env.current_contract_address(), &to, amount);
            // TODO: set fee to collect to zero
        }
    }

    // ################################################################
    //                             KEEPER
    // ################################################################

    fn execute_schedule(
        env: Env,
        sender: Address,
        user: Address,
        schedule_timestamp: u64,
    ) -> NormalResult {
        sender.require_auth();

        let config = get_config(&env);
        let now = env.ledger().timestamp();

        if !config.keepers.contains(sender.clone()) {
            log!(
                &env,
                "Scheduler: Execute Schedule: You are not authorized to execute schedule orders!"
            );
            panic_with_error!(&env, ErrorCode::NotAuthorized);
        }

        let schedules = get_schedules(&env, &user);

        let mut target_schedule = match schedules
            .schedules
            .iter()
            .find(|s| s.schedule_timestamp == schedule_timestamp)
        {
            Some(schedule) => schedule,
            None => panic_with_error!(&env, ErrorCode::AdminNotSet), // TODO:
        };

        // TODO: Validate the schedule needs to be executed

        // TODO: Compute protocol and keeper fee
        let protocol_fee: u64 = 0;
        let keeper_fee: u64 = 0;

        // Calculate order amount
        let order_quote_asset_amount = calculate_order_amount(&env, &target_schedule)?;

        // Validate available balance compared to order amount
        let current_balance = schedules
            .balances
            .get(target_schedule.clone().quote_asset)
            .unwrap_or(0);

        if order_quote_asset_amount > current_balance {
            panic_with_error!(&env, ErrorCode::InsufficientFunds);
        }

        // Execute the order
        match target_schedule.schedule_type {
            ScheduleType::Asset => {
                // TODO:
                // let amm_response: SwapResponse = env.invoke_contract(
                //     &target_schedule.target_contract_address,
                //     &Symbol::new(&env, "swap"),
                //     vec![
                //         &env,
                //         user.into_val(&env),
                //         amount,
                //         other_amount_threshold,
                //         sqrt_price_limit,
                //         amount_specified_is_input,
                //         a_to_b
                //     ]
                // );
                // assert!(
                //     amm_response.ask_amount.is_some(),
                //     "Scheduler: Create Schedule: Invalid AMM response"
                // );
            }
            ScheduleType::Index => {
                // TODO:
                // let index_response: MintResponse = env.invoke_contract(
                //     &target_schedule.target_contract_address,
                //     &Symbol::new(&env, "mint"),
                //     vec![&env, user.into_val(&env), amount]
                // );
                // assert!(
                //     index_response.mint_amount.is_some(),
                //     "Scheduler: Create Schedule: Invalid Index response"
                // );
            }
        }

        // Update protocol and keeper fees
        let mut keeper = get_keeper(&env, &sender);
        // let keeper_fee_before = keeper.fees_owed.get(target_schedule.quote_asset);
        let keeper_fee_before = match keeper.fees_owed.get(target_schedule.clone().quote_asset) {
            Some(bal) => bal,
            None => panic_with_error!(&env, ErrorCode::AdminNotSet), // TODO:
        };

        keeper.fees_owed.set(
            target_schedule.clone().quote_asset,
            keeper_fee_before.safe_add(keeper_fee.cast::<i128>(&env)?, &env)?,
        );

        // ...

        // Update the Schedule
        target_schedule.executed_orders += 1;
        target_schedule.total_executed = target_schedule
            .total_executed
            .safe_add(order_quote_asset_amount, &env)?;
        target_schedule.total_fees_paid = target_schedule
            .total_fees_paid
            .safe_add(protocol_fee, &env)?
            .safe_add(keeper_fee, &env)?;
        target_schedule.last_order_ts = now;

        save_schedules(&env, &sender, &schedules);

        SchedulerEvents::order_execution(&env, sender, user, schedule_timestamp);

        Ok(())
    }

    fn collect_keeper_fees(env: Env, sender: Address) {
        sender.require_auth();

        let mut keeper = get_keeper(&env, &sender);

        for (address, amount) in keeper.fees_owed.iter() {
            utils::transfer_token(
                &env,
                &address,
                &env.current_contract_address(),
                &sender,
                amount,
            );
            // TODO: set fee to collect to zero
        }

        keeper.last_fee_collection_time = env.ledger().timestamp();

        save_keeper(&env, &sender, &keeper);
    }

    // ################################################################
    //                             USER
    // ################################################################

    fn deposit(env: Env, sender: Address, asset: Address, amount: i128) {
        utils::check_nonnegative_amount(amount);
        sender.require_auth();

        let mut schedules = get_schedules(&env, &sender);
        let current_balance = schedules.balances.get(asset.clone()).unwrap_or(0);

        utils::transfer_token(
            &env,
            &asset.clone(),
            &sender,
            &env.current_contract_address(),
            amount,
        );

        schedules
            .balances
            .set(asset.clone(), current_balance + amount);

        SchedulerEvents::deposit(&env, sender, asset, amount);
    }

    fn withdraw(env: Env, sender: Address, asset: Address, amount: i128) {
        utils::check_nonnegative_amount(amount);
        sender.require_auth();

        let mut schedules = get_schedules(&env, &sender);
        let current_balance = schedules.balances.get(asset.clone()).unwrap_or(0);

        if amount > current_balance {
            // return Err(ErrorCode::InsufficientFunds);
            panic_with_error!(&env, ErrorCode::InsufficientFunds);
        }

        utils::transfer_token(
            &env,
            &asset.clone(),
            &env.current_contract_address(),
            &sender,
            amount,
        );

        schedules
            .balances
            .set(asset.clone(), current_balance - amount);

        SchedulerEvents::withdrawal(&env, sender, asset, amount);
    }

    fn create_schedule(env: Env, sender: Address, params: ScheduleParams) {
        sender.require_auth();

        // TODO: Make sure target_contract_address exists

        let now = env.ledger().timestamp();
        let mut schedules = get_schedules(&env, &sender);

        let schedule = Schedule {
            schedule_type: params.schedule_type,
            quote_asset: params.quote_asset.clone(),
            target_contract_address: params.target_contract_address.clone(),
            base_asset_amount_per_interval: params.base_asset_amount_per_interval,
            direction: params.direction,
            interval_seconds: params.interval_seconds,
            min_price: params.min_price,
            max_price: params.max_price,
            schedule_timestamp: env.ledger().timestamp(),
            total_orders: 0,
            executed_orders: 0,
            total_executed: 0,
            total_fees_paid: 0,
            last_updated_ts: now,
            last_order_ts: 0,
        };
        schedules.schedules.push_back(schedule);

        save_schedules(&env, &sender, &schedules);

        SchedulerEvents::new_schedule(
            &env,
            sender,
            now,
            params.schedule_type,
            params.quote_asset,
            params.target_contract_address,
        );
    }

    fn delete_schedule(env: Env, sender: Address, schedule_timestamp: u64) {
        sender.require_auth();

        let mut schedules = get_schedules(&env, &sender);

        remove_schedule(&env, &mut schedules.schedules, schedule_timestamp);

        save_schedules(&env, &sender, &schedules);

        SchedulerEvents::delete_schedule(&env, sender, schedule_timestamp);
    }

    // ################################################################
    //                             QUERIES
    // ################################################################

    fn query_config(env: Env) -> ConfigResponse {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        ConfigResponse {
            config: get_config(&env),
        }
    }

    fn query_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        utils::get_admin(&env)
    }

    fn query_scheduled(env: Env, address: Address) -> ScheduledResponse {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let schedules = get_schedules(&env, &address);
        ScheduledResponse {
            schedules: schedules.schedules,
        }
    }
}

fn calculate_order_amount(env: &Env, schedule: &Schedule) -> NormalResult<i128> {
    let price: i128 = 0; // TODO: get the price
    let order_quote_asset_amount: i128 = schedule
        .base_asset_amount_per_interval
        .cast::<i128>(env)?
        .safe_mul(price, env)?;

    Ok(order_quote_asset_amount)
}

// Function to remove a schedule from the vector
fn remove_schedule(env: &Env, schedules: &mut Vec<Schedule>, schedule_timestamp: u64) {
    // Find the index of the stake that matches the given stake and schedule_timestamp
    if let Some(index) = schedules
        .iter()
        .position(|s| s.schedule_timestamp == schedule_timestamp)
    {
        // Remove the stake at the found index
        schedules.remove(index as u32);
    } else {
        // Schedule not found, return an error
        log!(&env, "Schedule: Remove schedule: Schedule not found");
        // panic_with_error!(&env, ContractError::StakeNotFound);
    }
}

// fn validate_target_info(env: Env, schedule_type: &ScheduleType, target_contract_address: Address) {
//     match schedule_type {
//         ScheduleType::Asset => {
//             // let amm_response: SimulateSwapResponse = env.invoke_contract(
//             //     &target_contract_address,
//             //     &Symbol::new(&env, "simulate_swap"),
//             //     Vec::new(&env), // TODO: update args OR use health_ping call instead
//             // );
//             // assert!(
//             //     amm_response.ask_amount.is_some(),
//             //     "Scheduler: Create Schedule: Invalid AMM response"
//             // )
//         }
//         ScheduleType::Index => {
//             // let index_response: SimulateMintResponse = env.invoke_contract(
//             //     &target_contract_address,
//             //     &Symbol::new(&env, "simulate_mint"),
//             //     Vec::new(&env), // TODO: update args OR use health_ping call instead
//             // );
//             // assert!(
//             //     index_response.mint_amount.is_some(),
//             //     "Scheduler: Create Schedule: Invalid Index response"
//             // )
//         }
//     }
// }
