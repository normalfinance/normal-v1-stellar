use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    InvalidMarginRatio,
    UserCantLiquidateThemself,
    #[msg("UserBankrupt")]
    UserBankrupt,
    #[msg("UserNotBankrupt")]
    UserNotBankrupt,
    #[msg("Cant transfer between same user account")]
    CantTransferBetweenSameUserAccount,
}
