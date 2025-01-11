use super::setup::{ deploy_synth_market_factory_contract };
use crate::{ contract::{ SynthMarketFactory, SynthMarketFactoryClient }, tests::setup::{} };

use soroban_sdk::{ testutils::{ arbitrary::std, Address as _ }, vec, Address, Env, String };

#[test]
fn factory_successfully_inits_itself() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    let factory = deploy_synth_market_factory_contract(&env, Some(admin.clone()), oracle);

    assert_eq!(factory.get_admin(), admin);
}

// #[test]
// fn factory_successfully_inits_index_token() {
//     let env = Env::default();
//     env.mock_all_auths();
//     env.cost_estimate().budget().reset_unlimited();

//     let admin = Address::generate(&env);
//     let oracle = Address::generate(&env);

//     let factory = deploy_synth_market_factory_contract(&env, Some(admin.clone()), oracle);

//     let index_address = factory.get_config().multihop_address;

//     assert!(!multihop_address.to_string().is_empty());
// }
