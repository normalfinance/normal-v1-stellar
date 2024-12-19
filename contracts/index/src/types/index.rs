pub struct IndexMarket {
    /// The index market's address. It is a pda of the market index
    pub pubkey: Pubkey,
    /// The owner/authority of the account
    pub authority: Pubkey,
    /// An addresses that can control the account on the authority's behalf. Has limited power, cant withdraw
    pub delegate: Pubkey,

    pub vault: Pubkey,
    pub token_mint: Pubkey,

    pub fee_authority: Address,
    pub whitelist_authority: Address,
    pub rebalance_authority: Address,

    pub market_index: u16,
    /// Encoded display name for the market e.g. BTC-SOL
    pub name: [u8; 32],
    /// Whether a market is active, reduce only, expired, etc
    /// Affects whether users can open/close positions
    pub status: MarketStatus,
    pub paused_operations: u8,
    pub number_of_users: u32,

    // Oracle
    //
    pub oracle: Pubkey,
    pub oracle_source: OracleSource,

    /// Index
    ///
    pub assets: Vec<IndexAsset>,
    /// The visibility of the index fund
    pub visibility: IndexVisibility,
    /// List of accounts allowed to purchase the index
    pub whitelist: Vec<Pubkey>,
    pub blacklist: Vec<Pubkey>,

    /// Fees
    ///
    /// Total taker fee paid in basis points
    pub expense_ratio: u64,
    ///
    pub revenue_share: u64,
    pub protocol_fee_owed: u64,
    pub manager_fee_owed: u64,
    pub referral_fee_owed: u64,
    pub total_fees: u64,

    // Metrics
    //
    pub total_minted: u64,
    pub total_redeemed: i64,

    // Shutdown
    //
    /// The ts when the market will be expired. Only set if market is in reduce only mode
    pub expiry_ts: i64,
    /// The price at which positions will be settled. Only set if market is expired
    /// precision = PRICE_PRECISION
    pub expiry_price: i64,

    // Timestamps
    pub rebalanced_ts: i64,
    pub updated_ts: i64,
}
