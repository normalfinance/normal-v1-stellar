use super::setup::{
    deploy_scheduler_contract,
    install_token_wasm,
    deploy_synth_market_factory_contract,
    deploy_index_factory_contract,
};
use crate::{ contract::{ Scheduler }, tests::setup::{ install_and_deploy_token_contract } };

use soroban_sdk::{ testutils::{ arbitrary::std, Address as _ }, vec, Address, Env, String };

#[test]
fn scheduler_successfully_inits_itself() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let mut synth_market_factory = deploy_synth_market_factory_contract(&env, &admin);
    let mut index_factory = deploy_index_factory_contract(&env, &admin);
    let keepers = [Address::generate(&env)];

    let scheduler = deploy_scheduler_contract(
        &env,
        Some(admin.clone()),
        &synth_market_factory.address,
        &index_factory.address,
        keepers,
        500,
        200
    );

    assert_eq!(scheduler.get_admin(), admin);
}
