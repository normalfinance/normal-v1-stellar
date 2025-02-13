use normal::{
    constants::{LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO, MAX_MARGIN_RATIO, MIN_MARGIN_RATIO},
    validate,
};
use soroban_sdk::{panic_with_error, Env};

use crate::errors::Errors;

pub fn validate_margin(
    env: &Env,
    margin_ratio_initial: u32,
    margin_ratio_maintenance: u32,
    liquidation_fee: u32,
) {
    if !(MIN_MARGIN_RATIO..=MAX_MARGIN_RATIO).contains(&margin_ratio_initial) {
        panic_with_error!(env, Errors::InvalidMarginRatio);
    }

    if margin_ratio_initial <= margin_ratio_maintenance {
        panic_with_error!(env, Errors::InvalidMarginRatio);
    }

    if !(MIN_MARGIN_RATIO..=MAX_MARGIN_RATIO).contains(&margin_ratio_maintenance) {
        panic_with_error!(env, Errors::InvalidMarginRatio);
    }

    validate!(
        env,
        margin_ratio_maintenance * LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO > liquidation_fee,
        Errors::InvalidMarginRatio,
        "margin_ratio_maintenance must be greater than liquidation fee"
    )?;
}
