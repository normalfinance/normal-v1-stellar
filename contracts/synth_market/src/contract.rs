use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env, Symbol };

use crate::{
    errors,
    storage::{ get_admin },
    storage_types::{ DataKey },
    constants::{ MIN_MARGIN_RATIO, MAX_MARGIN_RATIO, LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO },
};

contractmeta!(
    key = "Description",
    val = "Synthetic asset tracking the value of another cryptocurrency"
);

#[contract]
pub struct SynthMarket;

pub trait SynthMarketTrait {
    // Sets the token contract addresses for this pool
    // token_wasm_hash is the WASM hash of the deployed token contract for the pool share token
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        stake_wasm_hash: BytesN<32>,
        token_wasm_hash: BytesN<32>,
        lp_init_info: LiquidityPoolInitInfo,
        factory_addr: Address,
        share_token_decimals: u32,
        share_token_name: String,
        share_token_symbol: String,
        default_slippage_bps: i64,
        max_allowed_fee_bps: i64
    );
}

#[contractimpl]
impl SynthMarketTrait for SynthMarket {
    pub fn __constructor(e: Env, reflector_contract_id: Address, token_wasm_hash: BytesN<32>) {
        // create the price oracle client instance
        let reflector_contract = PriceOracleClient::new(&env, &reflector_contract_id);

        // get oracle prcie precision
        let decimals = reflector_contract.decimals();

        // let share_contract = create_share_token(&e, token_wasm_hash, &token_a, &token_b);

        
        
    }

    pub fn freeze_oracle(e: Env) {}

    pub fn init_shutdown(e: Env) {}

    pub fn delete(e: Env) {}

    // Updates

    pub fn update_debt_ceiling(e: Env, debt_ceiling: u128) {
        is_admin(&e);

        if debt_ceiling > MAX_PROTOCOL_FEE_RATE {
            return Err(ErrorCode::ProtocolFeeRateMaxExceeded.into());
        }

        set_debt_ceiling(&e, debt_ceiling);
    }

    pub fn update_debt_floor(e: Env, debt_floor: u128) {
        is_admin(&e);

        // TODO: calculate the actual min/max debt floor
        let min_debt_floor = 0;

        if debt_floor < min_debt_floor {
            return Err(ErrorCode::ProtocolFeeRateMaxExceeded.into());
        }

        set_debt_floor(&e, debt_floor);
    }

    pub fn update_imf_factor(e: Env, imf_factor: u32) {
        validate!(imf_factor <= SPOT_IMF_PRECISION, ErrorCode::DefaultError, "invalid imf factor")?;

        log!("market {}", market.market_index);

        log!("market.imf_factor: {} -> {}", market.imf_factor, imf_factor);

        e.storage().instance().set(&DataKey::IMFFactor, &imf_factor);
    }

    pub fn update_liquidation_fee(e: Env, liquidation_fee: u64, if_liquidation_fee: u32) {
        msg!("updating market {} liquidation fee", market.market_index);

        validate!(
            liquidator_fee.safe_add(if_liquidation_fee)? < LIQUIDATION_FEE_PRECISION,
            ErrorCode::DefaultError,
            "Total liquidation fee must be less than 100%"
        )?;

        validate!(
            if_liquidation_fee < LIQUIDATION_FEE_PRECISION,
            ErrorCode::DefaultError,
            "If liquidation fee must be less than 100%"
        )?;

        validate!(
            margin_ratio_maintenance * LIQUIDATION_FEE_TO_MARGIN_PRECISION_RATIO > liquidation_fee,
            ErrorCode::InvalidMarginRatio,
            "margin_ratio_maintenance must be greater than liquidation fee"
        )?;

        msg!("market.liquidator_fee: {:?} -> {:?}", market.liquidator_fee, liquidator_fee);

        msg!(
            "market.if_liquidation_fee: {:?} -> {:?}",
            market.if_liquidation_fee,
            if_liquidation_fee
        );

        market.liquidator_fee = liquidator_fee;
        market.if_liquidation_fee = if_liquidation_fee;
    }

    pub fn update_liquidation_penalty(e: Env, liquidation_penalty: u64) {
        log!(&e, "updating market {} liquidation penalty", market.market_index);

        log!(
            "market.liquidation_penalty: {:?} -> {:?}",
            e.storage().instance().get(&DataKey::LiquidationPenalty).unwrap(),,
            liquidation_penalty
        );

        e.storage().instance().set(&DataKey::LiquidationPenalty, &margin_ratio_initial);
    }

