use soroban_sdk::{ contracttype, Address };


#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Admin,
    EmergencyAdmin,
    Oracle,
    Frozen,
}
