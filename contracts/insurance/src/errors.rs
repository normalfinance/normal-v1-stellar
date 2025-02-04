// use normal::error::ErrorCode as SharedErrors;
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
    AlreadyInitialized = 15,
    NotAuthorized = 16,
    AdminNotSet = 17,
}
