use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    AlreadyInitialized = 1,
    KeeperAccountsEmpty = 2,
    NotAuthorized = 3,
    InvalidScheduleOwner = 4,
}
