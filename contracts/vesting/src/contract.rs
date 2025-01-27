use normal::{
    constants::{INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD},
    utils::{convert_i128_to_u128, convert_u128_to_i128},
};
use soroban_sdk::{
    contract, contractimpl, contractmeta, log, panic_with_error, Address, BytesN, Env, Vec,
};

use crate::{
    error::ContractError,
    storage::{
        get_admin_old, get_all_vestings, get_max_vesting_complexity, get_token_info, get_vesting,
        is_initialized, save_admin_old, save_max_vesting_complexity, save_token_info, save_vesting,
        set_initialized, update_vesting, VestingInfo, VestingSchedule, VestingTokenInfo, ADMIN,
    },
    token_contract,
    utils::{check_duplications, validate_vesting_schedule},
};

// Metadata that is added on to the WASM custom section
contractmeta!(
    key = "Description",
    val = "Normal Protocol Token Vesting Contract"
);
#[contract]
pub struct Vesting;

#[allow(dead_code)]
pub trait VestingTrait {
    fn initialize(
        env: Env,
        admin: Address,
        vesting_token: VestingTokenInfo,
        max_vesting_complexity: u32,
    );

    fn create_vesting_schedules(env: Env, vesting_accounts: Vec<VestingSchedule>);

    fn claim(env: Env, sender: Address, index: u64);

    fn update(env: Env, new_wash_hash: BytesN<32>);

    fn query_balance(env: Env, address: Address) -> i128;

    fn query_vesting_info(env: Env, address: Address, index: u64) -> VestingInfo;

    fn query_all_vesting_info(env: Env, address: Address) -> Vec<VestingInfo>;

    fn query_token_info(env: Env) -> VestingTokenInfo;

    fn query_vesting_contract_balance(env: Env) -> i128;

    fn query_available_to_claim(env: Env, address: Address, index: u64) -> i128;

    fn migrate_admin_key(env: Env) -> Result<(), ContractError>;
}

#[contractimpl]
impl VestingTrait for Vesting {
    fn initialize(
        env: Env,
        admin: Address,
        vesting_token: VestingTokenInfo,
        max_vesting_complexity: u32,
    ) {
        if is_initialized(&env) {
            log!(
                &env,
                "Stake: Initialize: initializing contract twice is not allowed"
            );
            panic_with_error!(&env, ContractError::AlreadyInitialized);
        }

        set_initialized(&env);

        save_admin_old(&env, &admin);

        let token_info = VestingTokenInfo {
            name: vesting_token.name,
            symbol: vesting_token.symbol,
            decimals: vesting_token.decimals,
            address: vesting_token.address,
        };

        save_token_info(&env, &token_info);
        save_max_vesting_complexity(&env, &max_vesting_complexity);

        env.events()
            .publish(("Initialize", "Vesting contract with admin: "), admin);
    }

    fn create_vesting_schedules(env: Env, vesting_schedules: Vec<VestingSchedule>) {
        let admin = get_admin_old(&env);
        admin.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        if vesting_schedules.is_empty() {
            log!(
                &env,
                "Vesting: Create vesting account: At least one vesting schedule must be provided."
            );
            panic_with_error!(env, ContractError::MissingBalance);
        }

        check_duplications(&env, vesting_schedules.clone());
        let max_vesting_complexity = get_max_vesting_complexity(&env);

        let mut total_vested_amount = 0;

        vesting_schedules.into_iter().for_each(|vesting_schedule| {
            let vested_amount = validate_vesting_schedule(&env, &vesting_schedule.curve)
                .expect("Invalid curve and amount");

            if max_vesting_complexity <= vesting_schedule.curve.size() {
                log!(
                    &env,
                    "Vesting: Create vesting account: Invalid curve complexity for {}",
                    vesting_schedule.recipient
                );
                panic_with_error!(env, ContractError::VestingComplexityTooHigh);
            }

            save_vesting(
                &env,
                &vesting_schedule.recipient.clone(),
                &(VestingInfo {
                    balance: vested_amount,
                    recipient: vesting_schedule.recipient,
                    schedule: vesting_schedule.curve.clone(),
                }),
            );

            total_vested_amount += vested_amount;
        });

        // check if the admin has enough tokens to start the vesting contract
        let vesting_token = get_token_info(&env);
        let token_client = token_contract::Client::new(&env, &vesting_token.address);

        if token_client.balance(&admin) < convert_u128_to_i128(total_vested_amount) {
            log!(
                &env,
                "Vesting: Create vesting account: Admin does not have enough tokens to start the vesting schedule"
            );
            panic_with_error!(env, ContractError::NoEnoughtTokensToStart);
        }

        token_client.transfer(
            &admin,
            &env.current_contract_address(),
            &convert_u128_to_i128(total_vested_amount),
        );
    }

    fn claim(env: Env, sender: Address, index: u64) {
        sender.require_auth();
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let available_to_claim = Self::query_available_to_claim(env.clone(), sender.clone(), index);

        if available_to_claim <= 0 {
            log!(&env, "Vesting: Claim: No tokens available to claim");
            panic_with_error!(env, ContractError::NeverFullyVested);
        }

        let token_client = token_contract::Client::new(&env, &get_token_info(&env).address);

        let vesting_info = get_vesting(&env, &sender, index);
        let vested = vesting_info.schedule.value(env.ledger().timestamp());

        let sender_balance = vesting_info.balance;
        let sender_liquid = sender_balance // this checks if we can withdraw any vesting
            .checked_sub(vested)
            .unwrap_or_else(|| panic_with_error!(env, ContractError::NotEnoughBalance));

        if sender_liquid < convert_i128_to_u128(available_to_claim) {
            log!(
                &env,
                "Vesting: Verify Vesting Update Balances: Remaining amount must be at least equal to vested amount"
            );
            panic_with_error!(env, ContractError::CantMoveVestingTokens);
        }

        update_vesting(
            &env,
            &sender,
            index,
            &(VestingInfo {
                balance: sender_balance - convert_i128_to_u128(available_to_claim),
                ..vesting_info
            }),
        );

        token_client.transfer(
            &env.current_contract_address(),
            &sender,
            &available_to_claim,
        );

        env.events()
            .publish(("Claim", "Claimed tokens: "), available_to_claim);
    }

    fn query_balance(env: Env, address: Address) -> i128 {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        token_contract::Client::new(&env, &get_token_info(&env).address).balance(&address)
    }

    fn query_vesting_info(env: Env, address: Address, index: u64) -> VestingInfo {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_vesting(&env, &address, index)
    }

    fn query_all_vesting_info(env: Env, address: Address) -> Vec<VestingInfo> {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_all_vestings(&env, &address)
    }

    fn query_token_info(env: Env) -> VestingTokenInfo {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        get_token_info(&env)
    }

    fn query_vesting_contract_balance(env: Env) -> i128 {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let token_address = get_token_info(&env).address;
        token_contract::Client::new(&env, &token_address).balance(&env.current_contract_address())
    }

    fn query_available_to_claim(env: Env, address: Address, index: u64) -> i128 {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let vesting_info = get_vesting(&env, &address, index);

        convert_u128_to_i128(
            vesting_info.balance - vesting_info.schedule.value(env.ledger().timestamp()),
        )
    }

    fn update(env: Env, new_wasm_hash: BytesN<32>) {
        let admin = get_admin_old(&env);
        admin.require_auth();

        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    fn migrate_admin_key(env: Env) -> Result<(), ContractError> {
        let admin = get_admin_old(&env);
        env.storage().instance().set(&ADMIN, &admin);

        Ok(())
    }
}
