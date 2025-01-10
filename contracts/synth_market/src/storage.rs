use soroban_sdk::{ Address, Env };

use crate::storage_types::{ DataKey, Stake };

use soroban_sdk::{ contracttype, Address };

pub const ADMIN: Symbol = symbol_short!("ADMIN");
pub const GOVERNOR: Symbol = symbol_short!("GOVERNOR");

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Market,
    Admin,
    Governor,
    // Initialized,
}

pub fn is_admin(env: &Env, address: Address) {
    let admin_addr = env.storage().instance().get(&ADMIN).unwrap_or_else(|| {
        log!(env, "Factory: Admin not set");
        panic_with_error!(&env, ContractError::AdminNotSet)
    });

    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

    if admin_addr != address {
        return Err(ErrorCode::NotAuthorized)
    }
}

pub fn is_governor(env: &Env, address: Address) {
    let governor_addr = env.storage().instance().get(&GOVERNOR).unwrap_or_else(|| {
        log!(env, "Market: Governor not set");
        panic_with_error!(&env, ContractError::GovernorNotSet)
    });

    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

    if governor_addr != address {
        return Err(ErrorCode::NotAuthorized)
    }
}


// ################################################################

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SynthMarketParams {}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SynthMarket {
    /// oracle price data public key
    pub oracle: Address,
    /// The token of the market
    pub token: Address,
    /// The market's liquidity pool
    pub amm: Address,



    /// LP Management 
    
    /// The optimatal AMM position to deposit new liquidity into
    pub liquidity_position_ts: u64,
    pub last_liquidity_position_rebalance_ts: u64,


    /// End LP Management 
     


    /// Encoded display name for the market e.g. BTC-XLM
    pub name: String,
    /// The market's token decimals. To from decimals to a precision, 10^decimals
    pub decimals: u32,
    /// Whether a market is active, reduce only, expired, etc
    /// Affects whether users can open/close positions
    pub status: MarketStatus,
    /// The synthetic tier determines how much insurance a market can receive, with more speculative markets receiving less insurance
    /// It also influences the order markets can be liquidated, with less speculative markets being liquidated first
    pub synth_tier: SyntheticTier,
    pub paused_operations: u8,
    pub number_of_users: u32,
    /// The sum of the scaled balances for collateral deposits across users
    /// To convert to the collateral token amount, multiply by the cumulative deposit interest
    /// precision: SPOT_BALANCE_PRECISION
    pub collateral_balance: u128,
    /// The sum of the scaled balances for borrows across users
    /// To convert to the borrow token amount, multiply by the cumulative borrow interest
    /// precision: SPOT_BALANCE_PRECISION
    pub debt_balance: u128,
    /// The cumulative interest earned by depositors
    /// Used to calculate the deposit token amount from the deposit balance
    /// precision: SPOT_CUMULATIVE_INTEREST_PRECISION
    pub cumulative_deposit_interest: u128,
    pub cumulative_lp_interest: u128,
    /// no withdraw limits/guards when deposits below this threshold
    /// precision: token mint precision
    pub withdraw_guard_threshold: u64,
    /// The max amount of token deposits in this market
    /// 0 if there is no limit
    /// precision: token mint precision
    pub max_token_deposits: u64,
    /// 24hr average of deposit token amount
    /// precision: token mint precision
    pub collateral_token_twap: u64,
    /// 24hr average of borrow token amount
    /// precision: token mint precision
    pub debt_token_twap: u64,
    /// 24hr average of utilization
    /// which is debt amount over collateral amount
    /// precision: SPOT_UTILIZATION_PRECISION
    pub utilization_twap: u64,
    /// Last time the cumulative deposit interest was updated
    pub last_interest_ts: u64,
    /// Last time the deposit/borrow/utilization averages were updated
    pub last_twap_ts: u64,
    /// The ts when the market will be expired. Only set if market is in reduce only mode
    pub expiry_timestamp: i64,
    /// The price at which positions will be settled. Only set if market is expired
    /// precision = PRICE_PRECISION
    pub expiry_price: i64,
    /// The maximum spot position size
    /// if the limit is 0, there is no limit
    /// precision: token mint precision
    pub max_position_size: u64,
    /// Every deposit has a deposit record id. This is the next id to use
    pub next_deposit_record_id: u64,
    /// The initial asset weight used to calculate a deposits contribution to a users initial total collateral
    /// e.g. if the asset weight is .8, $100 of deposits contributes $80 to the users initial total collateral
    /// precision: SPOT_WEIGHT_PRECISION
    pub initial_asset_weight: u32,
    /// The maintenance asset weight used to calculate a deposits contribution to a users maintenance total collateral
    /// e.g. if the asset weight is .9, $100 of deposits contributes $90 to the users maintenance total collateral
    /// precision: SPOT_WEIGHT_PRECISION
    pub maintenance_asset_weight: u32,
    /// The initial liability weight used to calculate a borrows contribution to a users initial margin requirement
    /// e.g. if the liability weight is .9, $100 of borrows contributes $90 to the users initial margin requirement
    /// precision: SPOT_WEIGHT_PRECISION
    pub initial_liability_weight: u32,
    /// The maintenance liability weight used to calculate a borrows contribution to a users maintenance margin requirement
    /// e.g. if the liability weight is .8, $100 of borrows contributes $80 to the users maintenance margin requirement
    /// precision: SPOT_WEIGHT_PRECISION
    pub maintenance_liability_weight: u32,
    /// The initial margin fraction factor. Used to increase margin ratio for large positions
    /// precision: MARGIN_PRECISION
    pub imf_factor: u32,
    // A fee applied to the collateral when the vault is liquidated, incentivizing users to maintain sufficient collateral.
    pub liquidation_penalty: u32,
    /// The fee the liquidator is paid for liquidating a Vault
    /// precision: LIQUIDATOR_FEE_PRECISION
    pub liquidator_fee: u32,
    /// The fee the insurance fund receives from liquidation
    /// precision: LIQUIDATOR_FEE_PRECISION
    pub if_liquidation_fee: u32,
    /// The margin ratio which determines how much collateral is required to open a position
    /// e.g. margin ratio of .1 means a user must have $100 of total collateral to open a $1000 position
    /// precision: MARGIN_PRECISION
    pub margin_ratio_initial: u32,
    /// The margin ratio which determines when a user will be liquidated
    /// e.g. margin ratio of .05 means a user must have $50 of total collateral to maintain a $1000 position
    /// else they will be liquidated
    /// precision: MARGIN_PRECISION
    pub margin_ratio_maintenance: u32,

    /// maximum amount of synthetic tokens that can be minted against the market's collateral
    pub debt_ceiling: u128,
    /// minimum amount of synthetic tokens that can be minted against a user's collateral to avoid inefficiencies
    pub debt_floor: u32,

    // Oracle
    //
    /// the oracle provider information. used to decode/scale the oracle public key
    pub oracle_source: OracleSource,
    /// stores historically witnessed oracle data
    pub historical_oracle_data: HistoricalOracleData,
    /// the pct size of the oracle confidence interval
    /// precision: PERCENTAGE_PRECISION
    pub last_oracle_conf_pct: u64,
    /// tracks whether the oracle was considered valid at the last AMM update
    pub last_oracle_valid: bool,
    /// the last seen oracle price partially shrunk toward the amm reserve price
    /// precision: PRICE_PRECISION
    pub last_oracle_normalised_price: i64,
    /// the gap between the oracle price and the reserve price = y * peg_multiplier / x
    pub last_oracle_reserve_price_spread_pct: i64,
    /// estimate of standard deviation of the oracle price at each update
    /// precision: PRICE_PRECISION
    pub oracle_std: u64,

    /// The total balance lent to 3rd party protocols
    pub collateral_loan_balance: u64,

    /// the ratio of collateral value to debt value, which must remain above the liquidation ratio.
    pub collateralization_ratio: u64,
    /// the debt created by minting synthetic against the collateral.
    pub synthetic_tokens_minted: u64,

    // Collateral / Liquidations
    //
    // Mint for the collateral token
    pub token_collateral: Address,

    ///
    pub collateral_lending_utilization: u64,

    // Insurance
    //
    /// The market's claim on the insurance fund
    pub insurance_claim: InsuranceClaim,
    /// The total socialized loss from borrows, in the mint's token
    /// precision: token mint precision
    pub total_gov_token_inflation: u128,

    /// Auction Config
    ///
    /// where collateral auctions should take place (3rd party AMM vs private)
    pub collateral_action_config: AuctionConfig,

    // Metrics
    //
    // Total synthetic token debt
    pub outstanding_debt: u128,
    // Unbacked synthetic tokens (result of collateral auction deficits)
    pub protocol_debt: u64,
}

