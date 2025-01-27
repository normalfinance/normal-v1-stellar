use soroban_sdk::{
    contract,
    contractimpl,
    contractmeta,
    log,
    panic_with_error,
    vec,
    Address,
    BytesN,
    Env,
    String,
    Vec,
};

use crate::{
    controller,
    events::SynthPoolEvents,
    math::bit_math::checked_mul_shift_right,
    pool::SynthPoolTrait,
    position::{ get_position_by_ts, get_positions, Position, PositionUpdate },
    reward::{ calculate_collect_reward, RewardInfo },
    storage::{
        get_pool,
        save_config,
        save_default_slippage_bps,
        utils::{ self, get_admin_old, is_initialized, set_initialized },
        Pool,
        SynthPoolParams,
    },
    tick::Tick,
    tick_array::TickArray,
    token_contract,
    utils::{ sparse_swap::SparseSwapTickSequenceBuilder, swap_utils::update_and_swap_amm },
};
use normal::{
    constants::{ INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD },
    error::ErrorCode,
    oracle::get_oracle_price,
    utils::{ convert_i128_to_u128, is_approx_ratio },
    validate_bps,
    validate_int_parameters,
};

fn check_nonnegative_amount(amount: i128) {
    if amount < 0 {
        panic!("negative amount is not allowed: {}", amount)
    }
}

fn is_admin(env: &Env, sender: Address) {
    let admin = read_administrator(&env);
    if admin != sender {
        log!(&env, "Index Token: You are not authorized!");
        panic_with_error!(&env, ErrorCode::NotAuthorized);
    }
}

contractmeta!(
    key = "Description",
    val = "Constant product AMM that maintains a synthetic asset peg"
);

#[contract]
pub struct SynthPool;

