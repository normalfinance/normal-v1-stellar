use soroban_sdk::{Address, Env, String, Symbol};

// ################################################################
//                             Market
// ################################################################
pub struct MarketEvents {}

impl MarketEvents {
    // Synth Market Events

    /// Emitted when a market is created
    ///
    /// Note: The size limit for an event is 8kB. Title and calldata must be within the limit
    /// to create the proposal.
    ///
    /// - topics - `["initialize_market", proposal_id: u32, proposer: Address]`
    /// - data - `[title: String, desc: String, action: ProposalAction, vote_start: u32, vote_end: u32]`
    pub fn initialize_market(env: &Env, market_name: String, ts: u64) {
        let topics = (Symbol::new(&env, "initialize_market"), market_name);
        env.events().publish(topics, ts);
    }

    /// Emitted when a proposal is canceled
    ///
    /// - topics - `["proposal_canceled", proposal_id: u32]`
    /// - data - ()
    pub fn shutdown(env: &Env, proposal_id: u32) {
        let topics = (Symbol::new(&env, "proposal_canceled"), proposal_id);
        env.events().publish(topics, ());
    }

    /// Emitted when a proposal is canceled
    ///
    /// - topics - `["proposal_canceled", proposal_id: u32]`
    /// - data - ()
    pub fn deletion(env: &Env, proposal_id: u32) {
        let topics = (Symbol::new(&env, "proposal_canceled"), proposal_id);
        env.events().publish(topics, ());
    }

    // Collateral Events

    /// Emitted when a user deposits collateral into a market
    ///
    /// - topics - `["collateral_deposit", market_name: String]`
    /// - data - `[user: Address, collateral_type: Address, amount: i128]`
    pub fn collateral_deposit(
        env: &Env,
        market_name: String,
        user: Address,
        collateral_type: Address,
        amount: i128,
    ) {
        let topics = (Symbol::new(&env, "collateral_deposit"), market_name);
        env.events()
            .publish(topics, (user, collateral_type, amount));
    }

    /// Emitted when collateral is lent to generate yield (either by the user or protocol)
    ///
    /// - topics - `["collateral_loan", market_name: String]`
    /// - data - `[user: Address, loan_contract: Address, collateral_type: Address, amount: u64, executor: String]`
    pub fn collateral_loan(
        env: &Env,
        market_name: String,
        user: Address,
        loan_contract: Address,
        collateral_type: Address,
        amount: u64,
        executor: String,
    ) {
        let topics = (Symbol::new(&env, "collateral_loan"), market_name);
        env.events().publish(
            topics,
            (user, loan_contract, collateral_type, amount, executor),
        );
    }

    /// Emitted when collateral is returned from loan (either by the user or protocol)
    ///
    /// - topics - `["collateral_loan_recall", market_name: String]`
    /// - data - `[user: Address, loan_contract: Address, collateral_type: Address, amount: u64, executor: String]`
    pub fn collateral_loan_recall(
        env: &Env,
        market_name: String,
        user: Address,
        loan_contract: Address,
        collateral_type: Address,
        amount: u64,
        executor: String,
    ) {
        let topics = (Symbol::new(&env, "collateral_loan_recall"), market_name);
        env.events().publish(
            topics,
            (user, loan_contract, collateral_type, amount, executor),
        );
    }

    /// Emitted when a user withdraws collateral from a market
    ///
    /// - topics - `["collateral_withdrawal", market_name: String]`
    /// - data - `[user: Address, collateral_type: Address, amount: i128]`
    pub fn collateral_withdrawal(
        env: &Env,
        market_name: String,
        user: Address,
        collateral_type: Address,
        amount: i128,
    ) {
        let topics = (Symbol::new(&env, "collateral_withdrawal"), market_name);
        env.events()
            .publish(topics, (user, collateral_type, amount));
    }

    // Keeper Events

    /// Emitted when a position is liquidated
    ///
    /// - topics - `["liquidation", market_name: u32]`
    /// - data - `[user: Address, liquidator: Address, margin_requirement: u128, total_collateral: i128, margin_freed: u64, liquidation_id: u32]`
    pub fn liquidation(
        env: &Env,
        market_name: String,
        user: Address,
        liquidator: Address,
        margin_requirement: u128,
        total_collateral: i128,
        margin_freed: u64,
        liquidation_id: u32,
    ) {
        let topics = (Symbol::new(&env, "liquidation"), market_name);
        env.events().publish(
            topics,
            (
                user,
                liquidator,
                margin_requirement,
                total_collateral,
                margin_freed,
                liquidation_id,
            ),
        );
    }

    /// Emitted when a position is bankrupt
    ///
    /// - topics - `["bankruptcy", market_name: u32]`
    /// - data - ()
    pub fn bankruptcy(
        env: &Env,
        market_name: String,
        depositor: Address,
        collateral_type: Address,
        amount: u64,
    ) {
        let topics = (Symbol::new(&env, "bankruptcy"), market_name);
        env.events().publish(topics, ());
    }
}

// ################################################################
//                             Pool
// ################################################################

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
    /// - data - `[amount_a: i128, amount_b: i128]`
    pub fn collect_fees(env: &Env, user: Address, amount_a: i128, amount_b: i128) {
        let topics = (Symbol::new(&env, "collect_fees"), user);
        env.events().publish(topics, (amount_a, amount_b));
    }
}
