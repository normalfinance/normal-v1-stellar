use normal::{
    constants::{
        LIQUIDATION_FEE_PRECISION, MARGIN_PRECISION, MARGIN_PRECISION_U128,
        MAX_LIQUIDATION_MULTIPLIER, PERCENTAGE_PRECISION_I64, PERCENTAGE_PRECISION_U64,
        PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD,
    },
    math::{casting::Cast, safe_math::SafeMath},
    oracle::OracleSource,
    types::{auction::Auction, market::SynthTier},
    validate,
};
use soroban_sdk::{contracttype, log, Address, Env, String, Symbol, Vec};

use crate::{
    math::{
        balance::{get_token_amount, BalanceType},
        margin::{calculate_size_premium_liability_weight, MarginRequirementType},
    },
    storage::DataKey,
};

use super::pool::Pool;

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub enum MarketOperation {
    Create,
    Deposit,
    Withdraw,
    Borrow,
    Repay,
    Lend,
    Transfer,
    Delete,
    Liquidation,
}

const ALL_MARKET_OPERATIONS: [MarketOperation; 7] = [
    MarketOperation::Create,
    MarketOperation::Deposit,
    MarketOperation::Withdraw,
    MarketOperation::Lend,
    MarketOperation::Transfer,
    MarketOperation::Delete,
    MarketOperation::Liquidation,
];

impl MarketOperation {
    pub fn is_operation_paused(current: Vec<MarketOperation>, operation: MarketOperation) -> bool {
        // (current & (operation as u8)) != 0
        current.contains(operation)
    }

    pub fn log_all_operations_paused(env: &Env, current: Vec<MarketOperation>) {
        for operation in ALL_MARKET_OPERATIONS.iter() {
            if Self::is_operation_paused(current, *operation) {
                log!(env, "{:?} is paused", operation);
            }
        }
    }
}

#[contracttype]
#[derive(Clone, Copy, PartialEq, Debug, Eq, Default)]
pub enum MarketStatus {
    /// warm up period for initialization, fills are paused
    #[default]
    Initialized,
    /// all operations allowed
    Active,
    /// fills only able to reduce liability
    ReduceOnly,
    /// market has determined settlement price and positions are expired must be settled
    Settlement,
    /// market has no remaining participants
    Delisted,
}

