use soroban_sdk::contracterror;

pub type NormalResult<T = ()> = core::result::Result<T, ErrorCode>;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    // Shared Errors
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

    // Governor Errors

    // Index Token Errors

    // Index Token Factory Errors
    IndexFactoryOperationPaused = 21,
    // Insurance Errors

    // Scheduler Errors

    // Synth Market Errors

    // Synth Market Factory Errors

    // Synth Pool Errors

    // Toke Errors

    // Vesting Errors

    // Vote Errors
}

#[macro_export]
macro_rules! print_error {
    ($err:expr, $env:expr) => {
        {
        || {
            let error_code: ErrorCode = $err;
            log!($env, "{:?} thrown at {}:{}", error_code, file!(), line!());
            $err
        }
        }
    };
}

#[macro_export]
macro_rules! math_error {
    ($env:expr) => {
        {
        || {
            let error_code = $crate::error::ErrorCode::MathError;
            log!(
                $env,
                "Error {} thrown at {}:{}",
                error_code,
                file!(),
                line!()
            );
            error_code
        }
        }
    };
}
