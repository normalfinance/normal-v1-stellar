use normal::{
    oracle::OracleGuardRails,
    types::market::{MarketFactoryConfig, MarketParams},
};
use soroban_sdk::{contractclient, Address, BytesN, Env, String, Vec};

#[contractclient(name = "StakingClient")]
pub trait StakingTrait {
    #[allow(clippy::too_many_arguments)]
    fn initialize(env: Env, admin: Address, governor: Address, emission_token: Address);

    fn update_market_emissions(
        env: Env,
        sender: Address,
        lp_token: Address,
        amount: i128,
        deadline: u64,
    );

    // ################################################################
    //                             Users
    // ################################################################

    fn lock(env: Env, sender: Address, tokens: i128);

    fn unlock(env: Env, sender: Address, stake_amount: i128, stake_timestamp: u64);

    fn withdraw_rewards(env: Env, sender: Address);

    // ################################################################
    //                             Queries
    // ################################################################

    fn query_config(env: Env) -> ConfigResponse;

    fn query_admin(env: Env) -> Address;

    fn query_staked(env: Env, address: Address) -> StakedResponse;
}
