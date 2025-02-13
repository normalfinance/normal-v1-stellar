use normal::{
    constants::BID_ASK_SPREAD_PRECISION,
    error::ErrorCode,
    math::{casting::Cast, safe_math::SafeMath},
    oracle::{HistoricalOracleData, OracleSource},
};
use soroban_decimal::Decimal;
use soroban_sdk::{contracttype, panic_with_error, Address, Env, Map, Vec};

use crate::math::token_math::{MAX_FEE_RATE, MAX_PROTOCOL_FEE_RATE};

use super::{reward::RewardInfo, tick_array::TickArray};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Pool {
    /// The synth token
    pub token_a: Address,
    /// The quote token (typically XLM or USDC)
    pub token_b: Address,
    /// The LP token representing liquidity ownership in the pool
    pub lp_token: Address,
    ///
    pub tick_spacing: u32,
    ///
    pub tick_current_index: i32,
    ///
    pub tick_arrays: Map<i32, TickArray>, // start_tick_index > TickArray

    /// oracle price data public key
    pub oracle: Address,
    /// the oracle provider information. used to decode/scale the oracle public key
    pub oracle_source: OracleSource,
    /// stores historically witnessed oracle data
    pub historical_oracle_data: HistoricalOracleData,
    /// the last seen oracle price partially shrunk toward the amm reserve price
    /// precision: PRICE_PRECISION
    pub last_oracle_normalised_price: i64,
    /// the gap between the oracle price and the reserve price = y * peg_multiplier / x
    pub last_oracle_price_spread_pct: i64,
    /// average estimate of price
    pub last_price_twap: u64,
    /// the pct size of the oracle confidence interval
    /// precision: PERCENTAGE_PRECISION
    pub last_oracle_conf_pct: u64,
    /// estimate of standard deviation of the oracle price at each update
    /// precision: PRICE_PRECISION
    pub oracle_std: u64,
    /// the last unix_timestamp the twap was updated
    pub last_price_twap_ts: i64,
    /// tracks whether the oracle was considered valid at the last AMM update
    pub last_oracle_valid: bool,

    // TODO: do we need to manually track reserve values?
    /// Current amount of liquidity in the pool
    pub liquidity: u128,
    /// Current conversion price of the pool
    pub sqrt_price: u128,
    /// Swap fee charged by the pool for distribution to Liquidity Providers
    pub fee_rate: i64,
    /// Swap fee charged by the protocol for distribution to the Buffer,
    pub protocol_fee_rate: i64,
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
    /// the maximum percent the pool price can deviate above or below the oracle twap
    pub max_allowed_variance_bps: i64,
    /// The last time rewards were updated
    pub reward_last_updated_timestamp: u64,
    ///
    pub reward_infos: Vec<RewardInfo>,

    /// the last blockchain slot the amm was updated
    pub last_update_slot: u64,
}

impl Pool {
    pub fn protocol_fee_rate(&self) -> Decimal {
        Decimal::bps(self.protocol_fee_rate)
    }

    pub fn max_allowed_slippage(&self) -> Decimal {
        Decimal::bps(self.max_allowed_slippage_bps)
    }

    pub fn initiliaze_tick_array(&mut self, env: &Env, start_tick_index: i32) {
        self.tick_arrays.set(
            start_tick_index,
            TickArray::new(
                env,
                env.current_contract_address(),
                start_tick_index,
                self.tick_spacing,
            ),
        )
    }

    pub fn get_reward_by_token(&self, token: Address) -> Option<(RewardInfo, usize)> {
        for (i, reward) in self.reward_infos.iter().enumerate() {
            if reward.token == token {
                return Some((reward, i));
            }
        }
        return (None, 0);
    }

    /// Update all reward values for the AMM.
    ///
    /// # Parameters
    /// - `reward_infos` - An array of all updated amm rewards
    /// - `reward_last_updated_timestamp` - The timestamp when the rewards were last updated
    pub fn update_rewards(
        &mut self,
        reward_infos: Vec<RewardInfo>,
        reward_last_updated_timestamp: u64,
    ) {
        self.reward_last_updated_timestamp = reward_last_updated_timestamp;
        self.reward_infos = reward_infos;
    }

    pub fn update_rewards_and_liquidity(
        &mut self,
        reward_infos: Vec<RewardInfo>,
        liquidity: u128,
        reward_last_updated_timestamp: u64,
    ) {
        self.update_rewards(reward_infos, reward_last_updated_timestamp);
        self.liquidity = liquidity;
    }

    /// Update the reward authority at the specified Whirlpool reward index.
    pub fn update_reward_authority(&mut self, reward_token: Address, authority: Address) {
        let (mut reward, _i) = self.get_reward_by_token(reward_token);
        reward.authority = authority;
    }

