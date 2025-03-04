use crate::math::casting::Cast;
use crate::math::safe_math::SafeMath;
use crate::{
    band_std_reference,
    constants::{PERCENTAGE_PRECISION_U64, PRICE_PRECISION_I64},
};
use soroban_sdk::{contracttype, log, Address, Env, Symbol, Vec};

#[contracttype]
#[derive(Default, Clone, Copy, Eq, PartialEq, Debug)]
pub struct HistoricalOracleData {
    /// precision: PRICE_PRECISION
    pub last_oracle_price: i64,
    /// precision: PRICE_PRECISION
    pub last_oracle_conf: u64,
    /// number of slots since last update
    pub last_oracle_delay: u64,
    /// precision: PRICE_PRECISION
    pub last_oracle_price_twap: i64,
    // /// precision: PRICE_PRECISION
    // pub last_oracle_price_twap_5min: i64,
    /// unix_timestamp of last snapshot
    pub last_oracle_price_twap_ts: u64,
}

impl HistoricalOracleData {
    pub fn default_quote_oracle() -> Self {
        HistoricalOracleData {
            last_oracle_price: PRICE_PRECISION_I64,
            last_oracle_conf: 0,
            last_oracle_delay: 0,
            last_oracle_price_twap: PRICE_PRECISION_I64,
            // last_oracle_price_twap_5min: PRICE_PRECISION_I64,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_price(price: i64) -> Self {
        HistoricalOracleData {
            last_oracle_price: price,
            last_oracle_conf: 0,
            last_oracle_delay: 10,
            last_oracle_price_twap: price,
            // last_oracle_price_twap_5min: price,
            ..HistoricalOracleData::default()
        }
    }

    pub fn default_with_current_oracle(oracle_price_data: OraclePriceData) -> Self {
        HistoricalOracleData {
            last_oracle_price: oracle_price_data.price,
            last_oracle_conf: oracle_price_data.confidence,
            last_oracle_delay: oracle_price_data.delay,
            last_oracle_price_twap: oracle_price_data.price,
            // last_oracle_price_twap_5min: oracle_price_data.price,
            // last_oracle_price_twap_ts: now,
            ..HistoricalOracleData::default()
        }
    }
}

#[contracttype]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OracleSource {
    Band, // (https://github.com/bandprotocol/band-std-reference-contracts-soroban/tree/main)
          // Reflector, // (https://github.com/reflector-network/reflector-contract)
          // QuoteAsset,
}

#[contracttype]
#[derive(Default, Clone, Copy, Debug)]
pub struct OraclePriceData {
    pub price: i64,
    pub confidence: u64,
    pub delay: u64,
    pub has_sufficient_data_points: bool,
}

pub fn get_oracle_price(
    env: &Env,
    oracle_source: &OracleSource,
    price_oracle_address: &Address,
    symbol_pair: (Symbol, Symbol),
    now: u64,
) -> OraclePriceData {
    match oracle_source {
        OracleSource::Band => get_band_price(env, price_oracle_address, symbol_pair, now),
    }
}

pub fn is_oracle_too_divergent_with_twap_5min(
    env: &Env,
    oracle_price: i64,
    oracle_twap_5min: i64,
    max_divergence: i64,
) -> bool {
    let percent_diff = oracle_price
        .safe_sub(oracle_twap_5min, env)
        .abs()
        .safe_mul(PERCENTAGE_PRECISION_U64.cast::<i64>(env), env)
        .safe_div(oracle_twap_5min.abs(), env);

    let too_divergent = percent_diff >= max_divergence;
    if too_divergent {
        log!(env, "max divergence {}", max_divergence);
        log!(
            env,
            "Oracle Price Too Divergent from TWAP 5min. oracle: {} twap: {}",
            oracle_price,
            oracle_twap_5min
        );
    }

    too_divergent
}

