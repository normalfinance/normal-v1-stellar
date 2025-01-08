use normal::ttl::{ PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD };
use soroban_sdk::{
    contracttype,
    log,
    panic_with_error,
    symbol_short,
    xdr::ToXdr,
    Address,
    Bytes,
    BytesN,
    ConversionError,
    Env,
    Symbol,
    TryFromVal,
    Val,
};

use crate::{ error::ContractError, token_contract };
use soroban_decimal::Decimal;

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    TotalShares = 0,
    ReserveA = 1,
    ReserveB = 2,
    Admin = 3,
    Initialized = 4,
}

impl TryFromVal<Env, DataKey> for Val {
    type Error = ConversionError;

    fn try_from_val(_env: &Env, v: &DataKey) -> Result<Self, Self::Error> {
        Ok((*v as u32).into())
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub token_a: Address,
    pub token_b: Address,
    pub share_token: Address,

    pub tick_arrays: TickArray,
    pub tick_spacing: u32,
    pub tick_current_index: i32,
    pub liquidity: u128,
    pub sqrt_price: u128,

    pub positions: Positions,

    pub fee_rate: u32,
    pub protocol_fee_rate: u32,

    pub fee_growth_global_a: u128,
    pub fee_growth_global_b: u128,

    pub protocol_fee_owed_a: u64,
    pub protocol_fee_owed_b: u64,

    /// The maximum amount of slippage (in bps) that is tolerated during providing liquidity
    pub max_allowed_slippage_bps: i64,
    /// The maximum amount of spread (in bps) that is tolerated during swap
    pub max_allowed_spread_bps: i64,
    /// the maximum percent the pool price can deviate above or below the oracle twap
    pub max_allowed_variance_bps: i64,

    pub reward_last_updated_timestamp: u64,
    pub reward_infos: Vec<RewardInfo>,
}
const CONFIG: Symbol = symbol_short!("CONFIG");

const DEFAULT_SLIPPAGE_BPS: Symbol = symbol_short!("DSLIPBPS");
pub fn save_default_slippage_bps(env: &Env, bps: i64) {
    env.storage().persistent().set(&DEFAULT_SLIPPAGE_BPS, &bps);
    env.storage()
        .persistent()
        .extend_ttl(&DEFAULT_SLIPPAGE_BPS, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT)
}

pub fn get_default_slippage_bps(env: &Env) -> i64 {
    let bps = env
        .storage()
        .persistent()
        .get(&DEFAULT_SLIPPAGE_BPS)
        .expect("Stable wasm hash not set");

    env.storage()
        .persistent()
        .extend_ttl(&DEFAULT_SLIPPAGE_BPS, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
    bps
}

impl Config {
    pub fn protocol_fee_rate(&self) -> Decimal {
        Decimal::bps(self.total_fee_bps)
    }

    pub fn max_allowed_slippage(&self) -> Decimal {
        Decimal::bps(self.max_allowed_slippage_bps)
    }

    /// Update all reward values for the AMM.
    ///
    /// # Parameters
    /// - `reward_infos` - An array of all updated amm rewards
    /// - `reward_last_updated_timestamp` - The timestamp when the rewards were last updated
    pub fn update_rewards(
        &mut self,
        reward_infos: [AMMRewardInfo; NUM_REWARDS],
        reward_last_updated_timestamp: u64
    ) {
        self.reward_last_updated_timestamp = reward_last_updated_timestamp;
        self.reward_infos = reward_infos;
    }

    pub fn update_rewards_and_liquidity(
        &mut self,
        reward_infos: [AMMRewardInfo; NUM_REWARDS],
        liquidity: u128,
        reward_last_updated_timestamp: u64
    ) {
        self.update_rewards(reward_infos, reward_last_updated_timestamp);
        self.liquidity = liquidity;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_after_swap(
        &mut self,
        liquidity: u128,
        tick_index: i32,
        sqrt_price: u128,
        fee_growth_global: u128,
        reward_infos: [AMMRewardInfo; NUM_REWARDS],
        protocol_fee: u64,
        is_token_fee_in_a: bool,
        reward_last_updated_timestamp: u64
    ) {
        self.tick_current_index = tick_index;
        self.sqrt_price = sqrt_price;
        self.liquidity = liquidity;
        self.reward_infos = reward_infos;
        self.reward_last_updated_timestamp = reward_last_updated_timestamp;
        if is_token_fee_in_a {
            // Add fees taken via a
            self.fee_growth_global_synthetic = fee_growth_global;
            self.protocol_fee_owed_synthetic += protocol_fee;
        } else {
            // Add fees taken via b
            self.fee_growth_global_quote = fee_growth_global;
            self.protocol_fee_owed_quote += protocol_fee;
        }
    }
}

pub fn get_config(env: &Env) -> Config {
    let config = env.storage().persistent().get(&CONFIG).unwrap();
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

// ...

pub mod utils {
    use normal::ttl::{ INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD };
    use soroban_sdk::String;

    use super::*;

    #[allow(clippy::too_many_arguments)]
    pub fn deploy_token_contract(
        env: &Env,
        token_wasm_hash: BytesN<32>,
        token_a: &Address,
        token_b: &Address,
        admin: Address,
        decimals: u32,
        name: String,
        symbol: String
    ) -> Address {
        let mut salt = Bytes::new(env);
        salt.append(&token_a.clone().to_xdr(env));
        salt.append(&token_b.clone().to_xdr(env));
        let salt = env.crypto().sha256(&salt);
        env.deployer()
            .with_current_contract(salt)
            .deploy_v2(token_wasm_hash, (admin, decimals, name, symbol))
    }

    // ...

    pub fn save_total_shares(e: &Env, amount: i128) {
        e.storage().persistent().set(&DataKey::TotalShares, &amount);
        e.storage()
            .persistent()
            .extend_ttl(
                &DataKey::TotalShares,
                PERSISTENT_LIFETIME_THRESHOLD,
                PERSISTENT_BUMP_AMOUNT
            );
    }

    pub fn save_pool_balance_a(e: &Env, amount: i128) {
        e.storage().persistent().set(&DataKey::ReserveA, &amount);
        e.storage()
            .persistent()
            .extend_ttl(&DataKey::ReserveA, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
    }

    pub fn save_pool_balance_b(e: &Env, amount: i128) {
        e.storage().persistent().set(&DataKey::ReserveB, &amount);
        e.storage()
            .persistent()
            .extend_ttl(&DataKey::ReserveB, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
    }

    // ...

    pub fn mint_shares(e: &Env, share_token: &Address, to: &Address, amount: i128) {
        let total = get_total_shares(e);

        token_contract::Client::new(e, share_token).mint(to, &amount);

        save_total_shares(e, total + amount);
    }

    pub fn burn_shares(e: &Env, share_token: &Address, amount: i128) {
        let total = get_total_shares(e);

        token_contract::Client::new(e, share_token).burn(&e.current_contract_address(), &amount);

        save_total_shares(e, total - amount);
    }

    // ...

    pub fn get_total_shares(e: &Env) -> i128 {
        let total_shares = e.storage().persistent().get(&DataKey::TotalShares).unwrap();
        e.storage()
            .persistent()
            .extend_ttl(
                &DataKey::TotalShares,
                PERSISTENT_LIFETIME_THRESHOLD,
                PERSISTENT_BUMP_AMOUNT
            );

        total_shares
    }
    pub fn get_pool_balance_a(e: &Env) -> i128 {
        let balance_a = e.storage().persistent().get(&DataKey::ReserveA).unwrap();
        e.storage()
            .persistent()
            .extend_ttl(&DataKey::ReserveA, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        balance_a
    }

    pub fn get_pool_balance_b(e: &Env) -> i128 {
        let balance_b = e.storage().persistent().get(&DataKey::ReserveB).unwrap();
        e.storage()
            .persistent()
            .extend_ttl(&DataKey::ReserveB, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

        balance_b
    }

    pub fn get_balance(e: &Env, contract: &Address) -> i128 {
        token_contract::Client::new(e, contract).balance(&e.current_contract_address())
    }

    // ...

    pub fn is_initialized(e: &Env) -> bool {
        e.storage().persistent().get(&DataKey::Initialized).unwrap_or(false)
    }

    pub fn set_initialized(e: &Env) {
        e.storage().persistent().set(&DataKey::Initialized, &true);
        e.storage()
            .persistent()
            .extend_ttl(
                &DataKey::Initialized,
                PERSISTENT_LIFETIME_THRESHOLD,
                PERSISTENT_BUMP_AMOUNT
            );
    }
}