    pub fn update_emissions(
        &mut self,
        index: usize,
        reward_infos: Vec<RewardInfo>,
        timestamp: u64,
        emissions_per_second_x64: u128,
    ) {
        self.update_rewards(reward_infos, timestamp);
        self.reward_infos[index].emissions_per_second_x64 = emissions_per_second_x64;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_after_swap(
        &mut self,
        liquidity: u128,
        tick_index: i32,
        sqrt_price: u128,
        fee_growth_global: u128,
        reward_infos: Vec<RewardInfo>,
        protocol_fee: u64,
        is_token_fee_in_a: bool,
        reward_last_updated_timestamp: u64,
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

    pub fn update_fee_rate(&mut self, env: &Env, fee_rate: i64) {
        if fee_rate > MAX_FEE_RATE {
            panic_with_error!(env, ErrorCode::FeeRateMaxExceeded);
        }
        self.fee_rate = fee_rate;
    }

    pub fn update_protocol_fee_rate(&mut self, env: &Env, protocol_fee_rate: i64) {
        if protocol_fee_rate > MAX_PROTOCOL_FEE_RATE {
            panic_with_error!(env, ErrorCode::ProtocolFeeRateMaxExceeded);
        }
        self.protocol_fee_rate = protocol_fee_rate;
    }

    pub fn reset_protocol_fees_owed(&mut self) {
        self.protocol_fee_owed_a = 0;
        self.protocol_fee_owed_b = 0;
    }

    pub fn get_oracle_price_deviance(self, env: &Env, now: u64) -> i128 {
        let oracle_price = self.get_oracle_twap(env, &self.oracle, now)?;

        let price_diff = self.sqrt_price.safe_sub(oracle_price, env);

        price_diff
    }

    pub fn get_liquidity_delta_for_price_impact(&self, price_impact: i64) -> i128 {}

    pub fn get_oracle_twap(&self, env: &Env, price_oracle: &Address, now: u64) -> Option<i64> {
        match self.oracle_source {
            OracleSource::Band => Some(self.get_band_twap(env, price_oracle, 1)),
        }
    }

    pub fn get_band_twap(&self, env: &Env, price_oracle: &Address, multiple: u128) -> i64 {
        let mut pyth_price_data: &[u8] = &price_oracle
            .try_borrow_data()
            .or(Err(ErrorCode::UnableToLoadOracle))?;

        let oracle_price: i64;
        let oracle_twap: i64;
        let oracle_exponent: i32;

        // let price_message = pyth_solana_receiver_sdk::price_update::PriceUpdateV2::try_deserialize(
        //     &mut pyth_price_data,
        // )
        // .or(Err(crate::error::ErrorCode::UnableToLoadOracle))?;
        // oracle_price = price_message.price_message.price;
        // oracle_twap = price_message.price_message.ema_price;
        // oracle_exponent = price_message.price_message.exponent;

        // assert!(oracle_twap > oracle_price / 10);

        // let oracle_precision = (10_u128)
        //     .pow(oracle_exponent.unsigned_abs())
        //     .safe_div(multiple)?;

        let mut oracle_scale_mult = 1;
        let mut oracle_scale_div = 1;

        // if oracle_precision > PRICE_PRECISION {
        //     oracle_scale_div = oracle_precision.safe_div(PRICE_PRECISION)?;
        // } else {
        //     oracle_scale_mult = PRICE_PRECISION.safe_div(oracle_precision)?;
        // }

        oracle_twap
            .cast::<i128>(env)
            .safe_mul(oracle_scale_mult.cast(env), env)
            .safe_div(oracle_scale_div.cast(env), env)
            .cast::<i64>(env)
    }

    pub fn get_new_oracle_conf_pct(
        &self,
        env: &Env,
        confidence: u64,    // price precision
        reserve_price: u64, // price precision
        now: i64,
    ) -> u64 {
        // use previous value decayed as lower bound to avoid shrinking too quickly
        let upper_bound_divisor = 21_u64;
        let lower_bound_divisor = 5_u64;
        let since_last = now
            .safe_sub(self.historical_oracle_data.last_oracle_price_twap_ts, env)
            .max(0);

        let confidence_lower_bound = if since_last > 0 {
            let confidence_divisor = upper_bound_divisor
                .saturating_sub(since_last.cast::<u64>(env))
                .max(lower_bound_divisor);
            self.last_oracle_conf_pct
                .safe_sub(self.last_oracle_conf_pct / confidence_divisor, env)
        } else {
            self.last_oracle_conf_pct
        };

        confidence
            .safe_mul(BID_ASK_SPREAD_PRECISION, env)
            .safe_div(reserve_price, env)
            .max(confidence_lower_bound)
    }

    pub fn is_recent_oracle_valid(&self, current_slot: u64) -> bool {
        self.last_oracle_valid && current_slot == self.last_update_slot
    }
}
