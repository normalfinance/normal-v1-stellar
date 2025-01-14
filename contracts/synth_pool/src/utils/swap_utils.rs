use soroban_sdk::Env;

use crate::{
    contract::SynthPool,
    controller::swap::PostSwapUpdate,
    storage::Config,
    token_contract,
};

#[allow(clippy::too_many_arguments)]
pub fn update_and_swap_amm(
    env: &Env,
    user: Address,
    token_a: Address,
    token_b: Address,
    swap_update: PostSwapUpdate,
    is_token_fee_in_a: bool,
    reward_last_updated_timestamp: u64
) -> Result<()> {
    Config::update_after_swap(
        swap_update.next_liquidity,
        swap_update.next_tick_index,
        swap_update.next_sqrt_price,
        swap_update.next_fee_growth_global,
        swap_update.next_reward_infos,
        swap_update.next_protocol_fee,
        is_token_fee_in_a,
        reward_last_updated_timestamp
    );

    perform_swap(
        env,
        user,
        token_a,
        token_b,
        swap_update.amount_a,
        swap_update.amount_b,
        is_token_fee_in_a
    )
}

#[allow(clippy::too_many_arguments)]
fn perform_swap(
    env: &Env,
    user: Address,
    token_a: Address,
    token_b: Address,
    amount_a: u64,
    amount_b: u64,
    a_to_b: bool
) -> Result<()> {
    // Transfer from user to pool
    let deposit_account_user;
    let deposit_account_pool;
    let deposit_amount;

    // Transfer from pool to user
    let withdrawal_account_user;
    let withdrawal_account_pool;
    let withdrawal_amount;

    if a_to_b {
        deposit_account_user = user;
        deposit_account_pool = env.current_contract_address(); // token_vault_a;
        deposit_amount = amount_a;

        withdrawal_account_user = user;
        withdrawal_account_pool = env.current_contract_address(); // token_vault_b;
        withdrawal_amount = amount_b;
    } else {
        deposit_account_user = user;
        deposit_account_pool = env.current_contract_address(); // token_vault_b;
        deposit_amount = amount_b;

        withdrawal_account_user = user;
        withdrawal_account_pool = env.current_contract_address(); // token_vault_a;
        withdrawal_amount = amount_a;
    }

    let deposit_token_client = token_contract::Client::new(env, token_a);
    deposit_token_client.transfer(&deposit_account_user, &deposit_account_pool, &deposit_amount);

    let withdrawal_token_client = token_contract::Client::new(env, token_b);
    withdrawal_token_client.transfer(
        &withdrawal_account_pool,
        &withdrawal_account_user,
        &withdrawal_amount
    );

    Ok(())
}
