use soroban_sdk::{contractclient, Address, Env, String};

use crate::types::{GovernorSettings, Proposal, ProposalAction, VoteCount};

#[contractclient(name = "RewardsClient")]
pub trait RewardsTrait {
    fn claim_revenue_reward(e: Env, sender: Address);
}
