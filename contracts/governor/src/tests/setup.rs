use soroban_sdk::{ Address, BytesN, Env };

use crate::{ contract::{ Governor, GovernorClient }, types::GovernorSettings };

// pub fn deploy_token_contract<'a>(env: &Env, admin: &Address) -> token_contract::Client<'a> {
//     token_contract::Client::new(
//         env,
//         &env.register_stellar_asset_contract_v2(admin.clone()).address()
//     )
// }

// pub fn install_token_wasm(env: &Env) -> BytesN<32> {
//     env.deployer().upload_contract_wasm(token_contract::WASM)
// }

pub fn deploy_governance_contract<'a>(env: &Env, admin: &Address) -> GovernorClient<'a> {
    let governor = GovernorClient::new(env, &env.register(Governor, ()));

    let settings = GovernorSettings {
        proposal_threshold: 0,
        vote_delay: 0,
        vote_period: 0,
        timelock: 0,
        grace_period: 0,
        quorum: 0,
        counting_type: 0,
        vote_threshold: 0,
    };

    governor.initialize(&admin, &admin, &settings);

    governor
}
