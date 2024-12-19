use soroban_sdk::{ contracttype, Address };

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    TokenA = 0,
    TokenB = 1,
    ReserveA = 2,
    ReserveB = 3,
}