    pub fn update_margin_ratio(e: Env, margin_ratio_initial: u32, margin_ratio_maintenance: u32) {
        log!(&e, "updating market {} margin ratio", market.market_index);

        if !(MIN_MARGIN_RATIO..=MAX_MARGIN_RATIO).contains(&margin_ratio_initial) {
            return Err(ErrorCode::InvalidMarginRatio);
        }

        if margin_ratio_initial <= margin_ratio_maintenance {
            return Err(ErrorCode::InvalidMarginRatio);
        }

        if !(MIN_MARGIN_RATIO..=MAX_MARGIN_RATIO).contains(&margin_ratio_maintenance) {
            return Err(ErrorCode::InvalidMarginRatio);
        }

        log!(
            &e,
            "market.margin_ratio_initial: {} -> {}",
            e.storage().instance().get(&DataKey::MarginRatioInitial).unwrap(),
            margin_ratio_initial
        );

        log!(
            &e,
            "market.margin_ratio_maintenance: {} -> {}",
            e.storage().instance().get(&DataKey::MarginRatioMaintenance).unwrap(),
            margin_ratio_maintenance
        );

        e.storage().instance().set(&DataKey::MarginRatioInitial, &margin_ratio_initial);
        e.storage().instance().set(&DataKey::MarginRatioMaintenance, &margin_ratio_maintenance);
    }

    pub fn update_name(e: Env, name: Symbol) {
        log!("market.name: {} -> {}", e.storage().instance().get(&DataKey::Name).unwrap(), name);
        e.storage().instance().set(&DataKey::Name, &margin_ratio_initial);
    }

    pub fn update_number_of_users(e: Env) -> u128 {}

    pub fn update_oracle(e: Env) {}

    pub fn update_paused_operations(e: Env, paused_operations: Vec<Operation>) {
        e.storage().instance().set(&DataKey::PausedOperations, &paused_operations);

        log_all_operations_paused(e.storage().instance().get(&DataKey::PausedOperations).unwrap())
    }

    pub fn update_status(e: Env, status: MarketStatus) {
        // validate!(
        //     !matches!(status, MarketStatus::Delisted | MarketStatus::Settlement),
        //     ErrorCode::DefaultError,
        //     "must set settlement/delist through another instruction"
        // )?;

        log!("market {}", market.market_index);

        log!("market.status: {:?} -> {:?}", market.status, status);

        e.storage().instance().set(&DataKey::Status, &status);
    }

    pub fn update_synthetic_tier(e: Env, synthetic_tier: SyntheticTier) {
        is_admin(&e);
        e.storage().instance().set(&DataKey::SyntheticTier, &synthetic_tier);
    }

    // Keeper

    pub fn liquidate(e: Env, liquidator_max_base_asset_amount: u64, limit_price: Option<u64>) {
        if user_key == liquidator_key {
            return Err(ErrorCode::UserCantLiquidateThemself);
        }

        // controller::liquidation::liquidate_vault(
        //     vault_index,
        //     liquidator_max_base_asset_amount,
        //     limit_price,
        //     user,
        //     &user_key,
        //     user_stats,
        //     liquidator,
        //     &liquidator_key,
        //     liquidator_stats,
        //     &market_map,
        //     &vault_map,
        //     &mut oracle_map,
        //     slot,
        //     now,
        //     state
        // )?;

        let liquidation_margin_buffer_ratio = state.liquidation_margin_buffer_ratio;
        let initial_pct_to_liquidate = state.initial_pct_to_liquidate as u128;
        let liquidation_duration = state.liquidation_duration as u128;

        validate!(!user.is_bankrupt(), ErrorCode::UserBankrupt, "user bankrupt")?;

        validate!(!liquidator.is_bankrupt(), ErrorCode::UserBankrupt, "liquidator bankrupt")?;

        validate!(
            !market.is_operation_paused(SynthOperation::Liquidation),
            ErrorCode::InvalidLiquidation,
            "Liquidation operation is paused for market {}",
            market_index
        )?;

        let margin_calculation =
            calculate_margin_requirement_and_total_collateral_and_liability_info(
                user,
                market_map,
                vault_map,
                oracle_map,
                MarginContext::liquidation(
                    liquidation_margin_buffer_ratio
                ).track_market_margin_requirement(MarketIdentifier::perp(market_index))?
            )?;
    }

    pub fn liquidate(e: Env, fee_rate: u128) -> u128 {}
}

pub fn log_all_operations_paused(current: u8) {
    for operation in ALL_SYNTH_OPERATIONS.iter() {
        if Self::is_operation_paused(current, *operation) {
            msg!("{:?} is paused", operation);
        }
    }
}
