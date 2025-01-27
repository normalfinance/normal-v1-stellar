use normal::{
    constants::{ PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD },
    error::NormalResult,
    oracle::OracleSource,
};
use soroban_sdk::{
    contracttype,
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

use crate::{ reward::RewardInfo, token_contract };
use soroban_decimal::Decimal;

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Pool,
    TickArrays,
    Positions,
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
pub struct PoolParams {
    pub admin: Address,
    pub tick_spacing: u32,
    pub initial_sqrt_price: u128,
    pub fee_rate: u32,
    pub protocol_fee_rate: u32,
    pub max_allowed_slippage_bps: i64,
    pub default_slippage_bps: i64,
    pub max_allowed_spread_bps: i64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pool {
    /// The synth token
    pub token_a: Address,
    /// The quote token (typically XLM or USDC)
    pub token_b: Address,
    /// The LP token representing liquidity ownership in the pool
    pub share_token: Address,


    





    ///
    pub tick_spacing: u32,
    ///
    pub tick_current_index: i32,
    /// Current amount of liquidity in the pool
    pub liquidity: u128,
    /// Current conversion price of the pool
    pub sqrt_price: u128,
    /// Swap fee charged by the pool for distribution to Liquidity Providers
    pub fee_rate: u32,
    /// Swap fee charged by the protocol for distribution to the Buffer,
    pub protocol_fee_rate: u32,
    /// Fees accumulated by the protocol
    pub fee_growth_global_a: u128,
    ///
    pub fee_growth_global_b: u128,
    /// Fees accumulated by the protocol (in the form of the synthetic token)
    pub protocol_fee_owed_a: u64,
    /// Fees accumulated by the protoocol (in the form of the quote token)
    pub protocol_fee_owed_b: u64,
    /// The maximum amount of slippage (in bps) that is tolerated during providing liquidity
    pub max_allowed_slippage_bps: i64,
    /// The maximum amount of spread (in bps) that is tolerated during swap
    pub max_allowed_spread_bps: i64,
    /// the maximum percent the pool price can deviate above or below the oracle twap
    pub max_allowed_variance_bps: i64,
    /// The last time rewards were updated
    pub reward_last_updated_timestamp: u64,
    ///
    pub reward_infos: Vec<RewardInfo>,
}

impl Pool {
    pub fn protocol_fee_rate(&self) -> Decimal {
        Decimal::bps(self.total_fee_bps)
    }

    pub fn max_allowed_slippage(&self) -> Decimal {
        Decimal::bps(self.max_allowed_slippage_bps)
    }

    pub fn transfer_a(self, from: Address, to: Address, amount: u64) {
        let token_client = token_contract::Client::new(&env, &self.token_a);
        token_client.transfer(&from, &to, &amount);
    }

    pub fn transfer_b(self, from: Address, to: Address, amount: u64) {
        let token_client = token_contract::Client::new(&env, &self.token_b);
        token_client.transfer(&from, &to, &amount);
    }

    /// Update all reward values for the AMM.
    ///
    /// # Parameters
    /// - `reward_infos` - An array of all updated amm rewards
    /// - `reward_last_updated_timestamp` - The timestamp when the rewards were last updated
    pub fn update_rewards(
        &mut self,
        reward_infos: [RewardInfo; NUM_REWARDS],
        reward_last_updated_timestamp: u64
    ) {
        self.reward_last_updated_timestamp = reward_last_updated_timestamp;
        self.reward_infos = reward_infos;
    }

    pub fn update_rewards_and_liquidity(
        &mut self,
        reward_infos: [RewardInfo; NUM_REWARDS],
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
        reward_infos: [RewardInfo; NUM_REWARDS],
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
            self.fee_growth_global_a = fee_growth_global;
            self.protocol_fee_owed_a += protocol_fee;
        } else {
            // Add fees taken via b
            self.fee_growth_global_b = fee_growth_global;
            self.protocol_fee_owed_b += protocol_fee;
        }
    }

    pub fn reset_protocol_fees_owed(&mut self) {
        self.protocol_fee_owed_a = 0;
        self.protocol_fee_owed_b = 0;
    }

    pub fn get_oracle_twap(&self, price_oracle: &Address, now: u64) -> NormalResult<Option<i64>> {
        match self.oracle_source {
            OracleSource::Band => { Ok(Some(self.get_band_twap(price_oracle, 1, false)?)) }
            OracleSource::Reflector => { Ok(Some(self.get_band_twap(price_oracle, 1, false)?)) }
            OracleSource::QuoteAsset => {
                log!(&env, "Can't get oracle twap for quote asset");
                Err(ErrorCode::DefaultError)
            }
        }
    }

    pub fn get_band_twap(
        &self,
        price_oracle: &Address,
        multiple: u128,
        is_pull_oracle: bool
    ) -> NormalResult<i64> {
        let mut pyth_price_data: &[u8] = &price_oracle
            .try_borrow_data()
            .or(Err(ErrorCode::UnableToLoadOracle))?;

        let oracle_price: i64;
        let oracle_twap: i64;
        let oracle_exponent: i32;

        if is_pull_oracle {
            let price_message = pyth_solana_receiver_sdk::price_update::PriceUpdateV2
                ::try_deserialize(&mut pyth_price_data)
                .or(Err(crate::error::ErrorCode::UnableToLoadOracle))?;
            oracle_price = price_message.price_message.price;
            oracle_twap = price_message.price_message.ema_price;
            oracle_exponent = price_message.price_message.exponent;
        } else {
            let price_data = pyth_client::cast::<pyth_client::Price>(pyth_price_data);
            oracle_price = price_data.agg.price;
            oracle_twap = price_data.twap.val;
            oracle_exponent = price_data.expo;
        }

        assert!(oracle_twap > oracle_price / 10);

        let oracle_precision = (10_u128).pow(oracle_exponent.unsigned_abs()).safe_div(multiple)?;

        let mut oracle_scale_mult = 1;
        let mut oracle_scale_div = 1;

        if oracle_precision > PRICE_PRECISION {
            oracle_scale_div = oracle_precision.safe_div(PRICE_PRECISION)?;
        } else {
            oracle_scale_mult = PRICE_PRECISION.safe_div(oracle_precision)?;
        }

        oracle_twap
            .cast::<i128>()?
            .safe_mul(oracle_scale_mult.cast()?)?
            .safe_div(oracle_scale_div.cast()?)?
            .cast::<i64>()
    }

    pub fn get_new_oracle_conf_pct(
        &self,
        confidence: u64, // price precision
        reserve_price: u64, // price precision
        now: i64
    ) -> NormalResult<u64> {
        // use previous value decayed as lower bound to avoid shrinking too quickly
        let upper_bound_divisor = 21_u64;
        let lower_bound_divisor = 5_u64;
        let since_last = now
            .safe_sub(self.historical_oracle_data.last_oracle_price_twap_ts)?
            .max(0);

        let confidence_lower_bound = if since_last > 0 {
            let confidence_divisor = upper_bound_divisor
                .saturating_sub(since_last.cast::<u64>()?)
                .max(lower_bound_divisor);
            self.last_oracle_conf_pct.safe_sub(self.last_oracle_conf_pct / confidence_divisor)?
        } else {
            self.last_oracle_conf_pct
        };

        Ok(
            confidence
                .safe_mul(BID_ASK_SPREAD_PRECISION)?
                .safe_div(reserve_price)?
                .max(confidence_lower_bound)
        )
    }

    pub fn is_recent_oracle_valid(&self, current_slot: u64) -> NormalResult<bool> {
        Ok(self.last_oracle_valid && current_slot == self.last_update_slot)
    }

    pub fn is_price_divergence_ok(&self, oracle_price: i64) -> NormalResult<bool> {
        let oracle_divergence = oracle_price
            .safe_sub(self.historical_oracle_data.last_oracle_price_twap_5min)?
            .safe_mul(PERCENTAGE_PRECISION_I64)?
            .safe_div(self.historical_oracle_data.last_oracle_price_twap_5min.min(oracle_price))?
            .unsigned_abs();

        let oracle_divergence_limit = match self.synthetic_tier {
            SyntheticTier::A => PERCENTAGE_PRECISION_U64 / 200, // 50 bps
            SyntheticTier::B => PERCENTAGE_PRECISION_U64 / 200, // 50 bps
            SyntheticTier::C => PERCENTAGE_PRECISION_U64 / 100, // 100 bps
            SyntheticTier::Speculative => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
            SyntheticTier::HighlySpeculative => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
            SyntheticTier::Isolated => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
        };

        if oracle_divergence >= oracle_divergence_limit {
            msg!(
                "market_index={} price divergence too large to safely settle pnl: {} >= {}",
                self.market_index,
                oracle_divergence,
                oracle_divergence_limit
            );
            return Ok(false);
        }

        let min_price = oracle_price.min(self.historical_oracle_data.last_oracle_price_twap_5min);

        let std_limit = (
            match self.synthetic_tier {
                SyntheticTier::A => min_price / 50, // 200 bps
                SyntheticTier::B => min_price / 50, // 200 bps
                SyntheticTier::C => min_price / 20, // 500 bps
                SyntheticTier::Speculative => min_price / 10, // 1000 bps
                SyntheticTier::HighlySpeculative => min_price / 10, // 1000 bps
                SyntheticTier::Isolated => min_price / 10, // 1000 bps
            }
        ).unsigned_abs();

        if self.oracle_std.max(self.mark_std) >= std_limit {
            msg!(
                "market_index={} std too large to safely settle pnl: {} >= {}",
                self.market_index,
                self.oracle_std.max(self.mark_std),
                std_limit
            );
            return Ok(false);
        }

        Ok(true)
    }

    pub fn get_max_confidence_interval_multiplier(self) -> NormalResult<u64> {
        // assuming validity_guard_rails max confidence pct is 2%
        Ok(match self.synthetic_tier {
            SyntheticTier::A => 1, // 2%
            SyntheticTier::B => 1, // 2%
            SyntheticTier::C => 2, // 4%
            SyntheticTier::Speculative => 10, // 20%
            SyntheticTier::HighlySpeculative => 50, // 100%
            SyntheticTier::Isolated => 50, // 100%
        })
    }

    pub fn get_sanitize_clamp_denominator(self) -> NormalResult<Option<i64>> {
        Ok(match self.synthetic_tier {
            SyntheticTier::A => Some(10_i64), // 10%
            SyntheticTier::B => Some(5_i64), // 20%
            SyntheticTier::C => Some(2_i64), // 50%
            SyntheticTier::Speculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            SyntheticTier::HighlySpeculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            SyntheticTier::Isolated => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
        })
    }
}

pub fn get_pool(env: &Env) -> Pool {
    let pool = env.storage().persistent().get(&DataKey::Pool).unwrap();
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Pool, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
    pool
}

pub fn save_pool(env: &Env, pool: Pool) {
    env.storage().persistent().set(&DataKey::Pool, &pool);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Pool, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

// ...

pub mod utils {
    use normal::constants::{ INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD };
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
