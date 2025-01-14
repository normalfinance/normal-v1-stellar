use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    InvalidEnum = 6000,
    InvalidStartTick = 6001,
    TickArrayExistInPool = 6002,
    TickArrayIndexOutofBounds = 6003,
    InvalidTickSpacing = 6004,
    ClosePositionNotEmpty = 6005,

    DivideByZero = 6006,
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
    DuplicateTwoHopPool = 6042,

    InvalidBundleIndex = 6043,
    BundledPositionAlreadyOpened = 6044,
    BundledPositionAlreadyClosed = 6045,
    PositionBundleNotDeletable = 6046,

    UnsupportedTokenMint = 6047,

    RemainingAccountsInvalidSlice = 6048,
    RemainingAccountsInsufficient = 6049,
    // NoExtraAccountsForTrz = 12,

    // IntermediateTokenAmountMismatch = 6051,

    // TransferFeeCalculationError = 6052,

    // RemainingAccountsDuplicatedAccountsType = 6053,

    // FullRangeOnlyPool = 6054,

    // TooManySupplementalTickArrays = 6055,
    // DifferentWhirlpoolTickArrayAccount = 6056,

    // PartialFillError = 6057,
}
