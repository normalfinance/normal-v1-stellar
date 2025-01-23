use crate::{constants::PERCENTAGE_PRECISION_U64, oracle::PriceDivergenceGuardRails};

pub fn is_oracle_mark_too_divergent(
    price_spread_pct: i64,
    oracle_guard_rails: &PriceDivergenceGuardRails,
) -> bool {
    // NormalResult<bool>
    let max_divergence = oracle_guard_rails
        .mark_oracle_percent_divergence
        .max(PERCENTAGE_PRECISION_U64 / 10);
    price_spread_pct.unsigned_abs() > max_divergence
}
