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
    pub fn initialize(e: &Env, admin: Address, name: String, symbol: String) {
        let topics = (Symbol::new(&e, "initialize"), admin);
        e.events().publish(topics, (name, symbol));
    }

    /// Emitted when index tokens are minted
    ///
    /// - topics - `["mint", minter: Address]`
    /// - data - `[amount: i128, to: Address]`
    pub fn mint(e: &Env, minter: Address, amount: i128, to: Address) {
        let topics = (Symbol::new(&e, "mint"), minter);
        e.events().publish(topics, (amount, to));
    }

    /// Emitted when index tokens are redeemed
    ///
    /// - topics - `["redeem", redeemer: Address]`
    /// - data - `[amount: i128]`
    pub fn redeem(e: &Env, redeemer: Address, amount: i128) {
        let topics = (Symbol::new(&e, "redeem"), redeemer);
        e.events().publish(topics, amount);
    }

    /// Emitted when an index is rebalanced
    ///
    /// - topics - `["rebalance", keeper: Address]`
    /// - data - `[assets: Vec<IndexAsset>`
    pub fn rebalance(e: &Env, keeper: Address, assets: Vec<IndexAsset>) {
        let topics = (Symbol::new(&e, "rebalance"), keeper);
        e.events().publish(topics, assets);
    }
}