impl SynthMarket {
    pub fn new(params: SynthMarketParams) -> Self {
        SynthMarket {
            oracle,
            token: Address,
            amm,

            name,
            /// The market's token decimals. To from decimals to a precision, 10^decimals
            decimals: u32,
            status: if active_status {
                MarketStatus::Active
            } else {
                MarketStatus::Initialized
            },
            synth_tier,
            paused_operations: 0,
            number_of_users: 0,
            collateral_balance: 0,
            debt_balance: 0,
            cumulative_deposit_interest: 0,
            cumulative_lp_interest: 0,
            withdraw_guard_threshold: 0,
            max_token_deposits: 0,
            collateral_token_twap: 0,
            debt_token_twap: 0,
            utilization_twap: 0,
            last_interest_ts: u64,
            last_twap_ts: u64,
            expiry_timestamp: i64,
            expiry_price: i64,
            max_position_size: u64,
            next_deposit_record_id: u64,
            initial_asset_weight: u32,
            maintenance_asset_weight: u32,
            initial_liability_weight: u32,
            maintenance_liability_weight: u32,
            imf_factor: u32,
            liquidation_penalty: u32,
            liquidator_fee: u32,
            if_liquidation_fee: u32,
            margin_ratio_initial: u32,
            margin_ratio_maintenance: u32,

            debt_ceiling: u128,
            debt_floor: u32,

            oracle_source: OracleSource,
            historical_oracle_data: HistoricalOracleData,
            last_oracle_conf_pct: u64,
            last_oracle_valid: bool,
            last_oracle_normalised_price: i64,
            last_oracle_reserve_price_spread_pct: i64,
            oracle_std: u64,

            collateral_loan_balance: 0,

            collateralization_ratio: 0,
            synthetic_tokens_minted: 0,
            token_collateral: Address,

            collateral_lending_utilization: 0,

            insurance_claim: InsuranceClaim::default(),
            total_gov_token_inflation: u128,

            collateral_action_config: AuctionConfig::default(),

            outstanding_debt: 0,
            protocol_debt: 0,
        }
    }

