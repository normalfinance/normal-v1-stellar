pub(crate) const MAX_MARGIN_RATIO: u32 = MARGIN_PRECISION; // 1x leverage
pub(crate) const MIN_MARGIN_RATIO: u32 = MARGIN_PRECISION / 50; // 50x leverage

pub(crate) const PERCENTAGE_PRECISION: u128 = 1_000_000; // expo -6 (represents 100%)
pub(crate) const MARGIN_PRECISION: u32 = 10_000; // expo = -4
pub(crate) const LIQUIDATION_FEE_PRECISION: u32 = PERCENTAGE_PRECISION as u32; // expo = -6
pub(crate) const LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO: u32 = // expo 2
    LIQUIDATION_FEE_PRECISION / MARGIN_PRECISION;
