use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    AlreadyInitialized = 100,
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
}

pub type NormalResult<T = ()> = core::result::Result<T, ErrorCode>;
