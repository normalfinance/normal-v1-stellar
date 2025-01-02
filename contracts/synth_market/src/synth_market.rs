use soroban_sdk::{ contractclient, Address, Env, String };

#[contractclient(name = "SynthMarketClient")]
pub trait SynthMarket {
    /// Setup the governor contract
    ///
    /// ### Arguments
    /// * `votes` - The address of the contract used to track votes
    /// * `council` - The address of the security council for the DAO
    /// * `settings` - The settings for the governor
    fn initialize(e: Env, votes: Address, council: Address, settings: GovernorSettings);

    pub fn 

    pub fn set_fee_rate(e: Env, fee_rate: u128) -> u128 {}

    pub fn freeze_oracle(e: Env) -> u128 {}

    pub fn init_shutdown(e: Env) -> u128 {}

    pub fn update_debt_floor(e: Env) -> u128 {}

    pub fn update_debt_ceiling(e: Env) -> u128 {}

    pub fn liquidate(e: Env, fee_rate: u128) -> u128 {}

    pub fn liquidate(e: Env, fee_rate: u128) -> u128 {}
}
