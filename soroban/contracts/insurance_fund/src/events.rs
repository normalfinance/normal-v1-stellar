use soroban_sdk::{ Address, Env, String, Symbol };

pub struct InsuranceEvents {}

impl InsuranceEvents {
    // Insurance Fund Events

    /// Emitted when a proposal is created
    ///
    /// Note: The size limit for an event is 8kB. Title and calldata must be within the limit
    /// to create the proposal.
    ///
    /// - topics - `["insurance_fund_initialization", proposal_id: u32, proposer: Address]`
    /// - data - `[title: String, desc: String, action: ProposalAction, vote_start: u32, vote_end: u32]`
    pub fn insurance_fund_initialization(
        e: &Env,
        proposal_id: u32,
        proposer: Address,
        title: String,
        desc: String,
        action: ProposalAction,
        vote_start: u32,
        vote_end: u32
    ) {
        let topics = (Symbol::new(&e, "insurance_fund_initialization"), proposal_id, proposer);
        e.events().publish(topics, (title, desc, action, vote_start, vote_end));
    }

    // TODO: claim?

    // Insurance Staker Events

    /// Emitted when a user stakes into the Insurance Fund
    ///
    /// - topics - `["stake", user: u32]`
    /// - data - `[asset: Address, amount: i128]`
    pub fn stake(e: &Env, user: Address, asset: Address, amount: i128) {
        let topics = (Symbol::new(&e, "stake"), user);
        e.events().publish(topics, (asset, amount));
    }

    /// Emitted when a user removes part/all of their stake in the Insurance Fund
    ///
    /// - topics - `["unstake", user: u32]`
    /// - data - `[asset: Address, amount: i128]`
    pub fn unstake(e: &Env, user: Address, asset: Address, amount: i128) {
        let topics = (Symbol::new(&e, "unstake"), user);
        e.events().publish(topics, (asset, amount));
    }
}
