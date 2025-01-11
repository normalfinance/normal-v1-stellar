use soroban_sdk::contracttype;

#[derive(Clone)]
#[contracttype]
pub enum OrderDirection {
    Buy,
    Sell,
}