pub fn get_band_price(
    env: &Env,
    oracle_contract_address: &Address,
    symbol_pair: (Symbol, Symbol),
    now: u64, // multiple: u128,
) -> OraclePriceData {
    let client = band_std_reference::Client::new(env, oracle_contract_address);

    let reference_datum = client
        .get_reference_data(&Vec::from_array(env, [symbol_pair]))
        .get_unchecked(0);

    //  or(Err(crate::error::ErrorCode::UnableToLoadOracle))?;

    let oracle_price = reference_datum.rate;
    // let oracle_conf: u64;
    let has_sufficient_data_points: bool = true;
    // let mut oracle_precision: u128;
    let published_slot = reference_datum.last_updated_base;

    // oracle_price = price_message.price_message.price;
    // oracle_conf = price_message.price_message.conf;
    // oracle_precision = (10_u128).pow(price_message.price_message.exponent.unsigned_abs());
    // published_slot = price_message.posted_slot;

    // if oracle_precision <= multiple {
    //     log!("Multiple larger than oracle precision");
    //     return Err(crate::error::ErrorCode::InvalidOracle);
    // }
    // oracle_precision = oracle_precision.safe_div(multiple)?;

    let oracle_scale_mult = 1;
    let oracle_scale_div = 1;

    // if oracle_precision > PRICE_PRECISION {
    //     oracle_scale_div = oracle_precision.safe_div(PRICE_PRECISION)?;
    // } else {
    //     oracle_scale_mult = PRICE_PRECISION.safe_div(oracle_precision)?;
    // }

    let oracle_price_scaled = oracle_price
        .cast::<i128>(env)
        .safe_mul(oracle_scale_mult.cast(env), env)
        .safe_div(oracle_scale_div.cast(env), env)
        .cast::<i64>(env);

    // let oracle_conf_scaled = oracle_conf
    //     .cast::<u128>()?
    //     .safe_mul(oracle_scale_mult)?
    //     .safe_div(oracle_scale_div)?
    //     .cast::<u64>()?;

    let oracle_delay: u64 = now.safe_sub(published_slot, env);

    OraclePriceData {
        price: oracle_price_scaled,
        confidence: 1, // oracle_conf_scaled,
        delay: oracle_delay,
        has_sufficient_data_points,
    }
}

#[contracttype]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OracleGuardRails {
    pub price_divergence: PriceDivergenceGuardRails,
    pub validity: ValidityGuardRails,
}

