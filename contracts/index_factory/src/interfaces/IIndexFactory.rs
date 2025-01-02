use soroban_sdk::{Address, Env};

use crate::storage_types::Pair;

pub trait IIndexFactory {
    fn init(e: Env, admin: Address, fee: u64);
    fn get_admin(e: Env) -> Address;
    fn get_fee(e: Env) -> u64;
    fn set_fee(e: Env, new_fee: u64);
    fn get_pair(e: Env, token0: Address, token1: Address) -> Pair;
    fn get_pair_by_id(e: Env, id: u64) -> Pair;
    // fn get_pair_by_index(e: Env, index: u64) -> Pair;
    fn get_pairs_length(e: Env) -> u64;
    fn create_pair(e: Env, token0: Address, token1: Address) -> Address;
}