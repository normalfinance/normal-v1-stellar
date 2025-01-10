use soroban_sdk::contracttype;

// pub type NormalResult<T = ()> = core::result::Result<T, ErrorCode>;

#[derive(Clone)]
#[contracttype]
pub enum OrderDirection {
    Buy,
    Sell,
}
