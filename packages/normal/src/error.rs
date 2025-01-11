use soroban_sdk::contracterror;

pub type NormalResult<T = ()> = core::result::Result<T, ErrorCode>;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    AlreadyInitialized = 1,
    NotAuthorized = 3,
    AdminNotSet = 8,
    TransferAmountTooSmallAfterFees,
}
