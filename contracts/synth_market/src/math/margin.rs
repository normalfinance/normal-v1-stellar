


pub fn calculate_margin_requirement_and_total_collateral_and_liability_info(
	user: &User,
	market_id: u64,
	oracle_map: &mut OracleMap,
	context: MarginContext
) -> NormalResult<MarginCalculation> {
	let mut calculation = MarginCalculation::new(context);

	let user_custom_margin_ratio = if
		context.margin_type == MarginRequirementType::Initial
	{
		user.max_margin_ratio
	} else {
		0_u32
	};

	for vault_position in user.vault_positions.iter() {
		if vault_position.is_available() {
			continue;
		}

		let market = market_map.get_ref(&vault_position.market_index)?;

		let (oracle_price_data, oracle_validity) =
			oracle_map.get_price_data_and_validity(
				MarketType::Synth,
				market.market_index,
				&market.oracle,
				market.historical_oracle_data.last_oracle_price_twap,
				market.get_max_confidence_interval_multiplier()?
			)?;

		calculation.update_all_oracles_valid(
			is_oracle_valid_for_action(
				oracle_validity,
				Some(NormalAction::MarginCalc)
			)?
		);

		// TODO: ...
	}

	calculation.validate_num_spot_liabilities()?;

	Ok(calculation)
}