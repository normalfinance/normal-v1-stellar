use normal::error::ErrorCode as SharedErrors;
use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Errors {
    #[doc = "MaxIFWithdrawReached"]
    MaxIFWithdrawReached = 0,
    #[doc = "NoIFWithdrawAvailable"]
    NoIFWithdrawAvailable = 1,
    #[doc = "InvalidIFUnstake"]
    InvalidIFUnstake = 2,
    #[doc = "InvalidIFUnstakeSize"]
    InvalidIFUnstakeSize = 3,
    #[doc = "InvalidIFUnstakeCancel"]
    InvalidIFUnstakeCancel = 4,
    #[doc = "InvalidIFForNewStakes"]
    InvalidIFForNewStakes = 5,
    #[doc = "InvalidIFRebase"]
    InvalidIFRebase = 6,
    #[doc = "InvalidInsuranceUnstakeSize"]
    InvalidInsuranceUnstakeSize = 7,
    #[doc = "InsuranceFundOperationPaused"]
    InsuranceFundOperationPaused = 8,
    #[doc = "IFWithdrawRequestInProgress"]
    IFWithdrawRequestInProgress = 9,
    #[doc = "NoIFWithdrawRequestInProgress"]
    NoIFWithdrawRequestInProgress = 10,
    #[doc = "IFWithdrawRequestTooSmall"]
    IFWithdrawRequestTooSmall = 11,
    #[doc = "InvalidIFSharesDetected"]
    InvalidIFSharesDetected = 12,
    #[doc = "Insufficient IF shares"]
    InsufficientIFShares = 13,
    #[doc = "Trying to remove liqudity too fast after adding it"]
    TryingToRemoveLiquidityTooFast = 14,
}

pub enum NormalError {
    Shared(SharedErrors),
    Errors(Errors),
}

// Auto-convert from SharedErrors
impl From<SharedErrors> for NormalError {
    fn from(error: SharedErrors) -> Self {
        NormalError::Shared(error)
    }
}

// Auto-convert from Errors
impl From<Errors> for NormalError {
    fn from(error: Errors) -> Self {
        NormalError::Errors(error)
    }
}

pub type NormalResult<T = ()> = core::result::Result<T, NormalError>;
