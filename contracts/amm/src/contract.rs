use soroban_sdk::{
    contract,
    contractimpl,
    contractmeta,
    log,
    panic_with_error,
    Address,
    BytesN,
    Env,
    String,
    U256,
};

use num_integer::Roots;

use crate::{
    error::ContractError,
    stake_contract,
    storage::{
        get_config,
        get_default_slippage_bps,
        save_config,
        save_default_slippage_bps,
        utils::{ self, get_admin_old, is_initialized, set_initialized },
        Asset,
        ComputeSwap,
        Config,
        LiquidityPoolInfo,
        PairType,
        PoolResponse,
        SimulateReverseSwapResponse,
        SimulateSwapResponse,
        ADMIN,
    },
    token_contract,
};
use normal::{
    ttl::{ INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD },
    utils::{ convert_i128_to_u128, is_approx_ratio, AMMParams },
    validate_bps,
    validate_int_parameters,
};
use soroban_decimal::Decimal;
contractmeta!(
    key = "Description",
    val = "Constant product AMM that maintains a synthetic asset peg"
);

#[contract]
pub struct AMM;

#[contractimpl]
impl AMMTrait for AMM {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        e: Env,
        token_wasm_hash: BytesN<32>,
        params: AMMParams,
        share_token_decimals: u32,
        share_token_name: String,
        share_token_symbol: String,
        default_slippage_bps: i64,
        max_allowed_fee_bps: i64
    ) {
        if is_initialized(&e) {
            log!(&e, "Pool: Initialize: initializing contract twice is not allowed");
            panic_with_error!(&e, ContractError::AlreadyInitialized);
        }

        validate_bps!(
            params.fee_rate,
            params.protocol_fee_rate,
            max_allowed_slippage_bps,
            max_allowed_spread_bps,
            max_allowed_variance_bps,
            default_slippage_bps,
            max_allowed_fee_bps
        );

        if params.token_a >= params.token_b {
            panic!("token_a must be less than token_b");
        }

        // if !(MIN_SQRT_PRICE_X64..=MAX_SQRT_PRICE_X64).contains(&sqrt_price) {
        //     return Err(ErrorCode::SqrtPriceOutOfBounds.into());
        // }

        // if fee_rate > MAX_FEE_RATE {
        //     return Err(ErrorCode::FeeRateMaxExceeded.into());
        // }
        // if protocol_fee_rate > MAX_PROTOCOL_FEE_RATE {
        //     return Err(ErrorCode::ProtocolFeeRateMaxExceeded.into());
        // }

        // deploy and initialize token contract
        let share_token_address = utils::deploy_token_contract(
            &e,
            token_wasm_hash.clone(),
            &params.token_a,
            &params.token_b,
            e.current_contract_address(),
            share_token_decimals,
            share_token_name,
            share_token_symbol
        );

        let config = Config {
            token_a: params.token_a.clone(),
            token_b: params.token_b.clone(),
            share_token: share_token_address,

            tick_arrays: [],
            sqrt_price: initial_sqrt_price,
            liquidity: 0,
            tick_spacing,

            positions: [],

            fee_rate,
            protocol_fee_rate,
            protocol_fee_owed_synthetic: 0,
            protocol_fee_owed_quote: 0,
            fee_growth_global_synthetic: 0,
            fee_growth_global_quote: 0,

            max_allowed_slippage_bps,
            max_allowed_spread_bps,
            max_allowed_variance_bps,

            reward_last_updated_timestamp: 0,
            reward_infos: [RewardInfo::new(state.reward_emissions_super_authority); MAX_REWARDS],
        };

        save_config(&e, config);
        save_default_slippage_bps(&e, default_slippage_bps);

        utils::save_admin_old(&e, admin);
        utils::save_total_shares(&e, 0);
        utils::save_pool_balance_a(&e, 0);
        utils::save_pool_balance_b(&e, 0);

        AMMEvents::initialize(&e, index_id, from, amount);
    }

    #[allow(clippy::too_many_arguments)]
    fn update_config(
        e: Env,
        new_admin: Option<Address>,
        fee_rate: Option<i64>,
        protocol_fee_rate: Option<i64>,
        max_allowed_slippage_bps: Option<i64>,
        max_allowed_spread_bps: Option<i64>,
        max_allowed_variance_bps: Option<i64>
    ) {
        let admin: Address = utils::get_admin_old(&env);
        admin.require_auth();
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let mut config = get_config(&env);

        // Admin
        if let Some(new_admin) = new_admin {
            utils::save_admin_old(&env, new_admin);
        }

        // Fees
        if let Some(fee_rate) = fee_rate {
            validate_bps!(fee_rate);
            config.fee_rate = fee_rate;
        }
        if let Some(protocol_fee_rate) = protocol_fee_rate {
            validate_bps!(protocol_fee_rate);
            config.protocol_fee_rate = protocol_fee_rate;
        }

        // Slippage
        if let Some(max_allowed_slippage_bps) = max_allowed_slippage_bps {
            validate_bps!(max_allowed_slippage_bps);
            config.max_allowed_slippage_bps = max_allowed_slippage_bps;
        }

        // Spread
        if let Some(max_allowed_spread_bps) = max_allowed_spread_bps {
            validate_bps!(max_allowed_spread_bps);
            config.max_allowed_spread_bps = max_allowed_spread_bps;
        }

        // Variance
        if let Some(max_allowed_variance_bps) = max_allowed_variance_bps {
            validate_bps!(max_allowed_variance_bps);
            config.max_allowed_variance_bps = max_allowed_variance_bps;
        }

        save_config(&env, config);
    }

    pub fn reset_oracle_twap(e: Env) -> u128 {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
    }

    pub fn update_oracle_twap(e: Env) -> u128 {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
    }

    pub fn initialize_reward(e: Env, token_reward: Address) {
        // Check not exceeding max rewards
        if index >= NUM_REWARDS {
            return Err(ErrorCode::InvalidRewardIndex.into());
        }

        let reward = AMMRewardInfo {
            token: token_reward,
            emissions_per_second_x64: 0,
            growth_global_x64: 0,
        };

        increase_rewards_length(&e);
        set_reward(&e, 0, reward);
    }

    pub fn set_reward_emissions(e: Env, reward_index: u8, emissions_per_second_x64: u128) {
        let reward = get_reward_by_id(&e, id);

        // ...

        set_reward_emissions(&e, id, emissions_per_second_x64);
    }

    // User

    fn create_position(e: Env, tick_lower_index: i32, tick_upper_index: i32) {
        let new_position = Position {
            asset: "BTC".to_string(),
            amount: 100,
            entry_price: 50000,
        };

        // Add a new position for the user
        Positions::add(&env, &user_address, new_position);

        AMMEvents::CreatePosition();
    }

    fn close_position(e: Env, position_timestamp: u64) {
        if !Position::is_position_empty(&ctx.accounts.position) {
            return Err(ErrorCode::ClosePositionNotEmpty.into());
        }

        AMMEvents::ClosePosition();
    }

    pub fn increase_liquidity(
        e: Env,
        sender: Address,
        liquidity_amount: u128,
        token_max_a: u64,
        token_max_b: u64
    ) {
        // Depositor needs to authorize the deposit
        sender.require_auth();

        if liquidity_amount == 0 {
            return Err(ErrorCode::LiquidityZero.into());
        }
        let liquidity_delta = convert_to_liquidity_delta(liquidity_amount, true)?;

        let update = controller::liquidity::calculate_modify_liquidity(
            &ctx.accounts.amm,
            &ctx.accounts.position,
            &ctx.accounts.tick_array_lower,
            &ctx.accounts.tick_array_upper,
            liquidity_delta,
            timestamp
        )?;

        let token_a_client = token::Client::new(&e, &get_token_a(&e));
        let token_b_client = token::Client::new(&e, &get_token_b(&e));

        token_a_client.transfer(&to, &e.current_contract_address(), &amount_a);
        token_b_client.transfer(&to, &e.current_contract_address(), &amount_b);

        // mint token

        AMMEvents::add_liquidity(&e, to, amount_a, amount_b);
    }

    pub fn collect_fees(e: Env, to: Address, fee_amount: i128) -> (i128, i128) {
        to.require_auth();

        // ...

        let fee_owed_a = 0;
        let fee_owed_b = 0;

        // reset fees owed

        transfer_a(&e, to.clone(), fee_owed_a);
        transfer_b(&e, to, fee_owed_b);

        AMMEvents::collect_fees(&e, to, amount);

        (fee_owed_a, fee_owed_b)
    }

    pub fn decrease_liquidity(
        e: Env,
        sender: Address,
        liquidity_amount: u128,
        token_max_a: u64,
        token_max_b: u64
    ) -> (i128, i128) {
        to.require_auth();

        // First transfer the pool shares that need to be redeemed
        let share_token_client = token::Client::new(&e, &get_token_share(&e));
        share_token_client.transfer(&to, &e.current_contract_address(), &share_amount);

        let (balance_a, balance_b) = (get_balance_a(&e), get_balance_b(&e));
        let balance_shares = get_balance_shares(&e);

        let total_shares = get_total_shares(&e);

        // Now calculate the withdraw amounts
        let out_a = (balance_a * balance_shares) / total_shares;
        let out_b = (balance_b * balance_shares) / total_shares;

        if out_a < min_a || out_b < min_b {
            panic!("min not satisfied");
        }

        burn_shares(&e, balance_shares);
        transfer_a(&e, to.clone(), out_a);
        transfer_b(&e, to, out_b);
        put_reserve_a(&e, balance_a - out_a);
        put_reserve_b(&e, balance_b - out_b);

        AMMEvents::remove_liquidity(&e, to, out_a, out_b);

        (out_a, out_b)
    }

    pub fn swap(
        e: Env,
        sender: Address,
        amount: u64,
        other_amount_threshold: u64,
        sqrt_price_limit: u128,
        amount_specified_is_input: bool,
        a_to_b: bool // Zero for one
    ) {
        // validate amount above 0?

        sender.require_auth();

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // do swap

        let builder = SparseSwapTickSequenceBuilder::try_from(
            a_to_b,
            vec![
                ctx.accounts.tick_array_0.to_account_info(),
                ctx.accounts.tick_array_1.to_account_info(),
                ctx.accounts.tick_array_2.to_account_info()
            ],
            None
        )?;
        let mut swap_tick_sequence = builder.build()?;

        // ---

        let swap_update = do_swap(
            &mut swap_tick_sequence,
            amount,
            sqrt_price_limit,
            amount_specified_is_input,
            a_to_b,
            timestamp
        )?;

        // ---

        if amount_specified_is_input {
            if
                (a_to_b && other_amount_threshold > swap_update.amount_quote) ||
                (!a_to_b && other_amount_threshold > swap_update.amount_synthetic)
            {
                return Err(ErrorCode::AmountOutBelowMinimum.into());
            }
        } else if
            (a_to_b && other_amount_threshold < swap_update.amount_synthetic) ||
            (!a_to_b && other_amount_threshold < swap_update.amount_quote)
        {
            return Err(ErrorCode::AmountInAboveMaximum.into());
        }

        update_and_swap_amm(
            amm,
            &ctx.accounts.token_authority,
            &ctx.accounts.token_owner_account_synthetic,
            &ctx.accounts.token_owner_account_quote,
            &ctx.accounts.token_vault_synthetic,
            &ctx.accounts.token_vault_quote,
            &ctx.accounts.token_program,
            swap_update,
            synthetic_to_quote,
            timestamp,
            inside_range
        );

        AMMEvents::swap(&e, to, buy_a, out, in_max);
    }

    pub fn collect_reward(e: Env, to: Address) {
        to.require_auth();
    }

    // Queries

    fn query_share_token_address(env: Env) -> Address {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        get_config(&env).share_token
    }

    fn query_pool_info(env: Env) -> PoolResponse {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let config = get_config(&env);

        PoolResponse {
            asset_a: Asset {
                address: config.token_a,
                amount: utils::get_pool_balance_a(&env),
            },
            asset_b: Asset {
                address: config.token_b,
                amount: utils::get_pool_balance_b(&env),
            },
            asset_lp_share: Asset {
                address: config.share_token,
                amount: utils::get_total_shares(&env),
            },
        }
    }
}

// // Function to remove a stake from the vector
// fn remove_reward(env: &Env, stakes: &mut Vec<Stake>, stake: i128, stake_timestamp: u64) {
//     // Find the index of the stake that matches the given stake and stake_timestamp
//     if let Some(index) = stakes
//         .iter()
//         .position(|s| s.stake == stake && s.stake_timestamp == stake_timestamp)
//     {
//         // Remove the stake at the found index
//         stakes.remove(index as u32);
//     } else {
//         // Stake not found, return an error
//         log!(&env, "Stake: Remove stake: Stake not found");
//         panic_with_error!(&env, ContractError::StakeNotFound);
//     }
// }



fn do_swap(
    env: Env,
    sender: Address,
    // FIXM: Disable Referral struct
    // referral: Option<Referral>,
    offer_asset: Address,
    offer_amount: i128,
    ask_asset_min_amount: Option<i128>,
    max_spread: Option<i64>,
    max_allowed_fee_bps: Option<i64>
) -> i128 {}



