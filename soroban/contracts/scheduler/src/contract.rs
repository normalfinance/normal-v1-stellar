use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

use crate::{ errors, storage::{ get_admin }, storage_types::{ DataKey }, schedules::Schedule };

contractmeta!(
    key = "Description",
    val = "On-chain dollar cost average order scheduler for repetitive investments"
);

#[contract]
pub struct Scheduler;

#[contractimpl]
impl Schedule for Scheduler {
    pub fn __constructor(e: Env) {
        put_oracle_contract_id(&e, oracle_contract_id);
    }

    pub fn create_asset_schedule(e: Env, creator: Address, amm_id: Address, params: ScheduleData) -> u128 {
        creator.require_auth();
        // ...

        let schedule_id = storage::get_next_schedule_id(&e);

        let proposal_data = ProposalData {
            creator: creator.clone(),
            vote_start,
            vote_end,
            eta: 0,
            status: ProposalStatus::Open,
            executable: proposal_config.is_executable(),
        };
        storage::set_next_schedule_id(&e, schedule_id + 1);

        storage::create_proposal_data(&e, proposal_id, &proposal_data);

        ScheduleEvents::new_asset_schedule(
            &e,
            schedule_id,
            creator,
            params,
        );
        schedule_id
    }

    pub fn create_index_schedule(e: Env, creator: Address, index_id: u32, params: ScheduleData) -> u128 {
        creator.require_auth();
        // ...

        let schedule_id = storage::get_next_schedule_id(&e);

        let proposal_data = ProposalData {
            creator: creator.clone(),
            vote_start,
            vote_end,
            eta: 0,
            status: ProposalStatus::Open,
            executable: proposal_config.is_executable(),
        };
        storage::set_next_schedule_id(&e, schedule_id + 1);

        storage::create_proposal_data(&e, proposal_id, &proposal_data);

        ScheduleEvents::new_index_schedule(
            &e,
            schedule_id,
            creator,
            params,
        );
        schedule_id
    }

    fn get_schedule(e: Env, schedule_id: u32) -> Option<Proposal> {
        let config = storage::get_schedule_config(&e, schedule_id);
        let data = storage::get_schedule_data(&e, schedule_id);
        if config.is_none() || data.is_none() {
            None
        } else {
            Some(Proposal {
                id: schedule_id,
                // config: config.unwrap_optimized(),
                data: data.unwrap_optimized(),
            })
        }
    }

    pub fn deposit(e: Env, user: Address, asset: Asset, amount: u128) -> u128 {
        user.require_auth();

        // Handle asset transfer logic
        if let Some(contract_id) = asset.contract_id {
            // Transfer custom token
            let token_client = soroban_sdk::token::Client::new(env, contract_id.clone());
            token_client.transfer(&user, &env.current_contract_address(), amount);
        } else {
            // Native XLM deposit
            env.accounts().transfer(&user, &env.current_contract_address(), amount);
        }

        // Update the user's balance

        // ...

        ScheduleEvents::deposit(
            &e,
            schedule_id,
            creator,
            asset,amount
        );
    }

    pub fn execute(e: Env, keeper: Address, schedule_id: u32) {
        keeper.require_auth();

        // TODO: how to we enable auth on user's schedule for the keeper?

        let order_amount = 0;
        // validate available balance compared to order amount
        if (balance < order_amount) {
            return Err(ErrorCode::InsufficientFunds);
        }

        // Execute order
        // ...

        ScheduleEvents::order_execution(
            &e,
            schedule_id,
            keeper
        );
    }

    pub fn modify(e: Env, schedule_id: u32) {}

    pub fn withdraw(e: Env, user: Address, amount: u128) {
        user.require_auth();

        // Check user balance

        if (balance < current_balance) {
            return Err(ErrorCode::InsufficientFunds);
        }

        // ...

        if let Some(contract_id) = asset.contract_id {
            // Transfer custom token
            let token_client = soroban_sdk::token::Client::new(env, contract_id.clone());
            token_client.transfer(&user, &env.current_contract_address(), amount);
        } else {
            // Native XLM deposit
            env.accounts().transfer(&user, &env.current_contract_address(), amount);
        }

        // Update the user's balance
        // ...

        ScheduleEvents::withdrawal(
            &e,
            schedule_id,
            user, asset, amount
        );
    }

    pub fn delete(e: Env, creator: Address, schedule_id: u32) -> u128 {
        creator.require_auth();

    
        let mut schedule_data = storage::get_schedule_data(&e, schedule_id)
        .unwrap_or_else(|| panic_with_error!(&e, ErrorCode::NonExistentProposalError));


        if (creator != schedule_data.creator) {
            return Err(ErrorCode::InvalidOwner);
        }

        // ...

        ScheduleEvents::delete(
            &e,
            schedule_id
        );
    }
}
