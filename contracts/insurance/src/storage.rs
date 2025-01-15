use normal::{
    constants::{ PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD },
    error::NormalResult,
    safe_decrement,
    safe_increment,
};
use soroban_sdk::{ contracttype, symbol_short, Address, Env, Symbol, Vec };

// ################################################################

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Config,
    InsuranceFund,
    Initialized,
}

// ################################################################

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsuranceFund {
    pub total_shares: u128,
    pub user_shares: u128,
    pub shares_base: u128, // exponent for lp shares (for rebasing)
    pub last_revenue_settle_ts: u64,
    pub total_factor: u32, // percentage of interest for total insurance
    pub user_factor: u32, // percentage of interest for user staked insurance
}

impl InsuranceFund {
    pub fn is_operation_paused(&self, operation: &Operation) -> bool {
        self.paused_operations.contains(operation)
    }
}

pub fn save_config(env: &Env, config: Config) {
    env.storage().persistent().set(&DataKey::Config, &config);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Config, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

pub fn get_insurance_fund(env: &Env) -> InsuranceFund {
    let insurance_fund = env
        .storage()
        .persistent()
        .get(&DataKey::InsuranceFund)
        .expect("Config not set");

    env.storage()
        .persistent()
        .extend_ttl(&DataKey::InsuranceFund, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

    insurance_fund
}

// ################################################################

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq, contracttype)]
pub enum Operation {
    Add,
    RequestRemove,
    Remove,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub admin: Address,
    pub governor: Address,
    pub share_token: Address,
    pub stake_asset: Address,
    pub unstaking_period: i64,
    pub revenue_settle_period: i64,
    pub max_insurance: u64,
    pub paused_operations: Vec<Operation>,
}

pub fn save_config(env: &Env, config: Config) {
    env.storage().persistent().set(&DataKey::Config, &config);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Config, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

pub fn get_config(env: &Env) -> Config {
    let config = env.storage().persistent().get(&DataKey::Config).expect("Config not set");

    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Config, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

    config
}

// ################################################################

#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum StakeAction {
    #[default]
    Stake,
    UnstakeRequest,
    UnstakeCancelRequest,
    Unstake,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Stake {
    pub authority: Address,
    if_shares: u128,
    pub last_withdraw_request_shares: u128, // get zero as 0 when not in escrow
    pub if_base: u128, // exponent for if_shares decimal places (for rebase)
    pub last_valid_ts: u64,
    pub last_withdraw_request_value: u64,
    pub last_withdraw_request_ts: u64,
    pub cost_basis: i64,
}

impl Stake {
    pub fn new(authority: Address, now: u64) -> Self {
        Stake {
            authority,
            last_withdraw_request_shares: 0,
            last_withdraw_request_value: 0,
            last_withdraw_request_ts: 0,
            cost_basis: 0,
            if_base: 0,
            last_valid_ts: now,
            if_shares: 0,
        }
    }

    fn validate_base(&self, insurance_fund: &InsuranceFund) -> NormalResult {
        validate!(
            self.if_base == insurance_fund.shares_base,
            ErrorCode::InvalidIFRebase,
            "if stake bases mismatch. user base: {} market base {}",
            self.if_base,
            insurance_fund.shares_base
        )?;

        Ok(())
    }

    pub fn checked_if_shares(&self, insurance_fund: &InsuranceFund) -> NormalResult<u128> {
        self.validate_base(insurance_fund)?;
        Ok(self.if_shares)
    }

    pub fn unchecked_if_shares(&self) -> u128 {
        self.if_shares
    }

    pub fn increase_if_shares(
        &mut self,
        delta: u128,
        insurance_fund: &InsuranceFund
    ) -> NormalResult {
        self.validate_base(insurance_fund)?;
        safe_increment!(self.if_shares, delta);
        Ok(())
    }

    pub fn decrease_if_shares(
        &mut self,
        delta: u128,
        insurance_fund: &InsuranceFund
    ) -> NormalResult {
        self.validate_base(insurance_fund)?;
        safe_decrement!(self.if_shares, delta);
        Ok(())
    }

    pub fn update_if_shares(
        &mut self,
        new_shares: u128,
        insurance_fund: &InsuranceFund
    ) -> NormalResult {
        self.validate_base(insurance_fund)?;
        self.if_shares = new_shares;

        Ok(())
    }
}

pub fn get_stake(env: &Env, key: &Address) -> Stake {
    let stake_info = match env.storage().persistent().get::<_, Stake>(key) {
        Some(stake) => stake,
        None => Stake::new(key, env.ledger().timestamp()),
    };
    env.storage()
        .persistent()
        .has(&key)
        .then(|| {
            env.storage()
                .persistent()
                .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        });

    stake_info
}

pub fn save_stake(env: &Env, key: &Address, stake_info: &Stake) {
    env.storage().persistent().set(key, stake_info);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

// ################################################################

// Governor

// pub fn is_governor(e: &Env) {
//     if e.invoker() != get_governor(e) {
//         return Err(ErrorCode::OnlyGovernor);
//     }
//     // TODO: do we need to auth the governor?
//     // governor.require_auth();
// }

pub mod utils {
    use soroban_sdk::Bytes;

    use crate::token_contract;

    use super::*;

    pub fn is_initialized(env: &Env) -> bool {
        env.storage().persistent().get(&DataKey::Initialized).unwrap_or(false)
    }

    pub fn set_initialized(env: &Env) {
        env.storage().persistent().set(&DataKey::Initialized, &true);

        env.storage()
            .persistent()
            .extend_ttl(
                &DataKey::Initialized,
                PERSISTENT_LIFETIME_THRESHOLD,
                PERSISTENT_BUMP_AMOUNT
            );
    }

    pub fn is_admin(env: &Env) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn deploy_token_contract(
        env: &Env,
        token_wasm_hash: BytesN<32>,
        governor: &Address,
        admin: Address,
        decimals: u32,
        name: String,
        symbol: String
    ) -> Address {
        let mut salt = Bytes::new(env);
        salt.append(&governor.clone().to_xdr(env));
        let salt = env.crypto().sha256(&salt);
        env.deployer()
            .with_current_contract(salt)
            .deploy_v2(token_wasm_hash, (admin, decimals, name, symbol))
    }

    pub fn mint_shares(env: &Env, share_token: &Address, to: &Address, amount: i128) {
        let total = get_total_shares(env);

        token_contract::Client::new(env, share_token).mint(to, &amount);

        save_total_shares(env, total + amount);
    }

    pub fn burn_shares(e: &Env, share_token: &Address, amount: i128) {
        let total = get_total_shares(env);

        token_contract::Client
            ::new(env, share_token)
            .burn(&env.current_contract_address(), &amount);

        save_total_shares(e, total - amount);
    }
}
