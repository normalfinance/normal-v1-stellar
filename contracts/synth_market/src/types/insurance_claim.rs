use soroban_sdk::contracttype;

#[contracttype]
#[derive(Default, Eq, PartialEq, Debug)]
pub struct InsuranceClaim {
    /// The amount of revenue last settled
    /// Positive if funds left the market,
    /// negative if funds were pulled into the market
    /// precision: QUOTE_PRECISION
    pub revenue_withdraw_since_last_settle: i64,
    /// The max amount of revenue that can be withdrawn per period
    /// precision: QUOTE_PRECISION
    pub max_revenue_withdraw_per_period: u64,
    /// The max amount of insurance that market can use to resolve bankruptcy and pnl deficits
    /// precision: QUOTE_PRECISION
    pub quote_max_insurance: u64,
    /// The amount of insurance that has been used to resolve bankruptcy and pnl deficits
    /// precision: QUOTE_PRECISION
    pub quote_settled_insurance: u64,
    /// The last time revenue was settled in/out of market
    pub last_revenue_withdraw_ts: i64,
}
