use soroban_sdk::{ contracttype, Address };

pub(crate) const PERCENTAGE_PRECISION: u128 = 1_000_000; // expo -6 (represents 100%)
pub(crate) const PERCENTAGE_PRECISION_I128: i128 = PERCENTAGE_PRECISION as i128;
pub(crate) const PERCENTAGE_PRECISION_U64: u64 = PERCENTAGE_PRECISION as u64;
pub(crate) const PERCENTAGE_PRECISION_I64: i64 = PERCENTAGE_PRECISION as i64;

pub(crate) const PRICE_PRECISION: u128 = 1_000_000; //expo = -6;
pub(crate) const PRICE_PRECISION_I128: i128 = PRICE_PRECISION as i128;
pub(crate) const PRICE_PRECISION_U64: u64 = 1_000_000; //expo = -6;
pub(crate) const PRICE_PRECISION_I64: i64 = 1_000_000; //expo = -6;

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Admin,
    EmergencyAdmin,
    Oracle,
    Frozen,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum OracleSource {
    Band, // (https://github.com/bandprotocol/band-std-reference-contracts-soroban/tree/main)
    Reflector,
    Dia,
    QuoteAsset,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct OraclePriceData {
    pub price: i64,
    pub confidence: u64,
    pub delay: i64,
    pub has_sufficient_number_of_data_points: bool,
}

#[derive(Copy, Clone, Debug)]
#[contracttype]
pub struct OracleGuardRails {
    pub price_divergence: PriceDivergenceGuardRails,
    pub validity: ValidityGuardRails,
}

// impl Default for OracleGuardRails {
//     fn default() -> Self {
//         OracleGuardRails {
//             price_divergence: PriceDivergenceGuardRails::default(),
//             validity: ValidityGuardRails {
//                 slots_before_stale_for_amm: 10, // ~5 seconds
//                 confidence_interval_max_size: 20_000, // 2% of price
//                 too_volatile_ratio: 5, // 5x or 80% down
//             },
//         }
//     }
// }

#[derive(Copy, Clone, Debug)]
#[contracttype]
pub struct PriceDivergenceGuardRails {
    pub mark_oracle_percent_divergence: u64,
    pub oracle_twap_5min_percent_divergence: u64,
}

// impl Default for PriceDivergenceGuardRails {
//     fn default() -> Self {
//         PriceDivergenceGuardRails {
//             mark_oracle_percent_divergence: PERCENTAGE_PRECISION_U64 / 10,
//             oracle_twap_5min_percent_divergence: PERCENTAGE_PRECISION_U64 / 2,
//         }
//     }
// }

#[derive(Copy, Clone, Default, Debug)]
#[contracttype]
pub struct ValidityGuardRails {
    pub slots_before_stale_for_amm: i64,
    pub confidence_interval_max_size: u64,
    pub too_volatile_ratio: i64,
}
