use soroban_sdk::{ contracttype, Address };

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Admin,
    TokenA = 0,
	PausedOperations
}

pub enum SyntheticTier {
    /// max insurance capped at A level
    A,
    /// max insurance capped at B level
    B,
    /// max insurance capped at C level
    C,
    /// no insurance
    Speculative,
    /// no insurance, another tranches below
    #[default]
    HighlySpeculative,
    /// no insurance, only single position allowed
    Isolated,
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, contracttype)]
pub enum Operation {
    Create,
    Deposit,
    Withdraw,
    Lend,
    Transfer,
    Delete,
    Liquidation,
}
