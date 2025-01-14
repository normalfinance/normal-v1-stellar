use normal::{
    constants::{ PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD },
    error::NormalResult,
    safe_decrement,
    safe_increment,
};
use soroban_sdk::{ contracttype, symbol_short, Address, Env, Symbol };

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsuranceFund {
    pub total_shares: u128,
    pub user_shares: u128,
    pub shares_base: u128, // exponent for lp shares (for rebasing)
    pub last_revenue_settle_ts: i64,
    pub total_factor: u32, // percentage of interest for total insurance
    pub user_factor: u32, // percentage of interest for user staked insurance
    pub paused_operations: u8,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub unstaking_period: i64,
    pub revenue_settle_period: i64,
    pub max_insurance: u64,
    pub paused_operations: u8,
}
const CONFIG: Symbol = symbol_short!("CONFIG");

pub fn get_config(env: &Env) -> Config {
    let config = env.storage().persistent().get(&CONFIG).expect("Stake: Config not set");
    env.storage()
        .persistent()
        .extend_ttl(&CONFIG, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

    config
}

pub fn save_config(env: &Env, config: Config) {
    env.storage().persistent().set(&CONFIG, &config);
    env.storage()
        .persistent()
        .extend_ttl(&CONFIG, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

// ################################################################

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Stake {
    pub authority: Address,
    if_shares: u128,
    pub last_withdraw_request_shares: u128, // get zero as 0 when not in escrow
    pub if_base: u128, // exponent for if_shares decimal places (for rebase)
    pub last_valid_ts: i64,
    pub last_withdraw_request_value: u64,
    pub last_withdraw_request_ts: i64,
    pub cost_basis: i64,
}

impl Stake {
    pub fn new(authority: Address, now: i64) -> Self {
        InsuranceFundStake {
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

// ################################################################

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,

    Stake(Address),
}

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq, contracttype)]
pub enum Operation {
    Stake,
    Unstake,
    Transfer,
}

#[derive(Clone)]
#[contracttype]
pub struct Stake {
    pub authority: Address,
    if_shares: u128,
    pub last_withdraw_request_shares: u128, // get zero as 0 when not in escrow
    pub if_base: u128, // exponent for if_shares decimal places (for rebase)
    pub last_valid_ts: i64,
    pub last_withdraw_request_value: u64,
    pub last_withdraw_request_ts: i64,
    pub cost_basis: i64,
}

// ################################################################

#[contracttype]
#[derive(Default, Eq, PartialEq, Debug)]
#[repr(C)]
pub struct ProtocolIfSharesTransferConfig {
    pub whitelisted_signers: [Pubkey; 4],
    pub max_transfer_per_epoch: u128,
    pub current_epoch_transfer: u128,
    pub next_epoch_ts: i64,
    pub padding: [u128; 8],
}

impl ProtocolIfSharesTransferConfig {
    pub fn validate_signer(&self, signer: &Pubkey) -> DriftResult {
        validate!(
            self.whitelisted_signers.contains(signer) && *signer != Pubkey::default(),
            ErrorCode::DefaultError,
            "signer {} not whitelisted",
            signer
        )?;

        Ok(())
    }

    pub fn update_epoch(&mut self, now: i64) -> DriftResult {
        if now > self.next_epoch_ts {
            let n_epoch_durations = now
                .safe_sub(self.next_epoch_ts)?
                .safe_div(EPOCH_DURATION)?
                .safe_add(1)?;

            self.next_epoch_ts = self.next_epoch_ts.safe_add(
                EPOCH_DURATION.safe_mul(n_epoch_durations)?
            )?;

            self.current_epoch_transfer = 0;
        }

        Ok(())
    }

    pub fn validate_transfer(&self, requested_transfer: u128) -> DriftResult {
        let max_transfer = self.max_transfer_per_epoch.saturating_sub(self.current_epoch_transfer);

        validate!(
            requested_transfer < max_transfer,
            ErrorCode::DefaultError,
            "requested transfer {} exceeds max transfer {}",
            requested_transfer,
            max_transfer
        )?;

        Ok(())
    }
}

// ################################################################

// Governor

pub fn is_governor(e: &Env) {
    if e.invoker() != get_governor(e) {
        return Err(ErrorCode::OnlyGovernor);
    }
    // TODO: do we need to auth the governor?
    // governor.require_auth();
}

pub fn set_governor(e: &Env, governor: Address) {
    e.storage().instance().set(&DataKey::Governor, &governor);
}

pub fn get_governor(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Governor).unwrap()
}

// Admin

pub fn set_admin(e: &Env, admin: Address) {
    e.storage().instance().set(&DataKey::Admin, &admin);
}

pub fn get_admin(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Admin).unwrap()
}

pub fn is_admin(e: &Env) {
    let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();
}

// Stake

// pub fn get_stake_by_address(e: &Env, authority: Address) -> Option<Stake> {
//     e.storage().instance().get(&DataKey::Stake(authority))
// }

// Max Insurance
pub fn set_max_insurance(e: &Env, max_insurance: u64) {
    e.storage().instance().set(&DataKey::MaxInsurance, &max_insurance);
}

pub fn get_max_insurance(e: &Env) -> u64 {
    e.storage().instance().get(&DataKey::MaxInsurance).unwrap()
}

// Unstaking period

pub fn set_unstaking_period(e: &Env, unstaking_period: i64) {
    e.storage().instance().set(&DataKey::UnstakingPeriod, &unstaking_period);
}

pub fn get_unstaking_period(e: &Env) -> i64 {
    e.storage().instance().get(&DataKey::UnstakingPeriod).unwrap()
}

// Paused operations

pub fn set_paused_operations(e: &Env, paused_operations: Vec<Operation>) {
    e.storage().instance().set(&DataKey::PausedOperations, &paused_operations);
}

pub fn get_paused_operations(e: &Env) -> Vec<Operation> {
    e.storage()
        .get::<Vec<PausedOperation>>(&DataKey::PausedOperations)
        .unwrap_or_else(|| Vec::new(env));
}

pub fn is_operation_paused(e: &Env, operation: &Operation) -> bool {
    let paused_operations = get_paused_operations(e);
    paused_operations.contains(operation)
}
