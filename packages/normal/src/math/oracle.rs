use core::{cmp::max, fmt};

use soroban_sdk::{contracttype, log, Address, Env};

use crate::math::casting::Cast;
use crate::math::safe_math::SafeMath;
use crate::{
    constants::BID_ASK_SPREAD_PRECISION,
    error::ErrorCode,
    oracle::{OraclePriceData, ValidityGuardRails},
};

// use crate::
// use super::amm::is_oracle_mark_too_divergent;

// ordered by "severity"
#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq, Default)]
pub enum OracleValidity {
    NonPositive,
    TooVolatile,
    TooUncertain,
    StaleForMargin,
    InsufficientDataPoints,
    StaleForAMM,
    #[default]
    Valid,
}

impl OracleValidity {
    pub fn get_error_code(&self) -> ErrorCode {
        match self {
            OracleValidity::NonPositive => ErrorCode::OracleNonPositive,
            OracleValidity::TooVolatile => ErrorCode::OracleTooVolatile,
            OracleValidity::TooUncertain => ErrorCode::OracleTooUncertain,
            OracleValidity::StaleForMargin => ErrorCode::OracleStaleForMargin,
            OracleValidity::InsufficientDataPoints => ErrorCode::OracleInsufficientDataPoints,
            OracleValidity::StaleForAMM => ErrorCode::OracleStaleForAMM,
            OracleValidity::Valid => unreachable!(),
        }
    }
}

impl fmt::Display for OracleValidity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OracleValidity::NonPositive => write!(f, "NonPositive"),
            OracleValidity::TooVolatile => write!(f, "TooVolatile"),
            OracleValidity::TooUncertain => write!(f, "TooUncertain"),
            OracleValidity::StaleForMargin => write!(f, "StaleForMargin"),
            OracleValidity::InsufficientDataPoints => write!(f, "InsufficientDataPoints"),
            OracleValidity::StaleForAMM => write!(f, "StaleForAMM"),
            OracleValidity::Valid => write!(f, "Valid"),
        }
    }
}

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum NormalAction {
    Liquidate,
    MarginCalc,
    UpdateTwap,
    UpdateAMMCurve,
    OracleOrderPrice,
}

pub fn is_oracle_valid_for_action(
    oracle_validity: OracleValidity,
    action: Option<NormalAction>,
) -> bool {
    let is_ok = match action {
        Some(action) => match action {
            NormalAction::OracleOrderPrice => {
                matches!(
                    oracle_validity,
                    OracleValidity::Valid
                        | OracleValidity::StaleForAMM
                        | OracleValidity::InsufficientDataPoints
                )
            }
            // TODO: revisit
            NormalAction::IndexPricing => matches!(
                oracle_validity,
                OracleValidity::Valid
                    | OracleValidity::StaleForAMM
                    | OracleValidity::InsufficientDataPoints
            ),
            NormalAction::MarginCalc => !matches!(
                oracle_validity,
                OracleValidity::NonPositive
                    | OracleValidity::TooVolatile
                    | OracleValidity::TooUncertain
                    | OracleValidity::StaleForMargin
            ),

            NormalAction::Liquidate => !matches!(
                oracle_validity,
                OracleValidity::NonPositive | OracleValidity::TooVolatile
            ),
            NormalAction::UpdateTwap => !matches!(oracle_validity, OracleValidity::NonPositive),
            NormalAction::UpdateAMMCurve => !matches!(oracle_validity, OracleValidity::NonPositive),
        },
        None => {
            matches!(oracle_validity, OracleValidity::Valid)
        }
    };

    is_ok
}

pub fn block_operation(// env: Env,
    // market_name: String,
    // oracle_price_data: &OraclePriceData,
    // guard_rails: &OracleGuardRails,
    // reserve_price: u64,
    // now: u64,
) -> bool {
    // let OracleStatus {
    //     oracle_validity,
    //     mark_too_divergent: is_oracle_mark_too_divergent,
    //     oracle_res_price_spread_pct: _,
    //     ..
    // } = get_oracle_status(env, market_name, oracle_price_data, guard_rails, reserve_price)?;

    // let is_oracle_valid = is_oracle_valid_for_action(
    //     oracle_validity,
    //     Some(NormalAction::IndexPricing)
    // )?;

    // // let slots_since_amm_update = slot.saturating_sub(market.amm.last_update_slot);

    // // TODO: when else should we block
    // let block = !is_oracle_valid || is_oracle_mark_too_divergent;
    let block = false;
    block
}

#[contracttype]
#[derive(Default, Clone, Copy, Debug)]
pub struct OracleStatus {
    pub price_data: OraclePriceData,
    pub oracle_res_price_spread_pct: i64,
    pub mark_too_divergent: bool,
    pub oracle_validity: OracleValidity,
}

