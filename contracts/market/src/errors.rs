// use normal::error::ErrorCode as SharedErrors;
use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
#[allow(clippy::too_many_arguments)]
pub enum Errors {
    AlreadyInitialized = 100,
    NotAuthorized = 2,
    AdminNotSet = 3,
    InvalidMarginRatio = 4,
    DefaultError = 5,
    InvalidOracle = 6,
    PriceBandsBreached = 7,
    MarketBeingInitialized = 8,
    PositionBankrupt = 9,
    InsufficientFunds = 10,
    UserCantLiquidateThemself = 11,
    InsufficientBalance = 12,
    InvalidAmount = 13,
    DivideByZero = 14,
    MultiplicationOverflow = 15,
    MultiplicationShiftRightOverflow = 16,
    PositionIsBeingLiquidated = 17,
    MarketOperationPaused = 18,
    InvalidPosition = 19,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PoolErrors {
    InvalidEnum = 6000,
    InvalidStartTick = 6001,
    // TickArrayExistInPool = 6002,
    TickArrayIndexOutofBounds = 6003,
    InvalidTickSpacing = 6004,
    ClosePositionNotEmpty = 6005,

    DivideByZero = 6006, // duplicates
    NumberCastError = 6007,
    NumberDownCastError = 6008,

    TickNotFound = 6009,
    InvalidTickIndex = 6010,
    SqrtPriceOutOfBounds = 6011,

    LiquidityZero = 6012,
    LiquidityTooHigh = 6013,
    LiquidityOverflow = 6014,
    LiquidityUnderflow = 6015,
    LiquidityNetError = 6016,

    TokenMaxExceeded = 6017,
    TokenMinSubceeded = 6018,

    MissingOrInvalidDelegate = 6019,
    InvalidPositionTokenAmount = 6020,

    InvalidTimestampConversion = 6021,
    InvalidTimestamp = 6022,

    InvalidTickArraySequence = 6023,
    InvalidTokenMintOrder = 6024,

    RewardNotInitialized = 6025,
    InvalidRewardIndex = 6026,

    RewardVaultAmountInsufficient = 6027,
    FeeRateMaxExceeded = 6028,
    ProtocolFeeRateMaxExceeded = 6029,

    MultiplicationShiftRightOverflow = 6030,
    MulDivOverflow = 6031,
    MulDivInvalidInput = 6032,
    MultiplicationOverflow = 6033,

    InvalidSqrtPriceLimitDirection = 6034,
    ZeroTradableAmount = 6035,

    AmountOutBelowMinimum = 6036,
    AmountInAboveMaximum = 6037,

    TickArraySequenceInvalidIndex = 6038,

    AmountCalcOverflow = 6039,
    AmountRemainingOverflow = 6040,

    InvalidIntermediaryMint = 6041,

    UnsupportedTokenMint = 6047,

    RemainingAccountsInvalidSlice = 6048,
    RemainingAccountsInsufficient = 6049,

    TransferFeeCalculationError = 6052,

    FullRangeOnlyPool = 6054,

    TooManySupplementalTickArrays = 6055,
    DifferentPoolTickArrayAccount = 6056,

    PartialFillError = 6057,
}

pub type NormalResult<T = ()> = core::result::Result<T, NormalError>;
