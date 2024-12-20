use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

use crate::{ errors, storage::{ get_admin }, storage_types::{ DataKey } };

contractmeta!(
    key = "Description",
    val = "Constant product AMM that maintains a synthetic asset peg"
);

#[contract]
pub struct AMM;

#[contractimpl]
impl AMM {
    pub fn __constructor(
        e: Env,
        token_wasm_hash: BytesN<32>,
        token_a: Address,
        token_b: Address,
        initial_sqrt_price: u128,
        fee_rate: u16,
        protocol_fee_rate: u16
    ) {
        if token_a >= token_b {
            panic!("token_a must be less than token_b");
        }

        if !(MIN_SQRT_PRICE_X64..=MAX_SQRT_PRICE_X64).contains(&sqrt_price) {
            return Err(ErrorCode::SqrtPriceOutOfBounds.into());
        }

        if fee_rate > MAX_FEE_RATE {
            return Err(ErrorCode::FeeRateMaxExceeded.into());
        }
        if protocol_fee_rate > MAX_PROTOCOL_FEE_RATE {
            return Err(ErrorCode::ProtocolFeeRateMaxExceeded.into());
        }

        let share_contract = create_share_token(&e, token_wasm_hash, &token_a, &token_b);

        put_token_a(&e, token_a);
        put_token_b(&e, token_b);
        // put_token_share(&e, share_contract);
        // put_total_shares(&e, 0);
        put_reserve_a(&e, 0);
        put_reserve_b(&e, 0);

        put_sqrt_price(&e, initial_sqrt_price);
        put_liquidity(&e, 0);

        put_fee_rate(&e, fee_rate);
        put_protocol_fee_rate(&e, protocol_fee_rate);
    }

    // Returns the token contract address for the pool share token
    pub fn share_id(e: Env) -> Address {
        get_token_share(&e)
    }

    pub fn set_fee_rate(e: Env, fee_rate: u128) -> u128 {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_fee_rate(&e, fee_rate);
    }

    pub fn set_protocol_fee_rate(e: Env, protocol_fee_rate: u128) -> u128 {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        set_protocol_fee_rate(&e, protocol_fee_rate);
    }

    pub fn reset_oracle_twap(e: Env) -> u128 {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        // set_fee_rate(&e, fee_rate);
    }

    pub fn update_oracle_twap(e: Env) -> u128 {
        let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        // set_fee_rate(&e, fee_rate);
    }

