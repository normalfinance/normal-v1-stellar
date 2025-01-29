use normal::{
    constants::{ PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD },
    error::{ ErrorCode, NormalResult },
    safe_decrement,
    safe_increment,
    types::OrderDirection,
    validate,
};
use soroban_sdk::{ contracttype, Address, Env, Vec, log };

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    InsuranceFund = 1,
    Buffer = 2,
    Admin = 3,
    Governor = 4,
    Initialized = 5,
}

// ################################################################
//                         Insurance Fund
// ################################################################

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum InsuranceFundOperation {
    Add,
    RequestRemove,
    Remove,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsuranceFund {
    pub stake_asset: Address,
    pub share_token: Address,
    pub unstaking_period: i64,
    pub revenue_settle_period: i64,
    pub max_insurance: u64,
    pub paused_operations: Vec<InsuranceFundOperation>,
    pub total_shares: u128,
    pub user_shares: u128,
    pub shares_base: u128, // exponent for lp shares (for rebasing)
    pub last_revenue_settle_ts: u64,
    pub total_factor: u32, // percentage of interest for total insurance
    pub user_factor: u32, // percentage of interest for user staked insurance
}

impl InsuranceFund {
    pub fn is_operation_paused(&self, operation: &InsuranceFundOperation) -> bool {
        self.paused_operations.contains(operation)
    }
}

pub fn save_insurance_fund(env: &Env, insurance_fund: InsuranceFund) {
    env.storage().persistent().set(&DataKey::InsuranceFund, &insurance_fund);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::InsuranceFund, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

pub fn get_insurance_fund(env: &Env) -> InsuranceFund {
    let insurance_fund = env
        .storage()
        .persistent()
        .get(&DataKey::InsuranceFund)
        .expect("Insurance Fund not set");

    env.storage()
        .persistent()
        .extend_ttl(&DataKey::InsuranceFund, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

    insurance_fund
}

// ################################################################
//                             Auction
// ################################################################

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum AuctionType {
    /// selling collateral from a liquidation
    Collateral,
    /// selling newly minted NORM to cover Protocol Debt (the deficit from Collateral Auctions)
    Debt,
    /// selling excess synthetic token proceeds over the Insurance Fund max limit for NORM to be burned
    Surplus,
}

/**
 * Native auctions:
 * - set a balance available for purchase
 * - set a price and bidding config
 * - users can use functions to bid and purchase on the auction
 * - contract simply updates properties as purchases occur
 */

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum AuctionLocation {
    /// Sell the asset directly to users via Normal interface
    Native,
    /// Sell the asset via a 3rd-party DEX
    External,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Auction {
    pub amount: i128,
    pub direction: OrderDirection,
    pub location: AuctionLocation,
    pub duration: u64,
    pub start_ts: u64,
    pub total_auctioned: i128,
    pub start_price: u64,
    pub end_price: u64,
}

// ################################################################
//                             Buffer
// ################################################################

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Buffer {
    pub gov_token: Address,
    pub gov_token_pool: Address, // DEX pool - Aquarius pool router: CBQDHNBFBZYE4MKPWBSJOPIYLW4SFSXAXUTSXJN76GNKYVYPCKWC6QUK
    pub quote_token: Address,
    // Auction
    pub auctions: Vec<Auction>,
    pub min_auction_duration: u64,
    // other
    pub max_balance: i128,
    pub total_burns: i128,
    pub total_mints: i128,
}

impl Buffer {}

pub fn save_buffer(env: &Env, buffer: Buffer) {
    env.storage().persistent().set(&DataKey::Buffer, &buffer);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Buffer, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

pub fn get_buffer(env: &Env) -> Buffer {
    let buffer = env.storage().persistent().get(&DataKey::Buffer).expect("Buffer not set");

    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Buffer, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

    buffer
}

// ################################################################
//                             Stake
// ################################################################

#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StakeAction {
    Stake,
    UnstakeRequest,
    UnstakeCancelRequest,
    Unstake,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stake {
    // pub authority: Address,
    if_shares: u128,
    pub last_withdraw_request_shares: u128, // get zero as 0 when not in escrow
    pub if_base: u128, // exponent for if_shares decimal places (for rebase)
    pub last_valid_ts: u64,
    pub last_withdraw_request_value: i128,
    pub last_withdraw_request_ts: u64,
    pub cost_basis: i64,
}

impl Stake {
    pub fn new(now: u64) -> Self {
        Stake {
            // authority,
            last_withdraw_request_shares: 0,
            last_withdraw_request_value: 0,
            last_withdraw_request_ts: 0,
            cost_basis: 0,
            if_base: 0,
            last_valid_ts: now,
            if_shares: 0,
        }
    }

    fn validate_base(&self, env: &Env, insurance_fund: &InsuranceFund) -> NormalResult {
        validate!(
            env,
            self.if_base == insurance_fund.shares_base,
            ErrorCode::InvalidIFRebase,
            "if stake bases mismatch. user base: {} market base {}",
            self.if_base,
            insurance_fund.shares_base
        )?;

        Ok(())
    }

    pub fn checked_if_shares(
        &self,
        env: &Env,
        insurance_fund: &InsuranceFund
    ) -> NormalResult<u128> {
        self.validate_base(env, insurance_fund)?;
        Ok(self.if_shares)
    }

    pub fn unchecked_if_shares(&self) -> u128 {
        self.if_shares
    }

    pub fn increase_if_shares(
        &mut self,
        env: &Env,
        delta: u128,
        insurance_fund: &InsuranceFund
    ) -> NormalResult {
        self.validate_base(env, insurance_fund)?;
        safe_increment!(self.if_shares, delta);
        Ok(())
    }

    pub fn decrease_if_shares(
        &mut self,
        env: &Env,
        delta: u128,
        insurance_fund: &InsuranceFund
    ) -> NormalResult {
        self.validate_base(env, insurance_fund)?;
        safe_decrement!(self.if_shares, delta);
        Ok(())
    }

    pub fn update_if_shares(
        &mut self,
        env: &Env,
        new_shares: u128,
        insurance_fund: &InsuranceFund
    ) -> NormalResult {
        self.validate_base(env, insurance_fund)?;
        self.if_shares = new_shares;

        Ok(())
    }
}

pub fn get_stake(env: &Env, key: &Address) -> Stake {
    let stake_info = match env.storage().persistent().get::<_, Stake>(key) {
        Some(stake) => stake,
        None => Stake::new(env.ledger().timestamp()),
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
//                             Utils
// ################################################################

pub mod utils {
    use normal::error::ErrorCode;
    use soroban_sdk::{ log, panic_with_error, xdr::ToXdr, Bytes, BytesN, String };

    use crate::token_contract;

    use super::*;

    pub fn transfer_token(env: &Env, asset: &Address, from: &Address, to: &Address, amount: i128) {
        let token_client = token_contract::Client::new(env, asset);
        token_client.transfer(from, to, &amount);
    }

    pub fn check_nonnegative_amount(amount: i128) {
        if amount < 0 {
            panic!("negative amount is not allowed: {}", amount)
        }
    }

    pub fn is_governor(_env: &Env, _sender: Address) {
        // let factory_client = index_factory_contract::Client::new(&env, &read_factory(&env));
        // let config = factory_client.query_config();

        // if config.governor != sender {
        //     log!(&env, "Index Token: You are not authorized!");
        //     panic_with_error!(&env, ErrorCode::NotAuthorized);
        // }
    }

    pub fn is_admin(env: &Env, sender: Address) {
        let admin = get_admin(env);
        if admin != sender {
            log!(&env, "Index Token: You are not authorized!");
            panic_with_error!(&env, ErrorCode::NotAuthorized);
        }
    }

    pub fn is_initialized(e: &Env) -> bool {
        e.storage().instance().get(&DataKey::Initialized).unwrap_or(false)
    }

    pub fn set_initialized(e: &Env) {
        e.storage().instance().set(&DataKey::Initialized, &true);
        e.storage().instance().extend_ttl(PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
    }

    pub fn save_admin(e: &Env, address: &Address) {
        e.storage().persistent().set(&DataKey::Admin, address);
        e.storage()
            .persistent()
            .extend_ttl(&DataKey::Admin, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
    }

    pub fn get_admin(e: &Env) -> Address {
        let admin = e.storage().persistent().get(&DataKey::Admin).unwrap();
        e.storage()
            .persistent()
            .extend_ttl(&DataKey::Admin, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        admin
    }

    pub fn save_governor(e: &Env, address: &Address) {
        e.storage().persistent().set(&DataKey::Governor, address);
        e.storage()
            .persistent()
            .extend_ttl(&DataKey::Governor, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
    }

    pub fn get_governor(e: &Env) -> Address {
        let governor = e.storage().persistent().get(&DataKey::Governor).unwrap();
        e.storage()
            .persistent()
            .extend_ttl(&DataKey::Governor, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        governor
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
}
