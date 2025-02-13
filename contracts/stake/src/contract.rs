use normal::constants::{INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD};
use soroban_sdk::{
    contract, contractimpl, log, panic_with_error, Address, BytesN, Env, Map, String, Vec,
};

use crate::{
    errors::Errors,
    storage::{
        get_config, get_stakes, save_config, save_stakes,
        utils::{is_initialized, set_initialized},
        Config, Stake,
    },
    token_contract,
};

#[contract]
pub struct Staking;

#[contractimpl]
/// Implementation of the SEP-41 Token trait.
impl StakingTrait for Staking {
    #[allow(clippy::too_many_arguments)]
    fn initialize(env: Env, admin: Address, governor: Address, emission_token: Address) {
        if is_initialized(&env) {
            log!(
                &env,
                "Stake: Initialize: initializing contract twice is not allowed"
            );
            panic_with_error!(&env, Errors::AlreadyInitialized);
        }

        set_initialized(&env);

        env.events().publish(
            ("initialize", "LP Share token staking contract"),
            &emission_token,
        );

        let config = Config {
            admin,
            governor,
            emission_token,
            emission_infos: Map::new(&env),
        };
        save_config(&env, config);

        // utils::save_admin_old(&env, &admin);
        // utils::init_total_staked(&env);
        // save_total_staked_history(&env, map![&env]);
    }

    fn update_market_emissions(
        env: Env,
        sender: Address,
        lp_token: Address,
        amount: i128,
        deadline: u64,
    ) {
        sender.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let config = get_config(&env);
        if sender != config.governor {
            log!(env, "Stake: create distribution: Non-authorized creation!");
            panic_with_error!(&env, Errors::Unauthorized);
        }

        // ...

        save_config(&env, config);
    }

    // ################################################################
    //                             Users
    // ################################################################

    fn lock(env: Env, sender: Address, tokens: i128) {
        sender.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let ledger = env.ledger();
        let config = get_config(&env);

        if tokens < config.min_bond {
            log!(
                &env,
                "Stake: Bond: Trying to stake less than minimum required"
            );
            panic_with_error!(&env, Errors::InvalidBond);
        }

        let lp_token_client = token_contract::Client::new(&env, &config.lp_token);
        lp_token_client.transfer(&sender, &env.current_contract_address(), &tokens);

        let mut stakes = get_stakes(&env, &sender);

        stakes.total_stake += tokens;
        let stake = Stake {
            stake: tokens,
            stake_timestamp: ledger.timestamp(),
        };
        stakes.stakes.push_back(stake);

        save_stakes(&env, &sender, &stakes);

        env.events().publish(("bond", "user"), &sender);
        env.events().publish(("bond", "token"), &config.lp_token);
        env.events().publish(("bond", "amount"), tokens);
    }

    fn unlock(env: Env, sender: Address, stake_amount: i128, stake_timestamp: u64) {
        sender.require_auth();

        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let config = get_config(&env);

        let mut stakes = get_stakes(&env, &sender);

        remove_stake(&env, &mut stakes.stakes, stake_amount, stake_timestamp);
        stakes.total_stake -= stake_amount;

        let lp_token_client = token_contract::Client::new(&env, &config.lp_token);
        lp_token_client.transfer(&env.current_contract_address(), &sender, &stake_amount);

        save_stakes(&env, &sender, &stakes);

        env.events().publish(("unbond", "user"), &sender);
        env.events().publish(("unbond", "token"), &config.lp_token);
        env.events().publish(("unbond", "amount"), stake_amount);
    }

    fn withdraw_rewards(env: Env, sender: Address) {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        env.events().publish(("withdraw_rewards", "user"), &sender);

        let mut stakes = get_stakes(&env, &sender);

        for asset in get_distributions(&env) {
            let pending_reward = calculate_pending_rewards(&env, &asset, &stakes);
            env.events()
                .publish(("withdraw_rewards", "reward_token"), &asset);

            token_contract::Client::new(&env, &asset).transfer(
                &env.current_contract_address(),
                &sender,
                &pending_reward,
            );
        }
        stakes.last_reward_time = env.ledger().timestamp();
        save_stakes(&env, &sender, &stakes);
    }

    // ################################################################
    //                             Queries
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
        get_admin_old(&env)
    }

    fn query_staked(env: Env, address: Address) -> StakedResponse {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let stakes = get_stakes(&env, &address);
        StakedResponse {
            stakes: stakes.stakes,
            total_stake: stakes.total_stake,
            last_reward_time: stakes.last_reward_time,
        }
    }
}

#[contractimpl]
impl Staking {
    #[allow(dead_code)]
    pub fn update(env: Env, new_wasm_hash: BytesN<32>) {
        let admin = get_admin_old(&env);
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
}

// Function to remove a stake from the vector
fn remove_stake(env: &Env, stakes: &mut Vec<Stake>, stake: i128, stake_timestamp: u64) {
    // Find the index of the stake that matches the given stake and stake_timestamp
    if let Some(index) = stakes
        .iter()
        .position(|s| s.stake == stake && s.stake_timestamp == stake_timestamp)
    {
        // Remove the stake at the found index
        stakes.remove(index as u32);
    } else {
        // Stake not found, return an error
        log!(&env, "Stake: Remove stake: Stake not found");
        panic_with_error!(&env, Errors::StakeNotFound);
    }
}
