use soroban_sdk::{ contracttype, Address };

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    DefaultOracle,
    ProtocolFee,
    Indexes(u64),
    IndexesLength,
    Index(Address),
}

#[derive(Clone)]
#[contracttype]
pub struct Index {
    // TODO: add meta info?
    pub index_address: Address,
}
