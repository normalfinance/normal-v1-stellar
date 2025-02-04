use normal::types::IndexAsset;
use soroban_sdk::{Address, Env, String, Symbol, Vec};

pub struct IndexTokenEvents {}

impl IndexTokenEvents {
    /// Emitted when an index token is created
    ///
    /// Note: The size limit for an event is 8kB. Title and calldata must be within the limit
    /// to create the proposal.
    ///
    /// - topics - `["initialize", admin: Address]`
    /// - data - `[name: String, symbol: String]`
    #[allow(dead_code)]
    pub fn initialize(env: &Env, admin: Address, name: String, symbol: String) {
        let topics = (Symbol::new(env, "initialize"), admin);
        env.events().publish(topics, (name, symbol));
    }

    /// Emitted when index tokens are minted
    ///
    /// - topics - `["mint", minter: Address]`
    /// - data - `[amount: i128]`
    pub fn mint(env: &Env, minter: Address, amount: i128) {
        let topics = (Symbol::new(env, "mint"), minter);
        env.events().publish(topics, amount);
    }

    /// Emitted when index tokens are redeemed
    ///
    /// - topics - `["redeem", redeemer: Address]`
    /// - data - `[amount: i128]`
    #[allow(dead_code)]
    pub fn redeem(env: &Env, redeemer: Address, amount: i128) {
        let topics = (Symbol::new(env, "redeem"), redeemer);
        env.events().publish(topics, amount);
    }

    /// Emitted when an index is rebalanced
    ///
    /// - topics - `["rebalance", keeper: Address]`
    /// - data - `[assets: Vec<IndexAsset>`
    pub fn rebalance(env: &Env, keeper: Address, assets: Vec<IndexAsset>) {
        let topics = (Symbol::new(env, "rebalance"), keeper);
        env.events().publish(topics, assets);
    }
}