impl OracleGuardRails {
    pub fn default() -> Self {
        OracleGuardRails {
            price_divergence: PriceDivergenceGuardRails::default(),
            validity: ValidityGuardRails {
                slots_before_stale_for_amm: 10,       // ~5 seconds
                slots_before_stale_for_margin: 120,   // ~60 seconds
                confidence_interval_max_size: 20_000, // 2% of price
                too_volatile_ratio: 5,                // 5x or 80% down
            },
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct PriceDivergenceGuardRails {
    pub mark_oracle_percent_divergence: u64,
    pub oracle_twap_5min_perc_div: u64,
}

impl PriceDivergenceGuardRails {
    pub fn default() -> Self {
        PriceDivergenceGuardRails {
            mark_oracle_percent_divergence: PERCENTAGE_PRECISION_U64 / 10,
            oracle_twap_5min_perc_div: PERCENTAGE_PRECISION_U64 / 2,
        }
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
#[contracttype]
pub struct ValidityGuardRails {
    pub slots_before_stale_for_amm: i64,
    pub slots_before_stale_for_margin: i64,
    pub confidence_interval_max_size: u64,
    pub too_volatile_ratio: i64,
}

//  ----------

// fn should_get_quote_asset_price_data(&self, pubkey: &Pubkey) -> bool {
//     pubkey == &Pubkey::default()
// }

// pub fn get_price_data(&mut self, pubkey: &Pubkey) -> NormalResult<&OraclePriceData> {
//     if self.should_get_quote_asset_price_data(pubkey) {
//         return Ok(&self.quote_asset_price_data);
//     }

//     if self.price_data.contains_key(pubkey) {
//         return self.price_data.get(pubkey).safe_unwrap();
//     }

//     let (account_info, oracle_source) = match self.oracles.get(pubkey) {
//         Some(AccountInfoAndOracleSource { account_info, oracle_source }) =>
//             (account_info, oracle_source),
//         None => {
//             msg!("oracle pubkey not found in oracle_map: {}", pubkey);
//             return Err(ErrorCode::OracleNotFound);
//         }
//     };

//     let price_data = get_oracle_price(oracle_source, account_info, self.slot)?;

//     self.price_data.insert(*pubkey, price_data);

//     self.price_data.get(pubkey).safe_unwrap()
// }

// pub fn get_price_data_and_validity(
//     &mut self,
//     market_type: MarketType,
//     market_index: u16,
//     pubkey: &Pubkey,
//     last_oracle_price_twap: i64,
//     max_confidence_interval_multiplier: u64
// ) -> NormalResult<(&OraclePriceData, OracleValidity)> {
//     if self.should_get_quote_asset_price_data(pubkey) {
//         return Ok((&self.quote_asset_price_data, OracleValidity::Valid));
//     }

//     if self.price_data.contains_key(pubkey) {
//         let oracle_price_data = self.price_data.get(pubkey).safe_unwrap()?;

//         let oracle_validity = if let Some(oracle_validity) = self.validity.get(pubkey) {
//             *oracle_validity
//         } else {
//             let oracle_validity = oracle_validity(
//                 market_type,
//                 market_index,
//                 last_oracle_price_twap,
//                 oracle_price_data,
//                 &self.oracle_guard_rails.validity,
//                 max_confidence_interval_multiplier,
//                 true
//             )?;
//             self.validity.insert(*pubkey, oracle_validity);
//             oracle_validity
//         };
//         return Ok((oracle_price_data, oracle_validity));
//     }

//     let (account_info, oracle_source) = match self.oracles.get(pubkey) {
//         Some(AccountInfoAndOracleSource { account_info, oracle_source }) =>
//             (account_info, oracle_source),
//         None => {
//             msg!("oracle pubkey not found in oracle_map: {}", pubkey);
//             return Err(ErrorCode::OracleNotFound);
//         }
//     };

//     let price_data = get_oracle_price(oracle_source, account_info, self.slot)?;

//     self.price_data.insert(*pubkey, price_data);

//     let oracle_price_data = self.price_data.get(pubkey).safe_unwrap()?;
//     let oracle_validity = oracle_validity(
//         market_type,
//         market_index,
//         last_oracle_price_twap,
//         oracle_price_data,
//         &self.oracle_guard_rails.validity,
//         max_confidence_interval_multiplier,
//         true
//     )?;
//     self.validity.insert(*pubkey, oracle_validity);

//     Ok((oracle_price_data, oracle_validity))
// }

// pub fn get_price_data_and_guard_rails(
//     &mut self,
//     pubkey: &Pubkey
// ) -> NormalResult<(&OraclePriceData, &ValidityGuardRails)> {
//     if self.should_get_quote_asset_price_data(pubkey) {
//         let validity_guard_rails = &self.oracle_guard_rails.validity;
//         return Ok((&self.quote_asset_price_data, validity_guard_rails));
//     }

//     if self.price_data.contains_key(pubkey) {
//         let oracle_price_data = self.price_data.get(pubkey).safe_unwrap()?;
//         let validity_guard_rails = &self.oracle_guard_rails.validity;

//         return Ok((oracle_price_data, validity_guard_rails));
//     }

//     let (account_info, oracle_source) = match self.oracles.get(pubkey) {
//         Some(AccountInfoAndOracleSource { account_info, oracle_source }) =>
//             (account_info, oracle_source),
//         None => {
//             msg!("oracle pubkey not found in oracle_map: {}", pubkey);
//             return Err(ErrorCode::OracleNotFound);
//         }
//     };

//     let price_data = get_oracle_price(oracle_source, account_info, self.slot)?;

//     self.price_data.insert(*pubkey, price_data);

//     let oracle_price_data = self.price_data.get(pubkey).safe_unwrap()?;
//     let validity_guard_rails = &self.oracle_guard_rails.validity;

//     Ok((oracle_price_data, validity_guard_rails))
// }
