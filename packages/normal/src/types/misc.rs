use soroban_sdk::contracttype;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum OrderDirection {
    Buy,
    Sell,
}