    pub fn is_in_settlement(&self, now: i64) -> bool {
        let in_settlement = matches!(
            self.status,
            MarketStatus::Settlement | MarketStatus::Delisted
        );
        let expired = self.expiry_ts != 0 && now >= self.expiry_ts;
        in_settlement || expired
    }

    pub fn is_reduce_only(&self) -> bool {
        Ok(self.status == MarketStatus::ReduceOnly)
    }

    pub fn is_operation_paused(&self, operation: Operation) -> bool {
        Operation::is_operation_paused(self.paused_operations, operation)
    }

    pub fn get_max_confidence_interval_multiplier(self) -> u64 {
        // assuming validity_guard_rails max confidence pct is 2%
        match self.synth_tier {
            SynthTier::A => 1,                  // 2%
            SynthTier::B => 1,                  // 2%
            SynthTier::C => 2,                  // 4%
            SynthTier::Speculative => 10,       // 20%
            SynthTier::HighlySpeculative => 50, // 100%
            SynthTier::Isolated => 50,          // 100%
        }
    }

    pub fn get_sanitize_clamp_denominator(self) -> Option<i64> {
        match self.synth_tier {
            SynthTier::A => Some(10_i64),         // 10%
            SynthTier::B => Some(5_i64),          // 20%
            SynthTier::C => Some(2_i64),          // 50%
            SynthTier::Speculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            SynthTier::HighlySpeculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            SynthTier::Isolated => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
        }
    }

