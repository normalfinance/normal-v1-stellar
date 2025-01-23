use soroban_sdk::{ Address, Env, Symbol };

use crate::storage::StakeAction;

pub struct InsuranceFundEvents {}

impl InsuranceFundEvents {
    // Insurance Fund Events

    /// Emitted when a proposal is created
    ///
    /// Note: The size limit for an event is 8kB. Title and calldata must be within the limit
    /// to create the proposal.
    ///
    /// - topics - `["initialization", proposal_id: u32, proposer: Address]`
    /// - data - `[title: String, desc: String, action: ProposalAction, vote_start: u32, vote_end: u32]`
    pub fn initialization(
        env: &Env,
        ts: u64,
        admin: Address,
        governor: Address,
        share_token_address: Address
    ) {
        let topics = (Symbol::new(&env, "initialization"), admin, governor);
        env.events().publish(topics, (ts, share_token_address));
    }

    // Insurance Stake Events

    /// Emitted when a user updates their stake in the Insurance Fund
    ///
    /// - topics - `["insurance_fund_stake_record", user: Address]`
    /// - data - `[ts: u64, user: Address, action: StakeAction, amount: i128, insurance_vault_amount_before: u64, if_shares_before: u128, user_if_shares_before: u128, total_if_shares_before: u128, if_shares_after: u128, total_if_shares_after: u128, user_if_shares_after: u128]`
    pub fn insurance_fund_stake_record(
        env: &Env,
        ts: u64,
        user: Address,
        action: StakeAction,
        amount: i128,
        insurance_vault_amount_before: u64,
        if_shares_before: u128,
        user_if_shares_before: u128,
        total_if_shares_before: u128,
        if_shares_after: u128,
        total_if_shares_after: u128,
        user_if_shares_after: u128
    ) {
        let topics = (Symbol::new(&env, "insurance_fund_stake_record"), user);
        env.events().publish(topics, (
            ts,
            amount,
            insurance_vault_amount_before,
            if_shares_before,
            user_if_shares_before,
            total_if_shares_before,
            if_shares_after,
            total_if_shares_after,
            user_if_shares_after,
        ));
    }
}

pub struct BufferEvents {}
