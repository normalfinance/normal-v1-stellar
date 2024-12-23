use soroban_sdk::{ Address, Env, String, Symbol };

pub struct AMMEvents {}

impl AMMEvents {
    // AMM Events

    /// Emitted when an AMM is created
    ///
    /// Note: The size limit for an event is 8kB. Title and calldata must be within the limit
    /// to create the proposal.
    ///
    /// - topics - `["init", token_a: Address, token_b: Address, tick_spacing: u16]`
    /// - data - `[initial_sqrt_price: u128, fee_rate: u16, protocol_fee_rate: u16]`
    pub fn init(
        e: &Env,
        token_a: Address,
        token_b: Address,
        tick_spacing: u16,
        initial_sqrt_price: u128,
        fee_rate: u16,
        protocol_fee_rate: u16
    ) {
        let topics = (Symbol::new(&e, "init"), token_a, token_b, tick_spacing);
        e.events().publish(topics, (initial_sqrt_price, fee_rate, protocol_fee_rate));
    }

    /// Emitted when a user swaps with the AMM
    ///
    /// - topics - `["swap", amm_id: u32]`
    /// - data - `[to: Address, buy_a: bool, out: i128, in_max: i128]`
    pub fn swap(e: &Env, amm_id: u32, to: Address, buy_a: bool, out: i128, in_max: i128) {
        let topics = (Symbol::new(&e, "swap"), amm_id);
        e.events().publish(topics, to, buy_a, out, in_max);
    }

    // Liquidity Provider (LP) Events

    /// Emitted when a user adds liquidity to an AMM
    ///
    /// - topics - `["add_liquidity", amm_id: u32]`
    /// - data - `[to: Address, amount_a: i128, amount_b: i128]`
    pub fn add_liquidity(e: &Env, amm_id: u64, to: Address, amount_a: i128, amount_b: i128) {
        let topics = (Symbol::new(&e, "add_liquidity"), amm_id);
        e.events().publish(topics, (amount_a, amount_b));
    }

    /// Emitted when a user removes liquidity from an AMM
    ///
    /// - topics - `["remove_liquidity", amm_id: u32]`
    /// - data - `[to: Address, amount_a: i128, amount_b: i128]`
    pub fn remove_liquidity(e: &Env, amm_id: u64, to: Address, amount_a: i128, amount_b: i128) {
        let topics = (Symbol::new(&e, "remove_liquidity"), amm_id);
        e.events().publish(topics, (amount_a, amount_b));
    }

    /// Emitted when a user removes part/all of their stake in the Insurance Fund
    ///
    /// - topics - `["collect_fees", user: u32]`
    /// - data - `[asset: Address, amount: i128]`
    pub fn collect_fees(e: &Env, user: Address, asset: Address, amount: i128) {
        let topics = (Symbol::new(&e, "collect_fees"), user);
        e.events().publish(topics, (asset, amount));
    }
}