    pub fn get_auction_end_min_max_divisors(self) -> (u64, u64) {
        match self.synth_tier {
            SynthTier::A => (1000, 50),              // 10 bps, 2%
            SynthTier::B => (1000, 20),              // 10 bps, 5%
            SynthTier::C => (500, 20),               // 50 bps, 5%
            SynthTier::Speculative => (100, 10),     // 1%, 10%
            SynthTier::HighlySpeculative => (50, 5), // 2%, 20%
            SynthTier::Isolated => (50, 5),          // 2%, 20%
        }
    }

    pub fn get_max_price_divergence_for_funding_rate(
        self,
        oracle_price_twap: i64,
    ) -> i64 {
        // clamp to to 3% price divergence for safer markets and higher for lower contract tiers
        if self.synth_tier.is_as_safe_as_synth(&SynthTier::B) {
            oracle_price_twap.safe_div(33) // 3%
        } else if self.synth_tier.is_as_safe_as_synth(&SynthTier::C) {
            oracle_price_twap.safe_div(20) // 5%
        } else {
            oracle_price_twap.safe_div(10) // 10%
        }
    }

    pub fn get_margin_ratio(&self, size: u128, margin_type: MarginRequirementType) -> u32 {
        if self.status == MarketStatus::Settlement {
            return Ok(0); // no liability weight on size
        }

        let default_margin_ratio = match margin_type {
            MarginRequirementType::Initial => self.margin_ratio_initial,
            // MarginRequirementType::Fill => {
            // 	self.margin_ratio_initial.safe_add(self.margin_ratio_maintenance)? / 2
            // }
            MarginRequirementType::Maintenance => self.margin_ratio_maintenance,
        };

        let size_adj_margin_ratio = calculate_size_premium_liability_weight(
            size,
            self.imf_factor,
            default_margin_ratio,
            MARGIN_PRECISION_U128
        )?;

        let margin_ratio = default_margin_ratio.max(size_adj_margin_ratio);

        margin_ratio
    }

    pub fn get_max_liquidation_fee(&self) -> u32 {
        let max_liquidation_fee = self.liquidator_fee
            .safe_mul(MAX_LIQUIDATION_MULTIPLIER)?
            .min(
                self.margin_ratio_maintenance
                    .safe_mul(LIQUIDATION_FEE_PRECISION)?
                    .safe_div(MARGIN_PRECISION)?
            );
        max_liquidation_fee
    }

    // TODO: rework for AMM swap price change
    pub fn is_price_divergence_ok_for_settle_pnl(&self, oracle_price: i64) -> DriftResult<bool> {
        let oracle_divergence = oracle_price
            .safe_sub(self.amm.historical_oracle_data.last_oracle_price_twap_5min)?
            .safe_mul(PERCENTAGE_PRECISION_I64)?
            .safe_div(
                self.amm
                    .historical_oracle_data
                    .last_oracle_price_twap_5min
                    .min(oracle_price),
            )?
            .unsigned_abs();

        let oracle_divergence_limit = match self.contract_tier {
            ContractTier::A => PERCENTAGE_PRECISION_U64 / 200, // 50 bps
            ContractTier::B => PERCENTAGE_PRECISION_U64 / 200, // 50 bps
            ContractTier::C => PERCENTAGE_PRECISION_U64 / 100, // 100 bps
            ContractTier::Speculative => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
            ContractTier::HighlySpeculative => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
            ContractTier::Isolated => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
        };

        if oracle_divergence >= oracle_divergence_limit {
            msg!(
                "market_index={} price divergence too large to safely settle pnl: {} >= {}",
                self.market_index,
                oracle_divergence,
                oracle_divergence_limit
            );
            return Ok(false);
        }

        let min_price =
            oracle_price.min(self.amm.historical_oracle_data.last_oracle_price_twap_5min);

        let std_limit = match self.contract_tier {
            ContractTier::A => min_price / 50,                 // 200 bps
            ContractTier::B => min_price / 50,                 // 200 bps
            ContractTier::C => min_price / 20,                 // 500 bps
            ContractTier::Speculative => min_price / 10,       // 1000 bps
            ContractTier::HighlySpeculative => min_price / 10, // 1000 bps
            ContractTier::Isolated => min_price / 10,          // 1000 bps
        }
        .unsigned_abs();

        if self.amm.oracle_std.max(self.amm.mark_std) >= std_limit {
            msg!(
                "market_index={} std too large to safely settle pnl: {} >= {}",
                self.market_index,
                self.amm.oracle_std.max(self.amm.mark_std),
                std_limit
            );
            return Ok(false);
        }

        Ok(true)
    }

