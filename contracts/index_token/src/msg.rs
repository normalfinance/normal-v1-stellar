use soroban_sdk::contracttype;

use crate::storage::Index;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexResponse {
    pub index: Index,
}
