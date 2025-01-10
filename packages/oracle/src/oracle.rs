#![no_std]

use soroban_sdk::{ contracttype, vec, Env, Vec };

pub(crate) const PERCENTAGE_PRECISION: u128 = 1_000_000; // expo -6 (represents 100%)
pub(crate) const PERCENTAGE_PRECISION_I128: i128 = PERCENTAGE_PRECISION as i128;
pub(crate) const PERCENTAGE_PRECISION_U64: u64 = PERCENTAGE_PRECISION as u64;
pub(crate) const PERCENTAGE_PRECISION_I64: i64 = PERCENTAGE_PRECISION as i64;

pub(crate) const PRICE_PRECISION: u128 = 1_000_000; //expo = -6;
pub(crate) const PRICE_PRECISION_I128: i128 = PRICE_PRECISION as i128;
pub(crate) const PRICE_PRECISION_U64: u64 = 1_000_000; //expo = -6;
pub(crate) const PRICE_PRECISION_I64: i64 = 1_000_000; //expo = -6;

/// Handle Contract Errors
#[derive(Debug, Eq, PartialEq)]
pub enum OracleError {
    /// A monotonic function is a function between ordered sets that preserves
    /// or reverses the given order, but never both.
    // "Curve isn't monotonic"
    NotMonotonic,
}

/// Oracles types
#[contracttype]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OracleSource {
    Band, // (https://github.com/bandprotocol/band-std-reference-contracts-soroban/tree/main)
    Reflector, // (https://github.com/reflector-network/reflector-contract)
    // Dia,
    QuoteAsset,
}

#[contracttype]
#[derive(Default, Clone, Copy, Debug)]
pub struct OraclePriceData {
    pub price: i64,
    pub confidence: u64,
    pub delay: i64,
    pub has_sufficient_number_of_data_points: bool,
}

impl Oracle {
    pub fn get_oracle_price(
        env: Env,
        oracle_source: OracleSource,
        price_oracle_address: Address,
        base_asset: Option<Symbol>, // ("BTC", "USD")
        quote_asset: Option<Symbol>
    ) -> OraclePriceData {
        match oracle_source {
            OracleSource::Band =>
                Self::get_band_price(&env, price_oracle_address, base_asset, quote_asset),
            OracleSource::Reflector =>
                Self::get_reflector_price(env, price_oracle_address, base_asset),
            OracleSource::QuoteAsset =>
                OraclePriceData {
                    price: PRICE_PRECISION_I64,
                    confidence: 1,
                    delay: 0,
                    has_sufficient_number_of_data_points: true,
                },
        }
    }

    pub fn generate_oracle_price_data(oracle_price: i64) -> OraclePriceData {
        let mut oracle_scale_mult = 1;
        let mut oracle_scale_div = 1;

        if oracle_precision > PRICE_PRECISION {
            oracle_scale_div = oracle_precision.safe_div(PRICE_PRECISION)?;
        } else {
            oracle_scale_mult = PRICE_PRECISION.safe_div(oracle_precision)?;
        }

        let oracle_price_scaled = oracle_price
            .cast::<i128>()?
            .safe_mul(oracle_scale_mult.cast()?)?
            .safe_div(oracle_scale_div.cast()?)?
            .cast::<i64>()?;

        let oracle_conf_scaled = oracle_conf
            .cast::<u128>()?
            .safe_mul(oracle_scale_mult)?
            .safe_div(oracle_scale_div)?
            .cast::<u64>()?;

        // let oracle_delay: i64 = clock_slot.cast::<i64>()?.safe_sub(published_slot.cast()?)?;

        OraclePriceData {
            price: oracle_price_scaled,
            confidence: oracle_conf_scaled,
            delay: oracle_delay,
            has_sufficient_number_of_data_points,
        }
    }

    pub fn is_oracle_too_divergent_with_twap_5min(
        oracle_price: i64,
        oracle_twap_5min: i64,
        max_divergence: i64
    ) -> bool {
        let percent_diff = oracle_price
            .safe_sub(oracle_twap_5min)?
            .abs()
            .safe_mul(PERCENTAGE_PRECISION_U64.cast::<i64>()?)?
            .safe_div(oracle_twap_5min.abs())?;

        let too_divergent = percent_diff >= max_divergence;
        if too_divergent {
            log!("max divergence {}", max_divergence);
            log!(
                "Oracle Price Too Divergent from TWAP 5min. oracle: {} twap: {}",
                oracle_price,
                oracle_twap_5min
            );
        }

        too_divergent
    }
}

fn get_band_price(
    env: Env,
    oracle_contract_address: Address,
    base_asset: Symbol,
    quote_asset: Symbol
) -> OraclePriceData {
    let client = band_std_reference::Client::new(&env, &oracle_contract_address);

    let price = client
        .get_reference_data(&Vec::from_array(&env, [(base_asset, quote_asset)]))
        .get_unchecked(0)
        .unwrap().rate;

    let price_data = generate_oracle_price_data();

    price_data;
}

fn get_reflector_price(
    env: Env,
    price_oracle: &AccountInfo,
    base_asset: Symbol
) -> OraclePriceData {
    let client = reflector_price_oracle::Client::new(&env, &reflector_contract_id);

    // let decimals = client.decimals();

    let price = client.lastprice(&base_asset).unwrap(); // Asset::Other(Symbol::new(&env, "BTC"))

    let price_data = generate_oracle_price_data();

    price_data;
}

#[contracttype]
#[derive(Copy, Clone, Debug)]
pub struct OracleGuardRails {
    pub price_divergence: PriceDivergenceGuardRails,
    pub validity: ValidityGuardRails,
}

impl OracleGuardRails {
    fn default() -> Self {
        OracleGuardRails {
            price_divergence: PriceDivergenceGuardRails::default(),
            validity: ValidityGuardRails {
                slots_before_stale_for_amm: 10, // ~5 seconds
                confidence_interval_max_size: 20_000, // 2% of price
                too_volatile_ratio: 5, // 5x or 80% down
            },
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[contracttype]
pub struct PriceDivergenceGuardRails {
    pub mark_oracle_percent_divergence: u64,
    pub oracle_twap_5min_percent_divergence: u64,
}

impl PriceDivergenceGuardRails {
    fn default() -> Self {
        PriceDivergenceGuardRails {
            mark_oracle_percent_divergence: PERCENTAGE_PRECISION_U64 / 10,
            oracle_twap_5min_percent_divergence: PERCENTAGE_PRECISION_U64 / 2,
        }
    }
}

#[derive(Copy, Clone, Default, Debug)]
#[contracttype]
pub struct ValidityGuardRails {
    pub slots_before_stale_for_amm: i64,
    pub confidence_interval_max_size: u64,
    pub too_volatile_ratio: i64,
}
