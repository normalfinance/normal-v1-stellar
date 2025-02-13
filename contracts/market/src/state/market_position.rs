use normal::{
    constants::{PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD},
    error::NormalResult,
    math::{casting::Cast, safe_math::SafeMath},
    validate,
};
use soroban_sdk::{contracttype, Address, Env};

use crate::math::{
    balance::{get_signed_token_amount, get_token_amount, BalanceType},
    margin::{
        calculate_margin_requirement_and_total_collateral_and_liability_info,
        validate_any_isolated_tier_requirements, MarginRequirementType,
    },
};

use super::{
    margin_calculation::{MarginCalculation, MarginContext},
    market::Market,
};

#[contracttype]
#[derive(Default, Clone, Copy, PartialEq, Debug, Eq)]
pub enum MarketPositionStatus {
    #[default]
    Active = 0,
    BeingLiquidated = 1,
    Bankrupt = 2,
    ReduceOnly = 3,
}

#[contracttype]
#[derive(Default, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct MarketPosition {
    /// Whether the user is active, being liquidated or bankrupt
    pub status: MarketPositionStatus,

    /// The balance of the position
    /// precision: SPOT_BALANCE_PRECISION
    pub collateral_balance: u128,
    /// The balance of minted synthetic tokens
    pub debt_balance: u128,
    /// The balance of collateral provided as liquidity
    pub collateral_lp_balance: u128,
    /// The balance of collateral lent to money markets
    pub collateral_loan_balance: u128,

    /// The number of lp (liquidity provider) shares the user has in this perp market
    /// LP shares allow users to provide liquidity via the AMM
    /// precision: BASE_PRECISION
    pub lp_shares: u64,

    /// The total values of deposits the user has made
    /// precision: QUOTE_PRECISION
    pub total_deposits: u128,
    /// The total values of withdrawals the user has made
    /// precision: QUOTE_PRECISION
    pub total_withdraws: u128,
    /// The total values of mints the user has made
    /// precision: QUOTE_PRECISION
    pub total_mints: u128,
    /// The total values of burns the user has made
    /// precision: QUOTE_PRECISION
    pub total_burns: u128,

    /// The amount of margin freed during liquidation. Used to force the liquidation to occur over a period of time
    /// Defaults to zero when not being liquidated
    /// precision: QUOTE_PRECISION
    pub liquidation_margin_freed: u128,
    /// Custom max initial margin ratio for the user
    pub max_margin_ratio: u32,

    /// The last slot a user was active. Used to determine if a user is idle
    pub last_active_ts: u64,
    /// User is idle if they haven't interacted with the protocol in 1 week and they have no orders, perp positions or borrows
    /// Off-chain keeper bots can ignore users that are idle
    pub idle: bool,

    /// Whether the position is deposit or borrow
    pub balance_type: BalanceType,
}

impl MarketPosition {
    pub fn is_available(&self) -> bool {
        self.balance == 0
    }

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

    pub fn get_token_amount(&self, env: &Env, market: &Market) -> u128 {
        get_token_amount(
            env,
            self.scaled_balance.cast(env),
            market,
            &self.balance_type,
        )
    }

    pub fn get_signed_token_amount(&self, env: &Env, market: &Market) -> i128 {
        get_signed_token_amount(
            env,
            get_token_amount(
                env,
                self.scaled_balance.cast(env),
                market,
                &self.balance_type,
            ),
            &self.balance_type,
        )
    }

    pub fn increment_total_deposits(
        &mut self,
        env: &Env,
        amount: u64,
        price: i64,
        precision: u128,
    ) {
        let value = amount
            .cast::<u128>(env)
            .safe_mul(price.cast::<u128>(env), env)
            .safe_div(precision, env)
            .cast::<u64>(env);
        self.total_deposits = self.total_deposits.saturating_add(value);
    }

    pub fn increment_total_withdraws(
        &mut self,
        env: &Env,
        amount: u64,
        price: i64,
        precision: u128,
    ) {
        let value = amount
            .cast::<u128>(env)
            .safe_mul(price.cast(env), env)
            .safe_div(precision, env)
            .cast::<u64>(env);
        self.total_withdraws = self.total_withdraws.saturating_add(value);
    }

    pub fn enter_liquidation(&mut self) -> u32 {
        if self.is_being_liquidated() {
            return self.next_liquidation_id.safe_sub(1, env);
        }

        self.add_position_status(MarketPositionStatus::BeingLiquidated);
        self.liquidation_margin_freed = 0;
        get_then_update_id!(self, next_liquidation_id)
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

    pub fn increment_margin_freed(&mut self, margin_free: u64) {
        self.liquidation_margin_freed = self.liquidation_margin_freed.safe_add(margin_free, env);
    }

    pub fn update_last_active_ts(&mut self, ts: u64) {
        if !self.is_being_liquidated() {
            self.last_active_ts = ts;
        }
        self.idle = false;
    }

    pub fn update_reduce_only_status(&mut self, reduce_only: bool) {
        if reduce_only {
            self.add_position_status(MarketPositionStatus::ReduceOnly);
        } else {
            self.remove_user_status(MarketPositionStatus::ReduceOnly);
        }
    }

    pub fn calculate_margin(&mut self, context: MarginContext, now: i64) -> MarginCalculation {
        let margin_calculation =
            calculate_margin_requirement_and_total_collateral_and_liability_info(
                env, &self, context,
            );

        margin_calculation
    }

    pub fn meets_withdraw_margin_requirement(
        &mut self,
        env: &Env,
        margin_requirement_type: MarginRequirementType,
        withdraw_amount: u128,
        now: i64,
    ) -> bool {
        let strict = margin_requirement_type == MarginRequirementType::Initial;
        let context = MarginContext::standard(margin_requirement_type).strict(strict);

        let calculation = calculate_margin_requirement_and_total_collateral_and_liability_info(
            env, &self, context,
        );

        if calculation.margin_requirement > 0 || calculation.get_num_of_liabilities()? > 0 {
            validate!(
                env,
                calculation.all_oracles_valid,
                ErrorCode::InvalidOracle,
                "User attempting to withdraw with outstanding liabilities when an oracle is invalid"
            );
        }

        validate_any_isolated_tier_requirements(env, &self, calculation)?;

        validate!(
            env,
            calculation.meets_margin_requirement(),
            ErrorCode::InsufficientCollateral,
            "User attempting to withdraw where total_collateral {} is below initial_margin_requirement {}",
            calculation.total_collateral,
            calculation.margin_requirement
        );

        true
    }
}

pub fn get_market_position(env: &Env, key: &Address) -> MarketPosition {
    let position = match env.storage().persistent().get::<_, MarketPosition>(key) {
        Some(pos) => pos,
        None => MarketPosition {
            status: MarketPositionStatus::Active,
            scaled_balance: 0u64,
            cumulative_deposits: 0u128,
            cumulative_withdrawals: 0u128,
            last_active_ts: 0u64,
            idle: false,
            total_deposits: 0u64,
            total_withdraws: 0u64,
            liquidation_margin_freed: 0u64,
            max_margin_ratio: 0u32,
            next_liquidation_id: 0u32,
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