#[contractimpl]
impl SynthPoolTrait for SynthPool {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        env: Env,
        token_wasm_hash: BytesN<32>,
        params: SynthPoolParams,
        share_token_decimals: u32,
        share_token_name: String,
        share_token_symbol: String,
        default_slippage_bps: i64,
        max_allowed_fee_bps: i64
    ) {
        if is_initialized(&env) {
            log!(&env, "Pool: Initialize: initializing contract twice is not allowed");
            panic_with_error!(&env, ErrorCode::AlreadyInitialized);
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

        if !(MIN_SQRT_PRICE_X64..=MAX_SQRT_PRICE_X64).contains(&params.sqrt_price) {
            return Err(ErrorCode::SqrtPriceOutOfBounds.into());
        }

        // deploy and initialize token contract
        let share_token_address = utils::deploy_token_contract(
            &env,
            token_wasm_hash.clone(),
            &params.token_a,
            &params.token_b,
            env.current_contract_address(),
            share_token_decimals,
            share_token_name,
            share_token_symbol
        );

        let config = Config {
            token_a: params.token_a.clone(),
            token_b: params.token_b.clone(),
            share_token: share_token_address,

            tick_arrays: Vec::new(&env),
            sqrt_price: initial_sqrt_price,
            liquidity: 0,
            tick_spacing,

            positions: Vec::new(&env),

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
            reward_infos: Vec::new(&env),
        };

        save_config(&env, config);
        save_default_slippage_bps(&env, default_slippage_bps);

        SynthPoolEvents::initialize(&env, index_id, from, amount);
    }

    fn initialize_tick_array(env: Env, start_tick_index: i32) {
        let pool = get_pool(&env);

        if !Tick::check_is_valid_start_tick(start_tick_index, &pool.tick_spacing) {
            return Err(ErrorCode::InvalidStartTick.into());
        }

        // tick_array.market = &ctx.accounts.markets.key();
        tick_array.start_tick_index = start_tick_index;
    }

    #[allow(clippy::too_many_arguments)]
    fn update_config(
        env: Env,
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

        let mut pool = get_pool(&env);

        // Admin
        if let Some(new_admin) = new_admin {
            utils::save_admin_old(&env, new_admin);
        }

        // Fees
        if let Some(fee_rate) = fee_rate {
            validate_bps!(fee_rate);
            pool.fee_rate = fee_rate;
        }
        if let Some(protocol_fee_rate) = protocol_fee_rate {
            validate_bps!(protocol_fee_rate);
            pool.protocol_fee_rate = protocol_fee_rate;
        }

        // Slippage
        if let Some(max_allowed_slippage_bps) = max_allowed_slippage_bps {
            validate_bps!(max_allowed_slippage_bps);
            pool.max_allowed_slippage_bps = max_allowed_slippage_bps;
        }

        // Spread
        if let Some(max_allowed_spread_bps) = max_allowed_spread_bps {
            validate_bps!(max_allowed_spread_bps);
            pool.max_allowed_spread_bps = max_allowed_spread_bps;
        }

        // Variance
        if let Some(max_allowed_variance_bps) = max_allowed_variance_bps {
            validate_bps!(max_allowed_variance_bps);
            pool.max_allowed_variance_bps = max_allowed_variance_bps;
        }

        save_pool(&env, pool);
    }

    fn reset_oracle_twap(env: Env, sender: Address) {
        sender.require_auth();
        is_admin(&env, sender);

        // admin failsafe to reset amm oracle_twap to the mark_twap

        let pool = &mut get_pool(&env);

        log!(&env, "resetting amm oracle twap for market {}", market.market_index);
        log!(
            &env,
            "amm.historical_oracle_data.last_oracle_price_twap: {:?} -> {:?}",
            amm.historical_oracle_data.last_oracle_price_twap,
            amm.last_mark_price_twap.cast::<i64>()?
        );

        log!(
            &env,
            "amm.historical_oracle_data.last_oracle_price_twap_ts: {:?} -> {:?}",
            amm.historical_oracle_data.last_oracle_price_twap_ts,
            amm.last_mark_price_twap_ts
        );

        pool.historical_oracle_data.last_oracle_price_twap =
            pool.last_mark_price_twap.cast::<i64>()?;
        pool.historical_oracle_data.last_oracle_price_twap_ts = pool.last_mark_price_twap_ts;
    }

    fn update_oracle_twap(env: Env, sender: Address, price_oracle: Address) {
        sender.require_auth();
        is_admin(&env, sender);

        // allow update to amm's oracle twap iff price gap is reduced and thus more tame funding
        // otherwise if oracle error or funding flip: set oracle twap to mark twap (0 gap)

        let now = env.ledger().timestamp();

        let pool = &mut get_pool(&env);
        log!(&env, "updating amm oracle twap for market {}", market.market_index);
        // let price_oracle = &ctx.accounts.oracle;
        let oracle_twap = pool.get_oracle_twap(price_oracle, clock.slot)?;

        if let Some(oracle_twap) = oracle_twap {
            let oracle_mark_gap_before = pool.last_mark_price_twap
                .cast::<i64>()?
                .safe_sub(pool.historical_oracle_data.last_oracle_price_twap)?;

            let oracle_mark_gap_after = amm.last_mark_price_twap
                .cast::<i64>()?
                .safe_sub(oracle_twap)?;

            if
                (oracle_mark_gap_after > 0 && oracle_mark_gap_before < 0) ||
                (oracle_mark_gap_after < 0 && oracle_mark_gap_before > 0)
            {
                log!(
                    &env,
                    "amm.historical_oracle_data.last_oracle_price_twap {} -> {}",
                    amm.historical_oracle_data.last_oracle_price_twap,
                    amm.last_mark_price_twap.cast::<i64>()?
                );
                log!(
                    &env,
                    "amm.historical_oracle_data.last_oracle_price_twap_ts {} -> {}",
                    amm.historical_oracle_data.last_oracle_price_twap_ts,
                    now
                );
                amm.historical_oracle_data.last_oracle_price_twap =
                    amm.last_mark_price_twap.cast::<i64>()?;
                amm.historical_oracle_data.last_oracle_price_twap_ts = now;
            } else if oracle_mark_gap_after.unsigned_abs() <= oracle_mark_gap_before.unsigned_abs() {
                log!(
                    &env,
                    "amm.historical_oracle_data.last_oracle_price_twap {} -> {}",
                    amm.historical_oracle_data.last_oracle_price_twap,
                    oracle_twap
                );
                log!(
                    &env,
                    "amm.historical_oracle_data.last_oracle_price_twap_ts {} -> {}",
                    amm.historical_oracle_data.last_oracle_price_twap_ts,
                    now
                );
                amm.historical_oracle_data.last_oracle_price_twap = oracle_twap;
                amm.historical_oracle_data.last_oracle_price_twap_ts = now;
            } else {
                return Err(ErrorCode::PriceBandsBreached.into());
            }
        } else {
            return Err(ErrorCode::InvalidOracle.into());
        }
    }

    fn initialize_reward(env: Env, token_reward: Address) {
        let pool = &mut get_pool(&env);

        let index: usize = reward_index as usize;

        if index >= NUM_REWARDS {
            return Err(ErrorCode::InvalidRewardIndex.into());
        }

        let lowest_index = match pool.reward_infos.iter().position(|r| !r.initialized()) {
            Some(lowest_index) => lowest_index,
            None => {
                return Err(ErrorCode::InvalidRewardIndex.into());
            }
        };

        if lowest_index != index {
            return Err(ErrorCode::InvalidRewardIndex.into());
        }

        pool.reward_infos[index].mint = ctx.accounts.reward_mint.key();
        pool.reward_infos[index].vault = ctx.accounts.reward_vault.key();

        // let reward = AMMRewardInfo {
        //     token: token_reward,
        //     emissions_per_second_x64: 0,
        //     growth_global_x64: 0,
        // };
    }

    fn set_reward_emissions(env: Env, reward_index: u8, emissions_per_second_x64: u128) {
        // let reward = get_reward_by_id(&env, id);

        // ...

        let pool = get_pool(&env);
        // let reward_vault = &ctx.accounts.reward_vault;

        let emissions_per_day = checked_mul_shift_right(DAY_IN_SECONDS, emissions_per_second_x64)?;
        if reward_vault.amount < emissions_per_day {
            return Err(ErrorCode::RewardVaultAmountInsufficient.into());
        }

        let timestamp = env.ledger().timestamp();
        let next_reward_infos = controller::amm::next_amm_reward_infos(pool, timestamp)?;

        let index: usize = reward_index as usize;

        if index >= NUM_REWARDS {
            return Err(ErrorCode::InvalidRewardIndex.into());
        }
        pool.update_rewards(next_reward_infos, timestamp);
        pool.reward_infos[index].emissions_per_second_x64 = emissions_per_second_x64;
    }

    // ################################################################
    //                             KEEPER
    // ################################################################

    fn collect_protocol_fees(env: Env, sender: Address, to: Address) {
        sender.require_auth();
        // TODO: ensure DAO or keeper

        let pool = get_pool(&env);

        pool.transfer_a(env.current_contract_address(), to, pool.protocol_fee_owed_a);
        pool.transfer_a(env.current_contract_address(), to, pool.protocol_fee_owed_b);

        pool.reset_protocol_fees_owed();
    }

    // ################################################################
    //                             USER
    // ################################################################

    fn create_position(env: Env, tick_lower_index: i32, tick_upper_index: i32) {
        let pool = get_pool(&env);

        Position::open_position(&pool, tick_lower_index, tick_upper_index)?;

        mint_position_token(
            market,
            position_mint,
            &ctx.accounts.position_token_account,
            &ctx.accounts.token_program
        )?;

        SynthPoolEvents::CreatePosition();
    }

    fn modify_position(env: Env, sender: Address, position_ts: u64, update: PositionUpdate) {
        sender.require_auth();

        let position = &mut get_position_by_ts(&env, &sender, position_ts);

        position.update(&update);
    }

    fn close_position(env: Env, sender: Address, position_ts: u64) {
        verify_position_authority(
            &ctx.accounts.position_token_account,
            &ctx.accounts.position_authority
        )?;

        let position = get_position_by_ts(&env, &sender, position_ts);

        if !Position::is_position_empty(&position) {
            return Err(ErrorCode::ClosePositionNotEmpty.into());
        }

        burn_and_close_user_position_token(
            &ctx.accounts.position_authority,
            &ctx.accounts.receiver,
            &ctx.accounts.position_mint,
            &ctx.accounts.position_token_account,
            &ctx.accounts.token_program
        );

        SynthPoolEvents::ClosePosition();
    }

    fn increase_liquidity(
        env: Env,
        sender: Address,
        position_ts: u64,
        liquidity_amount: u128,
        token_max_a: u64,
        token_max_b: u64,
        tick_array_lower: TickArray,
        tick_array_upper: TickArray
    ) {
        // Depositor needs to authorize the deposit
        sender.require_auth();

        if liquidity_amount == 0 {
            return Err(ErrorCode::LiquidityZero.into());
        }
        let liquidity_delta = convert_to_liquidity_delta(liquidity_amount, true)?;
        let timestamp = env.ledger().timestamp();

        let pool = get_pool(&env);
        let position = get_position_by_ts(&env, &sender, position_ts);

        let update = controller::liquidity::calculate_modify_liquidity(
            &pool,
            &position,
            &tick_array_lower,
            &tick_array_upper,
            liquidity_delta,
            timestamp
        )?;

        controller::liquidity::sync_modify_liquidity_values(
            &mut pool,
            &mut position,
            &tick_array_lower,
            &tick_array_upper,
            update,
            timestamp
        )?;

        let (delta_a, delta_b) = controller::liquidity::calculate_liquidity_token_deltas(
            pool.tick_current_index,
            pool.sqrt_price,
            &position,
            liquidity_delta
        )?;

        if delta_a > token_max_a || delta_b > token_max_b {
            return Err(ErrorCode::TokenMaxExceeded.into());
        }

        let token_a_client = token_contract::Client::new(&env, &pool.token_a);
        let token_b_client = token_contract::Client::new(&env, &pool.token_b);

        token_a_client.transfer(&sender, &env.current_contract_address(), &delta_a);
        token_b_client.transfer(&sender, &env.current_contract_address(), &delta_b);

        // mint token

        SynthPoolEvents::add_liquidity(&env, to, amount_a, amount_b);
    }

    fn collect_fees(env: Env, sender: Address, to: Option<Address>, position_ts: u64) {
        sender.require_auth();

        // verify_position_authority_interface(
        //     &ctx.accounts.position_token_account,
        //     &ctx.accounts.position_authority
        // )?;

        let pool = get_pool(&env);
        let position = get_position_by_ts(&env, user, position_ts);

        // Store the fees owed to use as transfer amounts.
        let fee_owed_a = position.fee_owed_a;
        let fee_owed_b = position.fee_owed_b;

        position.reset_fees_owed();

        let recipient = match to {
            Some(to_address) => to_address, // Use the provided `to` address
            None => sender, // Otherwise use the sender address
        };

        pool.transfer_a(env.current_contract_address(), recipient, fee_owed_a);
        pool.transfer_b(env.current_contract_address(), recipient, fee_owed_b);

        SynthPoolEvents::collect_fees(&env, to, amount);
    }

    fn decrease_liquidity(
        env: Env,
        sender: Address,
        position_ts: u64,
        liquidity_amount: u128,
        token_max_a: u64,
        token_max_b: u64,
        tick_array_lower: TickArray,
        tick_array_upper: TickArray
    ) -> (i128, i128) {
        sender.require_auth();

        if liquidity_amount == 0 {
            return Err(ErrorCode::LiquidityZero.into());
        }
        let liquidity_delta = convert_to_liquidity_delta(liquidity_amount, true)?;
        let timestamp = env.ledger().timestamp();

        let pool = get_pool(&env);
        let position = get_position_by_ts(&env, &sender, position_ts);

        let update = controller::liquidity::calculate_modify_liquidity(
            &pool,
            &position,
            &tick_array_lower,
            &tick_array_upper,
            liquidity_delta,
            timestamp
        )?;

        controller::liquidity::sync_modify_liquidity_values(
            &mut ctx.accounts.amm,
            &mut ctx.accounts.position,
            &ctx.accounts.tick_array_lower,
            &ctx.accounts.tick_array_upper,
            update,
            timestamp
        )?;

        let (delta_a, delta_b) = controller::liquidity::calculate_liquidity_token_deltas(
            ctx.accounts.amm.tick_current_index,
            ctx.accounts.amm.sqrt_price,
            &ctx.accounts.position,
            liquidity_delta
        )?;

        if delta_a < token_max_a || delta_b < token_max_b {
            return Err(ErrorCode::TokenMinSubceeded.into());
        }

        let token_a_client = token_contract::Client::new(&env, &pool.token_a);
        let token_b_client = token_contract::Client::new(&env, &pool.token_b);

        token_a_client.transfer(&env.current_contract_address(), &sender, &delta_a);
        token_b_client.transfer(&env.current_contract_address(), &sender, &delta_b);

        SynthPoolEvents::remove_liquidity(&env, to, out_a, out_b);

        (out_a, out_b)
    }

    fn swap(
        env: Env,
        sender: Address,
        amount: u64,
        other_amount_threshold: u64,
        sqrt_price_limit: u128,
        amount_specified_is_input: bool,
        a_to_b: bool, // Zero for one,
        // other
        tick_array_0: TickArray,
        tick_array_1: TickArray,
        tick_array_2: TickArray
    ) {
        sender.require_auth();
        check_nonnegative_amount(amount);

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        let timestamp = env.ledger().timestamp();
        let config = get_pool(&env);

        let builder = SparseSwapTickSequenceBuilder::try_from(
            env,
            a_to_b,
            vec![tick_array_0, tick_array_1, tick_array_2],
            None
        )?;
        let mut swap_tick_sequence = builder.build()?;

        let swap_update = controller::swap::swap(
            &env,
            &mut swap_tick_sequence,
            amount,
            sqrt_price_limit,
            amount_specified_is_input,
            a_to_b,
            timestamp
        )?;

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

        // TODO: check price range and update oracle price by pulling/pushing collateral from Synth Market

        update_and_swap_amm(
            &env,
            sender,
            config.token_a,
            config.token_b,
            swap_update,
            a_to_b,
            timestamp
        );

        SynthPoolEvents::swap(&env, to, buy_a, out, in_max);
    }

    fn collect_reward(env: Env, sender: Address, to: Address, position_ts: u64) {
        sender.require_auth();

        // verify_position_authority_interface(
        //     &ctx.accounts.position_token_account,
        //     &ctx.accounts.position_authority
        // )?;

        let index = reward_index as usize;

        let position = &mut get_position_by_ts(&env, key, position_ts);
        let (transfer_amount, updated_amount_owed) = calculate_collect_reward(
            position.reward_infos[index],
            ctx.accounts.reward_vault.amount
        );

        position.update_reward_owed(index, updated_amount_owed);

        // pool.tr
        // transfer_from_vault_to_owner(
        //     &ctx.accounts.market,
        //     &ctx.accounts.reward_vault,
        //     &ctx.accounts.reward_owner_account,
        //     &ctx.accounts.token_program,
        //     transfer_amount
        // )
    }

    // Queries

    fn query_share_token_address(env: Env) -> Address {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        get_pool(&env).share_token
    }

    fn query_pool_info(env: Env) -> PoolResponse {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        let config = get_pool(&env);

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
