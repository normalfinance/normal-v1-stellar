use soroban_decimal::Decimal;
use soroban_sdk::{ contracttype, Address };

// Validate if int value is bigger then 0
#[macro_export]
macro_rules! validate_int_parameters {
    ($($arg:expr),*) => {
        {
            $(
                let value: Option<i128> = Into::<Option<_>>::into($arg);
                if let Some(val) = value {
                    if val <= 0 {
                        panic!("value cannot be less than or equal zero")
                    }
                }
            )*
        }
    };
}

// Validate all bps to be between the range 0..10_000
#[macro_export]
macro_rules! validate_bps {
    ($($value:expr),+) => {
        const MIN_BPS: i64 = 0;
        const MAX_BPS: i64 = 10_000;
        $(
            // if $value < MIN_BPS || $value > MAX_BPS {
            //     panic!("The value {} is out of range. Must be between {} and {} bps.", $value, MIN_BPS, MAX_BPS);
            // }
            assert!((MIN_BPS..=MAX_BPS).contains(&$value), "The value {} is out of range. Must be between {} and {} bps.", $value, MIN_BPS, MAX_BPS);
        )+
    };
}

pub fn is_approx_ratio(a: Decimal, b: Decimal, tolerance: Decimal) -> bool {
    let diff = (a - b).abs();
    diff <= tolerance
}

pub fn convert_i128_to_u128(input: i128) -> u128 {
    if input < 0 {
        panic!("Cannot convert i128 to u128");
    } else {
        input as u128
    }
}

pub fn convert_u128_to_i128(input: u128) -> i128 {
    if input > (i128::MAX as u128) {
        panic!("Cannot convert u128 to i128");
    } else {
        input as i128
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, Default)]
pub enum MarketStatus {
    /// warm up period for initialization, swapping is paused
    #[default]
    Initialized,
    /// all operations allowed
    Active,
    /// swaps only able to reduce liability
    ReduceOnly,
    /// market has determined settlement price and positions are expired must be settled
    Settlement,
    /// market has no remaining participants
    Delisted,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexAssetInfo {
    /// Address of the synth market
    pub market_address: Address,
    /// The portfolio allocation of the asset
    pub weight: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AMMParams {
    pub admin: Address,
    pub tick_spacing: u16,
    pub initial_sqrt_price: u128,
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub swap_fee_bps: i64,
    pub max_allowed_slippage_bps: i64,
    pub default_slippage_bps: i64,
    pub max_allowed_spread_bps: i64,
    pub token_init_info: TokenInitInfo,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexParams {
    /// The owner/authority of the index
    pub admin: Address,
    /// An address that can control the index on the admin's behalf. Has limited power, cant withdraw
    pub delegate: Address,
    /// An address that has the limited power to manage fees (such as updating and collecting them)
    pub fee_authority: Address,
    /// An address that has the limited power to update access control (such as the whitelist and blacklist)
    pub access_authority: Address,
    /// An address that has the limited power to manage assets and weights
    pub rebalance_authority: Address,
    /// Encoded display name for the index
    pub name: [u32; 8],
    pub symbol: [u32; 8],
    /// Private indexes can be updated, but are only mintable by the admin and whitelist
    /// Public indexes cannot be updated, but can be minted by anyone
    pub is_public: bool,
    pub assets: Vec<IndexAssetInfo>,
    /// Whether a market is active, reduce only, expired, etc
    /// Affects whether users can open/close positions
    pub status: MarketStatus,
    pub paused_operations: u8,
    pub manager_fee_bps: i64,
    pub revenue_share_bps: i64,
    /// List of accounts explicitly allowed to mint the index
    pub whitelist: Vec<Pubkey>,
    /// List of accounts blocked from minting the index
    pub blacklist: Vec<Pubkey>,
    /// The ts when the index will be expired. Only set if index is in reduce only mode
    pub expiry_ts: i64,
    /// The price at which tokens will be redeemed. Only set if index is expired
    /// precision = PRICE_PRECISION
    pub expiry_price: i64,
}