pub fn get_oracle_status(
    // env: Env,
    // market_name: String,
    // pool: &SynthPool,
    oracle_price_data: &OraclePriceData,
    // guard_rails: &OracleGuardRails,
    // reserve_price: u64,
) -> OracleStatus {
    // let oracle_validity = oracle_validity(
    //     env,
    //     market_name,
    //     market.amm.historical_oracle_data.last_oracle_price_twap,
    //     oracle_price_data,
    //     &guard_rails.validity,
    //     SynthMarket::get_max_confidence_interval_multiplier(),
    //     false
    // )?;

    // let oracle_res_price_spread_pct = math::amm::calculate_oracle_twap_5min_price_spread_pct(
    //     &market.amm,
    //     reserve_price
    // )?;
    // let is_oracle_mark_too_divergent = is_oracle_mark_too_divergent(
    //     oracle_res_price_spread_pct,
    //     &guard_rails.price_divergence
    // );

    // Ok(OracleStatus {
    //     price_data: *oracle_price_data,
    //     oracle_res_price_spread_pct,
    //     mark_too_divergent: is_oracle_mark_too_divergent,
    //     oracle_validity,
    // })
    OracleStatus {
        price_data: *oracle_price_data,
        oracle_res_price_spread_pct: 1,
        mark_too_divergent: false,
        oracle_validity: OracleValidity::Valid,
    }
}

pub fn oracle_validity(
    env: &Env,
    market_id: Address,
    last_oracle_twap: i64,
    oracle_price_data: &OraclePriceData,
    valid_oracle_guard_rails: &ValidityGuardRails,
    max_confidence_interval_multiplier: u64,
    log_validity: bool,
) -> OracleValidity {
    let OraclePriceData {
        price: oracle_price,
        confidence: oracle_conf,
        delay: oracle_delay,
        has_sufficient_data_points,
        ..
    } = *oracle_price_data;

    let is_oracle_price_nonpositive = oracle_price <= 0;

    let is_oracle_price_too_volatile = oracle_price
        .max(last_oracle_twap)
        .safe_div(last_oracle_twap.min(oracle_price).max(1), env)
        .gt(&valid_oracle_guard_rails.too_volatile_ratio);

    let conf_pct_of_price = max(1, oracle_conf)
        .safe_mul(BID_ASK_SPREAD_PRECISION, env)
        .safe_div(oracle_price.cast(env), env);

    // TooUncertain
    let is_conf_too_large = conf_pct_of_price.gt(&valid_oracle_guard_rails
        .confidence_interval_max_size
        .safe_mul(max_confidence_interval_multiplier, env));

    let is_stale_for_amm = oracle_delay.gt(&valid_oracle_guard_rails
        .slots_before_stale_for_amm
        .cast::<u64>(env));
    let is_stale_for_margin = oracle_delay.gt(&valid_oracle_guard_rails
        .slots_before_stale_for_margin
        .cast::<u64>(env));

    let oracle_validity = if is_oracle_price_nonpositive {
        OracleValidity::NonPositive
    } else if is_oracle_price_too_volatile {
        OracleValidity::TooVolatile
    } else if is_conf_too_large {
        OracleValidity::TooUncertain
    } else if is_stale_for_margin {
        OracleValidity::StaleForMargin
    } else if !has_sufficient_data_points {
        OracleValidity::InsufficientDataPoints
    } else if is_stale_for_amm {
        OracleValidity::StaleForAMM
    } else {
        OracleValidity::Valid
    };

    if log_validity {
        if !has_sufficient_data_points {
            log!(
                env,
                "Invalid {} {} Oracle: Insufficient Data Points",
                market_id
            );
        }

        if is_oracle_price_nonpositive {
            log!(
                env,
                "Invalid {} {} Oracle: Non-positive (oracle_price <=0)",
                market_id
            );
        }

        if is_oracle_price_too_volatile {
            log!(
                env,
                "Invalid {} {} Oracle: Too Volatile (last_oracle_price_twap={:?} vs oracle_price={:?})",
                market_id,
                last_oracle_twap,
                oracle_price
            );
        }

        if is_conf_too_large {
            log!(
                env,
                "Invalid {} {} Oracle: Confidence Too Large (is_conf_too_large={:?})",
                market_id,
                conf_pct_of_price
            );
        }

        if is_stale_for_amm || is_stale_for_margin {
            log!(
                env,
                "Invalid {} {} Oracle: Stale (oracle_delay={:?})",
                market_id,
                oracle_delay
            );
        }
    }

    oracle_validity
}
