use soroban_sdk::{Address, Env};

use crate::dependencies::TokenClient;

pub fn claim_revenue(e: &Env, total_supply: i128, user: &Address, balance: i128) -> i128 {
    if let Some(emis_config) = storage::get_emission_config(e) {
        let prev_emis_data = storage::get_emission_data(e).unwrap_optimized(); // exists if config exists
        let emis_data = match update_emission_data(e, &prev_emis_data, &emis_config, total_supply) {
            Some(data) => {
                storage::set_emission_data(e, &data);
                data
            }
            None => prev_emis_data,
        };
        let prev_data = storage::get_user_emission_data(e, user);
        let mut user_data = match update_user_emissions(&prev_data, &emis_data, balance) {
            Some(data) => data,
            None => prev_data.unwrap_optimized(),
        };

        let to_claim = user_data.accrued.clone();
        if to_claim > 0 {
            user_data.accrued = 0;
            storage::set_user_emission_data(e, user, &user_data);

            // balance::mint_balance(e, &user, to_claim);
            TokenClient::new(e, address).transfer(&e.current_contract_address(), &user, to_claim);

            // TokenVotesEvents::claim(&e, user.clone(), to_claim);
        } else {
            storage::set_user_emission_data(e, user, &user_data);
        }
        to_claim
    } else {
        0
    }
}
