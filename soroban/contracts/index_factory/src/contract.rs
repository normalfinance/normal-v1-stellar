use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

use crate::{
    errors,
    interfaces::{ IIndexFactory::IIndexFactory, IIndex::deploy_index },
    storage::{
        get_admin,
        get_protocol_fee,
        get_index_by_id,
        get_index_by_tokens,
        get_indexes_length,
        increase_indexs_length,
        set_admin,
        set_protocol_fee,
        set_index,
    },
    storage_types::{ DataKey, Index },
};

#[contract]
pub struct IndexFactory;

#[contractimpl]
impl IIndexFactory for IndexFactory {
    fn init(e: Env, admin: Address, fee: u64, default_oracle: Address) {
        // todo: already initiazed check
        //
        set_admin(&e, admin);
        set_protocol_fee(&e, fee);
        set_default_oracle(&e, default_oracle);
    }

    fn get_admin(e: Env) -> Address {
        get_admin(&e)
    }

    fn get_protocol_fee(e: Env) -> u64 {
        get_protocol_fee(&e)
    }

    fn set_protocol_fee(e: Env, new_protocol_fee: u64) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        set_protocol_fee(&e, new_protocol_fee);
    }

    fn get_default_oracle(e: Env) -> u64 {
        get_default_oracle(&e)
    }

    fn set_default_oracle(e: Env, new_default_oracle: Address) {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        set_default_oracle(&e, new_default_oracle);
    }

    fn get_index(e: Env, token0: Address, token1: Address) -> Index {
        get_index_by_tokens(&e, token0, token1).unwrap()
    }

    fn get_index_by_id(e: Env, id: u64) -> Index {
        get_index_by_id(&e, id)
    }

    fn get_indexes_length(e: Env) -> u64 {
        get_indexes_length(&e)
    }

    fn create_index(e: Env, token0: Address, token1: Address) -> Address {
        let index_exists = get_index_by_tokens(&e, token0.clone(), token1.clone()).is_some();

        assert_with_error!(&e, index_exists, errors::Error::IndexAlreadyExist);

        let (address, _) = deploy_index(
            &e,
            token0.clone(),
            token1.clone(),
            e.current_contract_address()
        );

        let index = Index {
            index_address: address.clone(),
        };

        increase_indexes_length(&e);
        set_index(&e, 0, index);

        address
    }
}