#[contracttype]
#[derive(Default, Eq, PartialEq, Debug)]
#[repr(C)]
pub struct InsuranceClaim {
    /// The amount of revenue last settled
    /// Positive if funds left the perp market,
    /// negative if funds were pulled into the perp market
    /// precision: QUOTE_PRECISION
    pub rev_withdraw_since_last_settle: i64,
    /// The max amount of revenue that can be withdrawn per period
    /// precision: QUOTE_PRECISION
    pub max_rev_withdraw_per_period: u64,
    /// The max amount of insurance that perp market can use to resolve bankruptcy and pnl deficits
    /// precision: QUOTE_PRECISION
    pub quote_max_insurance: u64,
    /// The amount of insurance that has been used to resolve bankruptcy and pnl deficits
    /// precision: QUOTE_PRECISION
    pub quote_settled_insurance: u64,
    /// The last time revenue was settled in/out of market
    pub last_revenue_withdraw_ts: i64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collateral {
    pub symbol: Symbol,
    pub token: Address,
    pub oracle: Address,
    /// the oracle provider information. used to decode/scale the oracle data
    pub oracle_source: OracleSource,
    pub oracle_frozen: bool,
    /// The sum of the balances for collateral deposits across users
    /// precision: SPOT_BALANCE_PRECISION
    pub balance: u128,
    /// The amount of collateral sent/received with the Pool to adjust price
    pub pool_delta_balance: i128,
    /// 24hr average of deposit token amount
    /// precision: token mint precision
    pub token_twap: u64,
    /// The margin ratio which determines how much collateral is required to open a position
    /// e.g. margin ratio of .1 means a user must have $100 of total collateral to open a $1000 position
    /// precision: MARGIN_PRECISION
    pub margin_ratio_initial: u32,
    /// The margin ratio which determines when a user will be liquidated
    /// e.g. margin ratio of .05 means a user must have $50 of total collateral to maintain a $1000 position
    /// else they will be liquidated
    /// precision: MARGIN_PRECISION
    pub margin_ratio_maintenance: u32,
    /// where collateral auctions should take place (3rd party AMM vs private)
    pub auction_config: Auction,
    /// The max amount of token deposits in this market
    /// 0 if there is no limit
    /// precision: token mint precision
    pub max_token_deposits: u64,
    /// What fraction of max_token_deposits
    /// disabled when 0, 1 => 1/10000 => .01% of max_token_deposits
    /// precision: X/10000
    pub max_token_borrows_fraction: u32,
    /// no withdraw limits/guards when deposits below this threshold
    /// precision: token mint precision
    pub withdraw_guard_threshold: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Synthetic {
    pub symbol: Symbol,
    pub token: Address,
    /// The synthetic tier determines how much insurance a market can receive, with more speculative markets receiving less insurance
    /// It also influences the order markets can be liquidated, with less speculative markets being liquidated first
    pub tier: SynthTier,
    /// The sum of the balances for synthetic debts across users
    /// precision: SPOT_BALANCE_PRECISION
    pub balance: u128,
    /// 24hr average of synthetic token amount
    /// precision: token mint precision
    pub token_twap: u64,
    /// The maximum position size
    /// if the limit is 0, there is no limit
    /// precision: token mint precision
    pub max_position_size: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Market {
    /// Encoded display name for the market e.g. BTC-XLM
    pub name: String,
    pub collateral: Collateral,
    pub synthetic: Synthetic,
    pub amm: Pool,
    /// The market's token decimals. To from decimals to a precision, 10^decimals
    pub decimals: u32,
    /// Whether a market is active, reduce only, expired, etc
    /// Affects whether users can open/close positions
    pub status: MarketStatus,
    pub paused_operations: Vec<MarketOperation>,

    /// 24hr average of utilization
    /// which is debt amount over collateral amount
    /// precision: SPOT_UTILIZATION_PRECISION
    pub utilization_twap: u64,
    /// Last time the deposit/borrow/utilization averages were updated
    pub last_twap_ts: u64,

    /// The optimatal AMM position to deposit new liquidity into
    pub lp_ts: u64,
    pub last_lp_rebalance_ts: u64,

    /// The ts when the market will be expired. Only set if market is in reduce only mode
    pub expiry_ts: u64,
    /// The price at which positions will be settled. Only set if market is expired
    /// precision = PRICE_PRECISION
    pub expiry_price: i64,

    /// Every deposit has a deposit record id. This is the next id to use
    pub next_deposit_record_id: u64,
    /// The next liquidation id to be used for user
    pub next_liquidation_id: u32,

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

    /// maximum amount of synthetic tokens that can be minted against the market's collateral
    pub debt_ceiling: u128,
    /// minimum amount of synthetic tokens that can be minted against a user's collateral to avoid inefficiencies
    pub debt_floor: u32,

    pub insurance: Address,
    /// The market's claim on the insurance fund
    pub insurance_claim: InsuranceClaim,

    // Unbacked synthetic tokens (result of collateral auction deficits)
    pub protocol_debt: u64,
}

impl Market {
    // spot

    pub fn get_precision(self) -> u64 {
        (10_u64).pow(self.decimals)
    }

    // other

    pub fn is_in_settlement(&self, now: i64) -> bool {
        let in_settlement = matches!(
            self.status,
            MarketStatus::Settlement | MarketStatus::Delisted
        );
        let expired = self.expiry_ts != 0 && now >= self.expiry_ts;
        in_settlement || expired
    }

    pub fn is_reduce_only(&self) -> bool {
        self.status == MarketStatus::ReduceOnly
    }

    pub fn is_operation_paused(&self, operation: MarketOperation) -> bool {
        MarketOperation::is_operation_paused(self.paused_operations, operation)
    }

    pub fn get_max_confidence_interval_multiplier(self) -> u64 {
        // assuming validity_guard_rails max confidence pct is 2%
        match self.synthetic.tier {
            SynthTier::A => 1,                  // 2%
            SynthTier::B => 1,                  // 2%
            SynthTier::C => 2,                  // 4%
            SynthTier::Speculative => 10,       // 20%
            SynthTier::HighlySpeculative => 50, // 100%
            SynthTier::Isolated => 50,          // 100%
        }
    }

    pub fn get_sanitize_clamp_denominator(self) -> Option<i64> {
        match self.synthetic.tier {
            SynthTier::A => Some(10_i64),         // 10%
            SynthTier::B => Some(5_i64),          // 20%
            SynthTier::C => Some(2_i64),          // 50%
            SynthTier::Speculative => None,       // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            SynthTier::HighlySpeculative => None, // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
            SynthTier::Isolated => None,          // DEFAULT_MAX_TWAP_UPDATE_PRICE_BAND_DENOMINATOR
        }
    }

    pub fn get_auction_end_min_max_divisors(self) -> (u64, u64) {
        match self.synthetic.tier {
            SynthTier::A => (1000, 50),              // 10 bps, 2%
            SynthTier::B => (1000, 20),              // 10 bps, 5%
            SynthTier::C => (500, 20),               // 50 bps, 5%
            SynthTier::Speculative => (100, 10),     // 1%, 10%
            SynthTier::HighlySpeculative => (50, 5), // 2%, 20%
            SynthTier::Isolated => (50, 5),          // 2%, 20%
        }
    }

    pub fn get_margin_ratio(&self, size: u128, margin_type: MarginRequirementType) -> u32 {
        if self.status == MarketStatus::Settlement {
            return Ok(0); // no liability weight on size
        }

        let default_margin_ratio = match margin_type {
            MarginRequirementType::Initial => self.margin_ratio_initial,
            MarginRequirementType::Fill => {
                self.margin_ratio_initial
                    .safe_add(self.margin_ratio_maintenance)?
                    / 2
            }
            MarginRequirementType::Maintenance => self.margin_ratio_maintenance,
        };

        let size_adj_margin_ratio = calculate_size_premium_liability_weight(
            size,
            self.imf_factor,
            default_margin_ratio,
            MARGIN_PRECISION_U128,
        );

        let margin_ratio = default_margin_ratio.max(size_adj_margin_ratio);

        margin_ratio
    }

    pub fn get_collateral(&self, env: &Env) -> u128 {
        get_token_amount(env, self.collateral.balance, self, &BalanceType::Deposit)
    }

    pub fn get_debt(&self, env: &Env) -> u128 {
        get_token_amount(env, self.synthetic.balance, self, &BalanceType::Borrow)
    }

    pub fn get_utilization(&self, env: &Env) -> u128 {
        get_token_amount(env, self.synthetic.balance, self, &BalanceType::Borrow)
    }

    pub fn validate_max_token_deposits_and_borrows(&self, env: &Env, do_max_borrow_check: bool) {
        let deposits = self.get_collateral(env);
        let max_token_deposits = self.collateral.max_token_deposits.cast::<u128>(env);

        validate!(
            env,
            max_token_deposits == 0 || deposits <= max_token_deposits,
            Errors::MaxDeposit,
            "max token amount ({}) < deposits ({})",
            max_token_deposits,
            deposits
        );

        if do_max_borrow_check && self.max_token_borrows_fraction > 0 && self.max_token_deposits > 0
        {
            let borrows = self.get_debt(env);
            let max_token_borrows = self
                .max_token_deposits
                .safe_mul(self.max_token_borrows_fraction.cast()?)?
                .safe_div(10000)?
                .cast::<u128>()?;

            validate!(
                env,
                max_token_borrows == 0 || borrows <= max_token_borrows,
                Errors::MaxBorrows,
                "max token amount ({}) < borrows ({})",
                max_token_borrows,
                borrows
            );
        }
    }

    pub fn get_max_liquidation_fee(&self, env: &Env) -> u32 {
        let max_liquidation_fee = self
            .liquidator_fee
            .safe_mul(MAX_LIQUIDATION_MULTIPLIER, env)
            .min(
                self.margin_ratio_maintenance
                    .safe_mul(LIQUIDATION_FEE_PRECISION)
                    .safe_div(MARGIN_PRECISION),
            );
        max_liquidation_fee
    }

    // TODO: rework for AMM swap price change
    // pub fn is_price_divergence_ok(&self, env: &Env, oracle_price: i64) -> bool {
    //     let oracle_divergence = oracle_price
    //         .safe_sub(self.amm.historical_oracle_data.last_oracle_price_twap, env)
    //         .safe_mul(PERCENTAGE_PRECISION_I64, env)
    //         .safe_div(
    //             self.amm.historical_oracle_data.last_oracle_price_twap_5min.min(oracle_price),
    //             env
    //         )
    //         .unsigned_abs();

    //     let oracle_divergence_limit = match self.contract_tier {
    //         SynthTier::A => PERCENTAGE_PRECISION_U64 / 200, // 50 bps
    //         SynthTier::B => PERCENTAGE_PRECISION_U64 / 200, // 50 bps
    //         SynthTier::C => PERCENTAGE_PRECISION_U64 / 100, // 100 bps
    //         SynthTier::Speculative => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
    //         SynthTier::HighlySpeculative => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
    //         SynthTier::Isolated => PERCENTAGE_PRECISION_U64 / 40, // 250 bps
    //     };

    //     if oracle_divergence >= oracle_divergence_limit {
    //         log!(
    //             env,
    //             "market_name={} price divergence too large to safely settle pnl: {} >= {}",
    //             self.name,
    //             oracle_divergence,
    //             oracle_divergence_limit
    //         );
    //         return false;
    //     }

    //     let min_price = oracle_price.min(
    //         self.amm.historical_oracle_data.last_oracle_price_twap_5min
    //     );

    //     let std_limit = (
    //         match self.tier {
    //             SynthTier::A => min_price / 50, // 200 bps
    //             SynthTier::B => min_price / 50, // 200 bps
    //             SynthTier::C => min_price / 20, // 500 bps
    //             SynthTier::Speculative => min_price / 10, // 1000 bps
    //             SynthTier::HighlySpeculative => min_price / 10, // 1000 bps
    //             SynthTier::Isolated => min_price / 10, // 1000 bps
    //         }
    //     ).unsigned_abs();

    //     if self.amm.oracle_std.max(self.amm.mark_std) >= std_limit {
    //         log!(
    //             env,
    //             "market_name={} std too large to safely settle pnl: {} >= {}",
    //             self.name,
    //             self.amm.oracle_std.max(self.amm.mark_std),
    //             std_limit
    //         );
    //         return false;
    //     }

    //     true
    // }
}

pub fn save_market(env: &Env, market: Market) {
    env.storage().persistent().set(&DataKey::Market, &market);
    env.storage().persistent().extend_ttl(
        &DataKey::Market,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn get_market(env: &Env) -> Market {
    let market = env
        .storage()
        .persistent()
        .get(&DataKey::Market)
        .expect("Market not set");

    env.storage().persistent().extend_ttl(
        &DataKey::Market,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );

    market
}
