use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum MarketStatus {
    /// warm up period for initialization, swapping is paused
    Initialized,
    /// all operations allowed
    Active,
    /// swaps only able to reduce liability
    ReduceOnly,
    /// market has determined settlement price and positions are expired must be settled
    Settlement,
    /// market has no remaining participants
    Delisted,
}
