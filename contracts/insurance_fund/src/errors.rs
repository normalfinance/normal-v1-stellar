use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    InsuranceFundOperationPaused = 0,
    InvalidInsuranceFundAuthority = 1,
    InsufficientIFShares = 2,
    InvalidInsuranceUnstakeSize = 3,
}
