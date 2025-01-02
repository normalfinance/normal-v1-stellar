use soroban_sdk::{ Address, Env, String, Symbol };

pub struct IndexEvents {}

impl IndexEvents {
    /// Emitted when an index is created
    ///
    /// Note: The size limit for an event is 8kB. Title and calldata must be within the limit
    /// to create the proposal.
    ///
    /// - topics - `["index_created", creator: Address, index_id: u32]`
    /// - data - `[title: String, desc: String, action: ProposalAction, vote_start: u32, vote_end: u32]`
    pub fn index_created(e: &Env, creator: Address, index_id: u32, name: String, symbol: String) {
        let topics = (Symbol::new(&e, "index_created"), creator, index_id);
        e.events().publish(topics, (name, symbol));
    }

    /// Emitted when index tokens are minted (an investment)
    ///
    /// - topics - `["index_minted", index_id: u32]`
    /// - data - `[minter: Address, amount: u64]`
    pub fn index_minted(e: &Env, index_id: u32, minter: Address, amount: u64) {
        let topics = (Symbol::new(&e, "index_minted"), index_id);
        e.events().publish(topics, (minter, amount));
    }

    /// Emitted when index tokens are redeemed (a withdrawal)
    ///
    /// - topics - `["index_redeemed", index_id: u32]`
    /// - data - `[redeemer: Address, amount: u64]`
    pub fn index_redeemed(e: &Env, index_id: u32, redeemer: Address, amount: u64) {
        let topics = (Symbol::new(&e, "index_redeemed"), index_id);
        e.events().publish(topics, (redeemer, amount));
    }

    /// Emitted when index tokens are redeemed (a withdrawal)
    ///
    /// - topics - `["index_redeemed", index_id: u32]`
    /// - data - `[redeemer: Address, amount: u64]`
    pub fn index_updated(e: &Env) {
        let topics = (Symbol::new(&e, "index_updated"), proposal_id, proposer);
        e.events().publish(topics, (title, desc, action, vote_start, vote_end));
    }

    /// Emitted when index tokens are redeemed (a withdrawal)
    ///
    /// - topics - `["index_redeemed", index_id: u32]`
    /// - data - `[redeemer: Address, amount: u64]`
    pub fn index_deleted(e: &Env) {
        let topics = (Symbol::new(&e, "index_created"), proposal_id, proposer);
        e.events().publish(topics, (title, desc, action, vote_start, vote_end));
    }
}
