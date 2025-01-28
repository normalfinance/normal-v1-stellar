use soroban_sdk::contracterror;

pub type NormalResult<T = ()> = core::result::Result<T, ErrorCode>;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    AlreadyInitialized = 1,
    NotAuthorized = 2,
    AdminNotSet = 3,
    TransferAmountTooSmallAfterFees = 4,
    InvalidFee = 5,
    TooSoonToRebalance = 7,
    InsufficientDeposit = 8,
    OracleNonPositive = 9,
    OracleTooVolatile = 10,
    OracleTooUncertain = 11,
    OracleStaleForMargin = 12,
    OracleInsufficientDataPoints = 13,
    OracleStaleForAMM = 14,
    MathError = 15,
    TokenNotAllowed = 16,
    BnConversionError = 17,
    CastingFailure = 18,
    FailedUnwrap = 19,
    InsufficientFunds = 20,

    // Governor Errors

    // Index Token Errors

    // Index Token Factory Errors
    IndexFactoryOperationPaused = 21,
    // Insurance Errors

    // Scheduler Errors

    // Synth Market Errors

    // Synth Market Factory Errors

    // Toke Errors

    // Vesting Errors

    // Vote Errors
}
