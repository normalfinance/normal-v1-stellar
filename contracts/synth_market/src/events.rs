use soroban_sdk::{ Address, Env, String, Symbol };

pub struct SynthMarketEvents {}

impl SynthMarketEvents {
    // Synth Market Events

    /// Emitted when a market is created
    ///
    /// Note: The size limit for an event is 8kB. Title and calldata must be within the limit
    /// to create the proposal.
    ///
    /// - topics - `["market_initialization", proposal_id: u32, proposer: Address]`
    /// - data - `[title: String, desc: String, action: ProposalAction, vote_start: u32, vote_end: u32]`
    pub fn initialization(
        e: &Env,
       
    ) {
        let topics = (Symbol::new(&e, "market_initialization"), proposal_id, proposer);
        e.events().publish(topics, (title, desc, action, vote_start, vote_end));
    }

    /// Emitted when a proposal is canceled
    ///
    /// - topics - `["proposal_canceled", proposal_id: u32]`
    /// - data - ()
    pub fn shutdown(e: &Env, proposal_id: u32) {
        let topics = (Symbol::new(&e, "proposal_canceled"), proposal_id);
        e.events().publish(topics, ());
    }

    /// Emitted when a proposal is canceled
    ///
    /// - topics - `["proposal_canceled", proposal_id: u32]`
    /// - data - ()
    pub fn deletion(e: &Env, proposal_id: u32) {
        let topics = (Symbol::new(&e, "proposal_canceled"), proposal_id);
        e.events().publish(topics, ());
    }

    // Collateral Events

    /// Emitted when a user deposits collateral into a market
    ///
    /// - topics - `["collateral_deposit", market_id: u32]`
    /// - data - `[depositor: Address, collateral_type: Address, amount: u64]`
    pub fn collateral_deposit(
        e: &Env,
        market: SynthMarket,
        depositor: Address,
        collateral_type: Address,
        amount: u64
    ) {
        let topics = (Symbol::new(&e, "collateral_deposit"), market_id);
        e.events().publish(topics, (depositor, collateral_type, amount));
    }

    /// Emitted when collateral is lent to generate yield (either by the user or protocol)
    ///
    /// - topics - `["collateral_loan", market_id: u32]`
    /// - data - `[user: Address, loan_contract: Address, collateral_type: Address, amount: u64, executor: String]`
    pub fn collateral_loan(
        e: &Env,
        market: SynthMarket,
        user: Address,
        loan_contract: Address,
        collateral_type: Address,
        amount: u64,
        executor: String
    ) {
        let topics = (Symbol::new(&e, "collateral_loan"), market_id);
        e.events().publish(topics, (user, loan_contract, collateral_type, amount, executor));
    }

    /// Emitted when collateral is returned from loan (either by the user or protocol)
    ///
    /// - topics - `["collateral_loan_recall", market_id: u32]`
    /// - data - `[user: Address, loan_contract: Address, collateral_type: Address, amount: u64, executor: String]`
    pub fn collateral_loan_recall(
        e: &Env,
        market: SynthMarket,
        user: Address,
        loan_contract: Address,
        collateral_type: Address,
        amount: u64,
        executor: String
    ) {
        let topics = (Symbol::new(&e, "collateral_loan_recall"), market_id);
        e.events().publish(topics, (user, loan_contract, collateral_type, amount, executor));
    }

    /// Emitted when a user transfers collateral to another user
    ///
    /// - topics - `["collateral_transfer", market_id: u32]`
    /// - data - `[from: Address, to: Address, collateral_type: Address, amount: u64]`
    pub fn collateral_transfer(
        e: &Env,
        market: SynthMarket,
        from: Address,
        to: Address,
        collateral_type: Address,
        amount: u64
    ) {
        let topics = (Symbol::new(&e, "collateral_transfer"), market_id);
        e.events().publish(topics, (from, to, collateral_type, amount));
    }

    /// Emitted when a user withdraws collateral from a market
    ///
    /// - topics - `["collateral_withdrawal", market_id: u32]`
    /// - data - `[withdrawer: Address, collateral_type: Address, amount: u64]`
    pub fn collateral_withdrawal(
        e: &Env,
        market: SynthMarket,
        withdrawer: Address,
        collateral_type: Address,
        amount: u64
    ) {
        let topics = (Symbol::new(&e, "collateral_withdrawal"), market_id);
        e.events().publish(topics, (withdrawer, collateral_type, amount));
    }

    // Keeper Events

    /// Emitted when a position is liquidated
    ///
    /// - topics - `["liquidation", market_id: u32]`
    /// - data - `[user: Address, liquidator: Address, margin_requirement: u128, total_collateral: i128, margin_freed: u64, liquidation_id: u16]`
    pub fn liquidation(
        e: &Env,
        market: SynthMarket,
        user: Address,
        liquidator: Address,
        margin_requirement: u128,
        total_collateral: i128,
        margin_freed: u64,
        liquidation_id: u16
    ) {
        let topics = (Symbol::new(&e, "liquidation"), market_id);
        e.events().publish(topics, (
            user,
            liquidator,
            margin_requirement,
            total_collateral,
            margin_freed,
            liquidation_id,
        ));
    }

    /// Emitted when a position is bankrupt
    ///
    /// - topics - `["bankruptcy", market_id: u32]`
    /// - data - ()
    pub fn bankruptcy(
        e: &Env,
        market: SynthMarket,
        depositor: Address,
        collateral_type: Address,
        amount: u64
    ) {
        let topics = (Symbol::new(&e, "bankruptcy"), market_id);
        e.events().publish(topics, ());
    }
}
