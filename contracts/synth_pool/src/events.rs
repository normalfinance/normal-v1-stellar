use soroban_sdk::{Address, Env, Symbol};

pub struct PoolEvents {}

impl PoolEvents {
    /// Emitted when an AMM is created
    ///
    /// Note: The size limit for an event is 8kB. Title and calldata must be within the limit
    /// to create the proposal.
    ///
    /// - topics - `["init", token_a: Address, token_b: Address, tick_spacing: u32]`
    /// - data - `[initial_sqrt_price: u128, fee_rate: u32, protocol_fee_rate: u32]`
    pub fn init(
        env: &Env,
        token_a: Address,
        token_b: Address,
        tick_spacing: u32,
        initial_sqrt_price: u128,
        fee_rate: u32,
        protocol_fee_rate: u32,
    ) {
        let topics = (Symbol::new(&env, "init"), token_a, token_b, tick_spacing);
        env.events()
            .publish(topics, (initial_sqrt_price, fee_rate, protocol_fee_rate));
    }

    /// Emitted when a user swaps with the AMM
    ///
    /// - topics - `["swap", amm_id: u32]`
    /// - data - `[to: Address, buy_a: bool, out: i128, in_max: i128]`
    pub fn swap(
        env: &Env,
        user: Address,
        token_a: Address,
        token_b: Address,
        buy_a: bool,
        out: i128,
        in_max: i128,
    ) {
        let topics = (Symbol::new(&env, "swap"), user, token_a, token_b);
        env.events().publish(topics, (buy_a, out, in_max));
    }

    // Liquidity Provider (LP) Events

    /// Emitted when a user adds liquidity to an AMM
    ///
    /// - topics - `["increase_liquidity", amm_id: u32]`
    /// - data - `[to: Address, amount_a: i128, amount_b: i128]`
    pub fn increase_liquidity(
        env: &Env,
        user: Address,
        token_a: Address,
        token_b: Address,
        amount_a: i128,
        amount_b: i128,
    ) {
        let topics = (
            Symbol::new(&env, "increase_liquidity"),
            user,
            token_a,
            token_b,
        );
        env.events().publish(topics, (amount_a, amount_b));
    }

    /// Emitted when a user removes liquidity from an AMM
    ///
    /// - topics - `["remove_liquidity", amm_id: u32]`
    /// - data - `[to: Address, amount_a: i128, amount_b: i128]`
    pub fn remove_liquidity(
        env: &Env,
        user: Address,
        token_a: Address,
        token_b: Address,
        amount_a: i128,
        amount_b: i128,
    ) {
        let topics = (
            Symbol::new(&env, "remove_liquidity"),
            user,
            token_a,
            token_b,
        );
        env.events().publish(topics, (amount_a, amount_b));
    }

    /// Emitted when a user removes part/all of their stake in the Insurance Fund
    ///
    /// - topics - `["collect_fees", user: u32]`
    /// - data - `[asset: Address, amount: i128]`
    pub fn collect_fees(env: &Env, user: Address, asset: Address, amount: i128) {
        let topics = (Symbol::new(&env, "collect_fees"), user);
        env.events().publish(topics, (asset, amount));
    }
}
