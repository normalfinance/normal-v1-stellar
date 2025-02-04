use normal::{
    constants::{LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO, MAX_MARGIN_RATIO, MIN_MARGIN_RATIO},
    error::ErrorCode,
    validate,
};
use soroban_sdk::Env;

pub fn validate_margin(
    env: &Env,
    margin_ratio_initial: u32,
    margin_ratio_maintenance: u32,
    liquidation_fee: u32,
) {
    if !(MIN_MARGIN_RATIO..=MAX_MARGIN_RATIO).contains(&margin_ratio_initial) {
        return Err(ErrorCode::InvalidMarginRatio);
    }

    if margin_ratio_initial <= margin_ratio_maintenance {
        return Err(ErrorCode::InvalidMarginRatio);
    }

    if !(MIN_MARGIN_RATIO..=MAX_MARGIN_RATIO).contains(&margin_ratio_maintenance) {
        return Err(ErrorCode::InvalidMarginRatio);
    }

    validate!(
        env,
        margin_ratio_maintenance * LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO > liquidation_fee,
        ErrorCode::InvalidMarginRatio,
        "margin_ratio_maintenance must be greater than liquidation fee"
    )?;
}
