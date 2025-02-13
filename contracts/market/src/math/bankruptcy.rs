use crate::state::market_position::MarketPosition;

pub fn is_position_bankrupt(position: &MarketPosition) -> bool {
    // user is bankrupt iff they have spot liabilities, no spot assets, and no perp exposure

    let mut has_liability = false;

    if position.base_asset_amount != 0
        || position.quote_asset_amount > 0
        || position.has_open_order()
        || position.is_lp()
    {
        return false;
    }

    if position.quote_asset_amount < 0 {
        has_liability = true;
    }

    has_liability
}