    // Deposits token_a and token_b. Also mints pool shares for the "to" Identifier. The amount minted
    // is determined based on the difference between the reserves stored by this contract, and
    // the actual balance of token_a and token_b for this contract.
    pub fn deposit(
        e: Env,
        to: Address,
        desired_a: i128,
        min_a: i128,
        desired_b: i128,
        min_b: i128
    ) {
        // Depositor needs to authorize the deposit
        to.require_auth();

        let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));

        // Calculate deposit amounts
        let (amount_a, amount_b) = get_deposit_amounts(
            desired_a,
            min_a,
            desired_b,
            min_b,
            reserve_a,
            reserve_b
        );

        if amount_a <= 0 || amount_b <= 0 {
            // If one of the amounts can be zero, we can get into a situation
            // where one of the reserves is 0, which leads to a divide by zero.
            panic!("both amounts must be strictly positive");
        }

        let token_a_client = token::Client::new(&e, &get_token_a(&e));
        let token_b_client = token::Client::new(&e, &get_token_b(&e));

        token_a_client.transfer(&to, &e.current_contract_address(), &amount_a);
        token_b_client.transfer(&to, &e.current_contract_address(), &amount_b);

        // Now calculate how many new pool shares to mint
        let (balance_a, balance_b) = (get_balance_a(&e), get_balance_b(&e));
        let total_shares = get_total_shares(&e);

        let zero = 0;
        let new_total_shares = if reserve_a > zero && reserve_b > zero {
            let shares_a = (balance_a * total_shares) / reserve_a;
            let shares_b = (balance_b * total_shares) / reserve_b;
            shares_a.min(shares_b)
        } else {
            (balance_a * balance_b).sqrt()
        };

        mint_shares(&e, to, new_total_shares - total_shares);
        put_reserve_a(&e, balance_a);
        put_reserve_b(&e, balance_b);
    }

    pub fn collect_fees(e: Env, to: Address, fee_amount: i128) -> (i128, i128) {
        to.require_auth();

        // ...

        let fee_owed_a = 0;
        let fee_owed_b = 0;

        // reset fees owed

        transfer_a(&e, to.clone(), fee_owed_a);
        transfer_b(&e, to, fee_owed_b);

        (fee_owed_a, fee_owed_b)
    }

    // transfers share_amount of pool share tokens to this contract, burns all pools share tokens in this contracts, and sends the
    // corresponding amount of token_a and token_b to "to".
    // Returns amount of both tokens withdrawn
    pub fn withdraw(
        e: Env,
        to: Address,
        share_amount: i128,
        min_a: i128,
        min_b: i128
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

        (out_a, out_b)
    }

    // If "buy_a" is true, the swap will buy token_a and sell token_b. This is flipped if "buy_a" is false.
    // "out" is the amount being bought, with in_max being a safety to make sure you receive at least that amount.
    // swap will transfer the selling token "to" to this contract, and then the contract will transfer the buying token to "to".
    pub fn swap(e: Env, to: Address, buy_a: bool, out: i128, in_max: i128) {
        to.require_auth();

        let (reserve_a, reserve_b) = (get_reserve_a(&e), get_reserve_b(&e));
        let (reserve_sell, reserve_buy) = if buy_a {
            (reserve_b, reserve_a)
        } else {
            (reserve_a, reserve_b)
        };

        if reserve_buy < out {
            panic!("not enough token to buy");
        }

        // First calculate how much needs to be sold to buy amount out from the pool
        let n = reserve_sell * out * 1000;
        let d = (reserve_buy - out) * 997;
        let sell_amount = n / d + 1;
        if sell_amount > in_max {
            panic!("in amount is over max");
        }

        // Transfer the amount being sold to the contract
        let sell_token = if buy_a { get_token_b(&e) } else { get_token_a(&e) };
        let sell_token_client = token::Client::new(&e, &sell_token);
        sell_token_client.transfer(&to, &e.current_contract_address(), &sell_amount);

        let (balance_a, balance_b) = (get_balance_a(&e), get_balance_b(&e));

        // residue_numerator and residue_denominator are the amount that the invariant considers after
        // deducting the fee, scaled up by 1000 to avoid fractions
        let residue_numerator = 997;
        let residue_denominator = 1000;
        let zero = 0;

        let new_invariant_factor = |balance: i128, reserve: i128, out: i128| {
            let delta = balance - reserve - out;
            let adj_delta = if delta > zero {
                residue_numerator * delta
            } else {
                residue_denominator * delta
            };
            residue_denominator * reserve + adj_delta
        };

        let (out_a, out_b) = if buy_a { (out, 0) } else { (0, out) };

        let new_inv_a = new_invariant_factor(balance_a, reserve_a, out_a);
        let new_inv_b = new_invariant_factor(balance_b, reserve_b, out_b);
        let old_inv_a = residue_denominator * reserve_a;
        let old_inv_b = residue_denominator * reserve_b;

        if new_inv_a * new_inv_b < old_inv_a * old_inv_b {
            panic!("constant product invariant does not hold");
        }

        if buy_a {
            transfer_a(&e, to, out_a);
        } else {
            transfer_b(&e, to, out_b);
        }

        let new_reserve_a = balance_a - out_a;
        let new_reserve_b = balance_b - out_b;

        if new_reserve_a <= 0 || new_reserve_b <= 0 {
            panic!("new reserves must be strictly positive");
        }

        put_reserve_a(&e, new_reserve_a);
        put_reserve_b(&e, new_reserve_b);
    }

    pub fn get_rsrvs(e: Env) -> (i128, i128) {
        (get_reserve_a(&e), get_reserve_b(&e))
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

    pub fn set_reward_emissions(e: Env, id: u64, emissions_per_second_x64: u128) {
        let reward = get_reward_by_id(&e, id);

        // ...

        set_reward_emissions(&e, id, emissions_per_second_x64);
    }

    pub fn collect_reward(e: Env, to: Address) {
        to.require_auth();
    }
}
