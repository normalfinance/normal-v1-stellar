use normal::ttl::{ PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD };
use soroban_sdk::{ contracttype, Address, Env };

pub(crate) const DAY_IN_LEDGERS: u32 = 17280;
pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub(crate) const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const BALANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub(crate) const BALANCE_LIFETIME_THRESHOLD: u32 = BALANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

#[derive(Clone)]
#[contracttype]
pub struct AllowanceDataKey {
    pub from: Address,
    pub spender: Address,
}

#[contracttype]
pub struct AllowanceValue {
    pub amount: i128,
    pub expiration_ledger: u32,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Allowance(AllowanceDataKey),
    Balance(Address),
    State(Address),
    IndexContract,
    LastTransfer(Address),
    Admin,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LastTransferInfo {
    pub last_transfer_ts: u64,
    pub last_balance: i128,
}

pub fn get_last_transfer_info(env: &Env, key: &Address) -> LastTransferInfo {
    let info = match env.storage().persistent().get::<_, LastTransferInfo>(key) {
        Some(_info) => _info,
        None =>
            LastTransferInfo {
                last_transfer_ts: 0u64, // current_time
                last_balance: 0i128,
            },
    };
    env.storage()
        .persistent()
        .has(&key)
        .then(|| {
            env.storage()
                .persistent()
                .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        });

    info
}

pub fn save_last_transfer_info(env: &Env, key: &Address, info: &LastTransferInfo) {
    env.storage().persistent().set(key, info);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}
