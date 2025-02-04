use normal::{
    constants::{PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD},
    error::NormalResult,
};
use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum MarketPositionStatus {
    Active = 0,
    BeingLiquidated = 1,
    Bankrupt = 2,
    ReduceOnly = 3,
}

#[contracttype]
#[derive(Default, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct MarketPosition {
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
    pub status: MarketPositionStatus,
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
    pub next_liquidation_id: u32,
}

impl MarketPosition {
    pub fn is_open_position(&self) -> bool {
        self.cumulative_deposits != 0
    }

    pub fn is_being_liquidated(&self) -> bool {
        self.status
            & ((MarketPositionStatus::BeingLiquidated as u8)
                | (MarketPositionStatus::Bankrupt as u8))
            > 0
    }

    pub fn is_bankrupt(&self) -> bool {
        // self.status & (MarketPositionStatus::Bankrupt as u8) > 0
        self.status == MarketPositionStatus::Bankrupt
    }

    pub fn is_reduce_only(&self) -> bool {
        self.status & (MarketPositionStatus::ReduceOnly as u8) > 0
    }

    pub fn add_position_status(&mut self, status: MarketPositionStatus) {
        self.status |= status as u8;
    }

    pub fn remove_user_status(&mut self, status: MarketPositionStatus) {
        self.status &= !(status as u8);
    }

    pub fn increment_total_deposits(
        &mut self,
        amount: u64,
        price: i64,
        precision: u128,
    ) -> NormalResult {
        let value = self.get_deposit_value(amount, price, precision);
        self.total_deposits = self.total_deposits.saturating_add(value);

        Ok(())
    }

    pub fn increment_total_withdraws(
        &mut self,
        amount: u64,
        price: i64,
        precision: u128,
    ) -> NormalResult {
        let value = amount
            .cast::<u128>()?
            .safe_mul(price.cast()?)?
            .safe_div(precision)?
            .cast::<u64>()?;
        self.total_withdraws = self.total_withdraws.saturating_add(value);

        Ok(())
    }

    pub fn enter_liquidation(&mut self) -> NormalResult<u32> {
        if self.is_being_liquidated() {
            return self.next_liquidation_id.safe_sub(1);
        }

        self.add_position_status(MarketPositionStatus::BeingLiquidated);
        self.liquidation_margin_freed = 0;
        Ok(get_then_update_id!(self, next_liquidation_id))
    }

    pub fn exit_liquidation(&mut self) {
        self.remove_user_status(MarketPositionStatus::BeingLiquidated);
        self.remove_user_status(MarketPositionStatus::Bankrupt);
        self.liquidation_margin_freed = 0;
    }

    pub fn enter_bankruptcy(&mut self) {
        self.remove_user_status(MarketPositionStatus::BeingLiquidated);
        self.add_position_status(MarketPositionStatus::Bankrupt);
    }

    pub fn exit_bankruptcy(&mut self) {
        self.remove_user_status(MarketPositionStatus::BeingLiquidated);
        self.remove_user_status(MarketPositionStatus::Bankrupt);
        self.liquidation_margin_freed = 0;
    }

    pub fn increment_margin_freed(&mut self, margin_free: u64) -> NormalResult {
        self.liquidation_margin_freed = self.liquidation_margin_freed.safe_add(margin_free)?;
        Ok(())
    }

    pub fn update_reduce_only_status(&mut self, reduce_only: bool) -> NormalResult {
        if reduce_only {
            self.add_position_status(MarketPositionStatus::ReduceOnly);
        } else {
            self.remove_user_status(MarketPositionStatus::ReduceOnly);
        }

        Ok(())
    }

    pub fn calculate_margin(
        &mut self,

        context: MarginContext,
        now: i64,
    ) -> NormalResult<MarginCalculation> {
        let margin_calculation =
            calculate_margin_requirement_and_total_collateral_and_liability_info(self, context)?;

        Ok(margin_calculation)
    }

    pub fn meets_withdraw_margin_requirement(
        &mut self,

        margin_requirement_type: MarginRequirementType,
        withdraw_market_index: u32,
        withdraw_amount: u128,
        now: i64,
    ) -> NormalResult<bool> {
        let strict = margin_requirement_type == MarginRequirementType::Initial;
        let context = MarginContext::standard(margin_requirement_type).strict(strict);

        let calculation =
            calculate_margin_requirement_and_total_collateral_and_liability_info(self, context)?;

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

pub fn get_market_position(env: &Env, key: &Address) -> MarketPosition {
    let position = match env.storage().persistent().get::<_, MarketPosition>(key) {
        Some(pos) => pos,
        None => MarketPosition {
            positions: Vec::new(env),
        },
    };
    env.storage().persistent().has(&key).then(|| {
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );
    });

    position
}

pub fn save_market_position(env: &Env, key: &Address, position_info: &MarketPosition) {
    env.storage().persistent().set(key, position_info);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}