    pub fn get_open_interest(&self) -> u128 {
        self.amm
            .base_asset_amount_long
            .abs()
            .max(self.amm.base_asset_amount_short.abs())
            .unsigned_abs()
    }
}

pub fn save_market(env: &Env, market: SynthMarket) {
    env.storage().persistent().set(&DataKey::SynthMarket, &market);
    env.storage().persistent().extend_ttl(
        &DataKey::SynthMarket,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn get_market(env: &Env) -> SynthMarket {
    let market = env
        .storage()
        .persistent()
        .get(&DataKey::SynthMarket)
        .expect("SynthMarket not set");

    env.storage().persistent().extend_ttl(
        &DataKey::SynthMarket,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );

    market
}

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum SynthTier {
    /// max insurance capped at A level
    A,
    /// max insurance capped at B level
    B,
    /// max insurance capped at C level
    C,
    /// no insurance
    Speculative,
    /// no insurance, another tranches below
    #[default]
    HighlySpeculative,
    /// no insurance, only single position allowed
    Isolated,
}

impl SynthTier {
    pub fn is_as_safe_as_synth(&self, other: &SynthTier) -> bool {
        // Synth Tier A safest
        self <= other
    }
}

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum Operation {
    Create,
    Deposit,
    Withdraw,
    Lend,
    Transfer,
    Delete,
    Liquidation,
}

// ################################################################
//                             POSITION
// ################################################################

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum PositionStatus {
    Active = 0,
    BeingLiquidated = 1,
    Bankrupt = 2,
    ReduceOnly = 3,
}

#[contracttype]
#[derive(Default, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Position {
    /// The scaled balance of the position. To get the token amount, multiply by the cumulative deposit/borrow
    /// interest of corresponding market.
    /// precision: SPOT_BALANCE_PRECISION
    pub scaled_balance: u64,
    /// The cumulative deposits a user has made into a market
    /// precision: token mint precision
    pub cumulative_deposits: i64,
    pub cumulative_withdrawals: i64,

    /// ----

    /// Whether the user is active, being liquidated or bankrupt
    pub status: u8,
    /// The total values of deposits the user has made
    /// precision: QUOTE_PRECISION
    pub total_deposits: u64,
    /// The total values of withdrawals the user has made
    /// precision: QUOTE_PRECISION
    pub total_withdraws: u64,
    /// The amount of margin freed during liquidation. Used to force the liquidation to occur over a period of time
    /// Defaults to zero when not being liquidated
    /// precision: QUOTE_PRECISION
    pub liquidation_margin_freed: u64,
    /// Custom max initial margin ratio for the user
    pub max_margin_ratio: u32,
    /// The next liquidation id to be used for user
    pub next_liquidation_id: u16,
}

impl Position {

    pub fn is_open_position(&self) -> bool {
        self.cumulative_deposits != 0
    }

    pub fn is_being_liquidated(&self) -> bool {
        self.status & ((PositionStatus::BeingLiquidated as u8) | (PositionStatus::Bankrupt as u8)) >
            0
    }

    pub fn is_bankrupt(&self) -> bool {
        self.status & (PositionStatus::Bankrupt as u8) > 0
    }

    pub fn is_reduce_only(&self) -> bool {
        self.status & (PositionStatus::ReduceOnly as u8) > 0
    }

    pub fn add_position_status(&mut self, status: PositionStatus) {
        self.status |= status as u8;
    }

    pub fn remove_user_status(&mut self, status: PositionStatus) {
        self.status &= !(status as u8);
    }

    pub fn increment_total_deposits(
        &mut self,
        amount: u64,
        price: i64,
        precision: u128
    ) -> NormalResult {
        let value = self.get_deposit_value(amount, price, precision);
        self.total_deposits = self.total_deposits.saturating_add(value);

        Ok(())
    }

    pub fn increment_total_withdraws(
        &mut self,
        amount: u64,
        price: i64,
        precision: u128
    ) -> NormalResult {
        let value = amount
            .cast::<u128>()?
            .safe_mul(price.cast()?)?
            .safe_div(precision)?
            .cast::<u64>()?;
        self.total_withdraws = self.total_withdraws.saturating_add(value);

        Ok(())
    }

    pub fn enter_liquidation(&mut self) -> NormalResult<u16> {
        if self.is_being_liquidated() {
            return self.next_liquidation_id.safe_sub(1);
        }

        self.add_position_status(PositionStatus::BeingLiquidated);
        self.liquidation_margin_freed = 0;
        Ok(get_then_update_id!(self, next_liquidation_id))
    }

    pub fn exit_liquidation(&mut self) {
        self.remove_user_status(PositionStatus::BeingLiquidated);
        self.remove_user_status(PositionStatus::Bankrupt);
        self.liquidation_margin_freed = 0;
    }

    pub fn enter_bankruptcy(&mut self) {
        self.remove_user_status(PositionStatus::BeingLiquidated);
        self.add_position_status(PositionStatus::Bankrupt);
    }

    pub fn exit_bankruptcy(&mut self) {
        self.remove_user_status(PositionStatus::BeingLiquidated);
        self.remove_user_status(PositionStatus::Bankrupt);
        self.liquidation_margin_freed = 0;
    }

    pub fn increment_margin_freed(&mut self, margin_free: u64) -> NormalResult {
        self.liquidation_margin_freed = self.liquidation_margin_freed.safe_add(margin_free)?;
        Ok(())
    }

    pub fn update_reduce_only_status(&mut self, reduce_only: bool) -> NormalResult {
        if reduce_only {
            self.add_position_status(PositionStatus::ReduceOnly);
        } else {
            self.remove_user_status(PositionStatus::ReduceOnly);
        }

        Ok(())
    }

    pub fn calculate_margin(
        &mut self,

        context: MarginContext,
        now: i64
    ) -> NormalResult<MarginCalculation> {
        let margin_calculation =
            calculate_margin_requirement_and_total_collateral_and_liability_info(self, context)?;

        Ok(margin_calculation)
    }

    pub fn meets_withdraw_margin_requirement(
        &mut self,

        margin_requirement_type: MarginRequirementType,
        withdraw_market_index: u16,
        withdraw_amount: u128,
        now: i64
    ) -> NormalResult<bool> {
        let strict = margin_requirement_type == MarginRequirementType::Initial;
        let context = MarginContext::standard(margin_requirement_type).strict(strict);

        let calculation = calculate_margin_requirement_and_total_collateral_and_liability_info(
            self,
            context
        )?;

        if calculation.margin_requirement > 0 || calculation.get_num_of_liabilities()? > 0 {
            validate!(
                calculation.all_oracles_valid,
                ErrorCode::InvalidOracle,
                "User attempting to withdraw with outstanding liabilities when an oracle is invalid"
            )?;
        }

        validate_any_isolated_tier_requirements(self, calculation)?;

        validate!(
            calculation.meets_margin_requirement(),
            ErrorCode::InsufficientCollateral,
            "User attempting to withdraw where total_collateral {} is below initial_margin_requirement {}",
            calculation.total_collateral,
            calculation.margin_requirement
        )?;

        Ok(true)
    }
}

pub fn get_position(env: &Env, key: &Address) -> Position {
    let position_info = match env.storage().persistent().get::<_, Position>(key) {
        Some(position) => position,
        None =>
            Position {
                stakes: Vec::new(env),
                reward_debt: 0u128,
                last_reward_time: 0u64,
                total_stake: 0i128,
            },
    };
    env.storage()
        .persistent()
        .has(&key)
        .then(|| {
            env.storage()
                .persistent()
                .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        });

    position_info
}
// ################################################################
