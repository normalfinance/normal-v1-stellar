use normal::{ error::{ ErrorCode, NormalResult }, oracle::OracleSource, types::SynthTier };
use soroban_decimal::Decimal;
use soroban_sdk::{ contracttype, log, Address, Env, Map, Vec };

use crate::math::token_math::{ MAX_FEE_RATE, MAX_PROTOCOL_FEE_RATE };

use super::{ reward::RewardInfo, tick_array::TickArray };

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolParams {
    pub tick_spacing: u32,
    pub initial_sqrt_price: u128,
    pub fee_rate: u32,
    pub protocol_fee_rate: u32,
    pub max_allowed_slippage_bps: i64,
    pub max_allowed_variance_bps: i64,
}

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

    /// `x` reserves for constant product mm formula (x * y = k)
    // /// precision: AMM_RESERVE_PRECISION
    // pub base_asset_reserve: u128,
    // /// `y` reserves for constant product mm formula (x * y = k)
    // /// precision: AMM_RESERVE_PRECISION
    // pub quote_asset_reserve: u128,
    // /// minimum base_asset_reserve allowed before AMM is unavailable
    // /// precision: AMM_RESERVE_PRECISION
    // pub min_base_asset_reserve: u128,
    // /// maximum base_asset_reserve allowed before AMM is unavailable
    // /// precision: AMM_RESERVE_PRECISION
    // pub max_base_asset_reserve: u128,

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
            TickArray::new(env, env.current_contract_address(), start_tick_index, self.tick_spacing)
        )
    }

    pub fn get_reward_by_token(&self, token: Address) -> Option<(RewardInfo, usize)> {
        for (i, reward) in self.reward_infos.iter().enumerate() {
            if reward.token == token {
                return (reward, i);
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
        reward_last_updated_timestamp: u64
    ) {
        self.reward_last_updated_timestamp = reward_last_updated_timestamp;
        self.reward_infos = reward_infos;
    }

    pub fn update_rewards_and_liquidity(
        &mut self,
        reward_infos: Vec<RewardInfo>,
        liquidity: u128,
        reward_last_updated_timestamp: u64
    ) {
        self.update_rewards(reward_infos, reward_last_updated_timestamp);
        self.liquidity = liquidity;
    }

    /// Update the reward authority at the specified Whirlpool reward index.
    pub fn update_reward_authority(
        &mut self,
        reward_token: Address,
        authority: Address
    ) -> NormalResult<()> {
        let (mut reward, _i) = self.get_reward_by_token(reward_token)?;
        reward.authority = authority;

        Ok(())
    }

    pub fn update_emissions(
        &mut self,
        index: usize,
        reward_infos: Vec<RewardInfo>,
        timestamp: u64,
        emissions_per_second_x64: u128
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

    pub fn update_fee_rate(&mut self, fee_rate: i64) -> NormalResult<()> {
        if fee_rate > MAX_FEE_RATE {
            return Err(ErrorCode::FeeRateMaxExceeded);
        }
        self.fee_rate = fee_rate;

        Ok(())
    }

    pub fn update_protocol_fee_rate(&mut self, protocol_fee_rate: i64) -> NormalResult<()> {
        if protocol_fee_rate > MAX_PROTOCOL_FEE_RATE {
            return Err(ErrorCode::ProtocolFeeRateMaxExceeded);
        }
        self.protocol_fee_rate = protocol_fee_rate;

        Ok(())
    }

    pub fn reset_protocol_fees_owed(&mut self) {
        self.protocol_fee_owed_a = 0;
        self.protocol_fee_owed_b = 0;
    }

    pub fn get_oracle_price_deviance(self, env: &Env) -> i128 {
        let oracle_price = self.get_oracle_twap(price_oracle, now)?;

        let price_diff = self.sqrt_price.safe_sub(oracle_price, env)?;

        price_diff
    }

    pub fn get_liquidity_delta_for_price_impact(&self, price_impact: i64) -> NormalResult<i128> {}

    pub fn get_oracle_twap(&self, price_oracle: &Address, now: u64) -> NormalResult<Option<i64>> {
        match self.oracle_source {
            OracleSource::Band => Ok(Some(self.get_band_twap(price_oracle, 1)?)),
            OracleSource::Reflector => Ok(Some(self.get_band_twap(price_oracle, 1)?)),
            OracleSource::QuoteAsset => {
                // log!(&env, "Can't get oracle twap for quote asset");
                Err(ErrorCode::DefaultError)
            }
        }
    }

    pub fn get_band_twap(&self, price_oracle: &Address, multiple: u128) -> NormalResult<i64> {
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

    pub fn is_price_divergence_ok(&self, env: &Env, oracle_price: i64) -> NormalResult<bool> {
        let oracle_divergence = oracle_price
            .safe_sub(self.historical_oracle_data.last_oracle_price_twap_5min)?
            .safe_mul(PERCENTAGE_PRECISION_I64)?
            .safe_div(self.historical_oracle_data.last_oracle_price_twap_5min.min(oracle_price))?
            .unsigned_abs();

        let oracle_divergence_limit = match self.synthetic_tier {
            SynthTier::A => PERCENTAGE_PRECISION_U64 / 200, // 50 bps
            SynthTier::B => PERCENTAGE_PRECISION_U64 / 200, // 50 bps
            SynthTier::C => PERCENTAGE_PRECISION_U64 / 100, // 100 bps
            SynthTier::Speculative => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
            SynthTier::HighlySpeculative => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
            SynthTier::Isolated => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
        };

        if oracle_divergence >= oracle_divergence_limit {
            log!(
                env,
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
                SynthTier::A => min_price / 50, // 200 bps
                SynthTier::B => min_price / 50, // 200 bps
                SynthTier::C => min_price / 20, // 500 bps
                SynthTier::Speculative => min_price / 10, // 1000 bps
                SynthTier::HighlySpeculative => min_price / 10, // 1000 bps
                SynthTier::Isolated => min_price / 10, // 1000 bps
            }
        ).unsigned_abs();

        if self.oracle_std.max(self.mark_std) >= std_limit {
            log!(
                env,
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
            SynthTier::A => 1, // 2%
            SynthTier::B => 1, // 2%
            SynthTier::C => 2, // 4%
            SynthTier::Speculative => 10, // 20%
            SynthTier::HighlySpeculative => 50, // 100%
            SynthTier::Isolated => 50, // 100%
        })
    }

    pub fn get_sanitize_clamp_denominator(self) -> NormalResult<Option<i64>> {
        Ok(match self.synthetic_tier {
            SynthTier::A => Some(10_i64), // 10%
            SynthTier::B => Some(5_i64), // 20%
            SynthTier::C => Some(2_i64), // 50%
            SynthTier::Speculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            SynthTier::HighlySpeculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            SynthTier::Isolated => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
        })
    }
}
