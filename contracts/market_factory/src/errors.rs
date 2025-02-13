use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Errors {
    AlreadyInitialized = 1,
    NotAuthorized = 2,
    MarketNotFound = 3,
    AdminNotSet = 4,
}
