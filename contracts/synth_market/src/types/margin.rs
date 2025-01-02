#[contracttype]
#[derive(Clone, Debug)]
pub struct MarginCalculation {
    pub context: MarginContext,
    pub total_collateral: i128,
    pub margin_requirement: u128,
    #[cfg(not(test))]
    margin_requirement_plus_buffer: u128,
    #[cfg(test)]
    pub margin_requirement_plus_buffer: u128,
    pub num_vault_liabilities: u8,
    pub all_oracles_valid: bool,
    pub with_perp_isolated_liability: bool,
    pub total_spot_asset_value: i128,
    pub total_vault_liability_value: u128,
    // pub open_orders_margin_requirement: u128,
    tracked_market_margin_requirement: u128,
}

impl MarginCalculation {
    pub fn new(context: MarginContext) -> Self {
        Self {
            context,
            total_collateral: 0,
            margin_requirement: 0,
            margin_requirement_plus_buffer: 0,
            num_vault_liabilities: 0,
            all_oracles_valid: true,
            with_perp_isolated_liability: false,
            total_spot_asset_value: 0,
            total_vault_liability_value: 0,
            tracked_market_margin_requirement: 0,
        }
    }

    pub fn add_total_collateral(
        &mut self,
        total_collateral: i128,
        env: &Env
    ) -> Result<(), Symbol> {
        self.total_collateral = self.total_collateral
            .checked_add(total_collateral)
            .ok_or_else(|| Symbol::from_str(env, "OverflowError"))?;
        Ok(())
    }
}
