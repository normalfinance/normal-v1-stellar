use soroban_sdk::{ Address, Env };

pub trait InsuranceFundTrait {
    fn initialize(
        env: Env,
        admin: Address,
        max_insurance: u64,
        unstaking_period: i64,
        paused_operations: u32
    );

    fn stake(env: Env, sender: Address, amount: i128);

    fn unstake(env: Env, sender: Address, amount: i128, stake_timestamp: u64);

    fn withdraw_rewards(env: Env, sender: Address);

    // QUERIES

    fn query_config(env: Env) -> ConfigResponse;

    fn query_admin(env: Env) -> Address;

    fn query_staked(env: Env, address: Address) -> StakedResponse;

    fn query_total_staked(env: Env) -> i128;
}
