use soroban_sdk::contracttype;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynthMarket {
    /// The market's address. It is a pda of the market index
    pub pubkey: Pubkey,
    /// oracle price data public key
    pub oracle: Pubkey,

    pub market_index: u16,
    /// Encoded display name for the market e.g. BTC-SOL
    pub name: [u8; 32],
    /// The market's token mint's decimals. To from decimals to a precision, 10^decimals
    pub decimals: u32,
    /// Whether a market is active, reduce only, expired, etc
    /// Affects whether users can open/close positions
    pub status: MarketStatus,
    /// The synthetic tier determines how much insurance a market can receive, with more speculative markets receiving less insurance
    /// It also influences the order markets can be liquidated, with less speculative markets being liquidated first
    pub synthetic_tier: SyntheticTier,
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
    pub expiry_ts: i64,
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
    pub token_mint_collateral: Pubkey,
    // Vault storing synthetic tokens from liquidation
    pub token_vault_synthetic: Pubkey,
    // Vault storing collateral tokens for auction
    pub token_vault_collateral: Pubkey,

    ///
    pub collateral_lending_utilization: u64,

    // AMM
    //
    pub amm: Address,

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
