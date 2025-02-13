use normal::{
    constants::{FIVE_MINUTE, SPOT_MARKET_TOKEN_TWAP_WINDOW},
    math::{casting::Cast, safe_math::SafeMath},
    oracle::OraclePriceData,
    validate,
};
use soroban_sdk::Env;

use crate::{
    errors::Errors,
    math::{
        balance::{calculate_utilization, get_balance, get_token_amount, BalanceType},
        stats::{calculate_new_twap, calculate_weighted_average},
    },
    state::{market::Market, market_position::MarketPosition},
};

pub fn update_market_twap_stats(
    env: &Env,
    market: &mut Market,
    collateral_oracle_price_data: &OraclePriceData,
    debt_oracle_price_data: &OraclePriceData,
    now: i64,
) {
    let since_last = max(0_i64, now.safe_sub(market.last_twap_ts.cast(env), env));
    let from_start = max(
        1_i64,
        SPOT_MARKET_TOKEN_TWAP_WINDOW.safe_sub(since_last, env),
    );

    let collateral_token_amount = get_token_amount(
        env,
        market.collateral.balance,
        market,
        &BalanceType::Deposit,
    );

    let debt_token_amount =
        get_token_amount(env, market.synthetic.balance, market, &BalanceType::Borrow);

    market.collateral.token_twap = calculate_weighted_average(
        env,
        collateral_token_amount.cast(env),
        market.collateral.token_twap.cast(env),
        since_last,
        from_start,
    )
    .cast(env);

    market.synthetic.token_twap = calculate_weighted_average(
        env,
        debt_token_amount.cast(env),
        market.synthetic.token_twap.cast(env),
        since_last,
        from_start,
    )
    .cast(env);

    let utilization = calculate_utilization(
        env,
        collateral_token_amount,
        debt_token_amount,
        collateral_oracle_price_data.price,
        debt_oracle_price_data.price,
        18, // TODO:
    );

    market.utilization_twap = calculate_weighted_average(
        env,
        utilization.cast(env),
        market.utilization_twap.cast(env),
        since_last,
        from_start,
    )
    .cast(env);

    let sanitize_clamp_denominator = market.get_sanitize_clamp_denominator();

    let capped_oracle_update_price: i64 = sanitize_new_price(
        debt_oracle_price_data.price,
        market.amm.historical_oracle_data.last_oracle_price_twap,
        sanitize_clamp_denominator,
    );

    let oracle_price_twap = calculate_new_twap(
        env,
        capped_oracle_update_price,
        now,
        market.amm.historical_oracle_data.last_oracle_price_twap,
        market.amm.historical_oracle_data.last_oracle_price_twap_ts,
        FIVE_MINUTE as i64,
    );

    market.amm.historical_oracle_data.last_oracle_price_twap = oracle_price_twap;

    market.amm.historical_oracle_data.last_oracle_price = debt_oracle_price_data.price;
    market.amm.historical_oracle_data.last_oracle_conf = debt_oracle_price_data.confidence;
    market.amm.historical_oracle_data.last_oracle_delay = debt_oracle_price_data.delay;
    market.amm.historical_oracle_data.last_oracle_price_twap_ts = now;

    market.last_twap_ts = now.cast(env);
}

// pub fn update_revenue_pool_balances(
//     token_amount: u128,
//     update_direction: &BalanceType,
//     spot_market: &mut SpotMarket
// ) -> DriftResult {
//     let mut spot_balance = spot_market.revenue_pool;
//     update_spot_balances(token_amount, update_direction, spot_market, &mut spot_balance, false);
//     spot_market.revenue_pool = spot_balance;

//     Ok(())
// }

pub fn update_balances(
    env: &Env,
    mut token_amount: u128,
    update_direction: &BalanceType,
    market: &mut Market,
    position: &mut MarketPosition,
    is_leaving_normal: bool,
) {
    // self.balance = self.balance.safe_sub(delta.cast(env), env);

    let increase_user_existing_balance = update_direction == position.balance_type;
    if increase_user_existing_balance {
        let round_up = position.balance_type == &BalanceType::Borrow;
        let balance_delta = get_balance(env, token_amount, market, update_direction, round_up);

        // balance.increase_balance(balance_delta);
        position.collateral_balance = position.collateral_balance.safe_sub(balance_delta, env);

        increase_balance(env, balance_delta, market, update_direction);
    } else {
        let current_token_amount =
            get_token_amount(env, position.balance(), market, position.balance_type);

        let reduce_user_existing_balance = current_token_amount != 0;
        if reduce_user_existing_balance {
            // determine how much to reduce balance based on size of current token amount
            let (token_delta, balance_delta) = if current_token_amount > token_amount {
                let round_up = is_leaving_normal || balance.balance_type() == &BalanceType::Borrow;
                let balance_delta =
                    get_balance(token_amount, market, balance.balance_type(), round_up);
                (token_amount, balance_delta)
            } else {
                (current_token_amount, balance.balance())
            };

            decrease_balance(balance_delta, market, balance.balance_type());
            balance.decrease_balance(balance_delta);
            token_amount = token_amount.safe_sub(token_delta);
        }

        if token_amount > 0 {
            balance.update_balance_type(*update_direction);
            let round_up = update_direction == &BalanceType::Borrow;
            let balance_delta = get_balance(token_amount, market, update_direction, round_up);
            balance.increase_balance(balance_delta);
            increase_balance(balance_delta, market, update_direction);
        }
    }

    if is_leaving_normal && update_direction == &BalanceType::Borrow {
        let collateral_token_amount = get_token_amount(
            env,
            market.collateral.balance,
            market,
            &BalanceType::Deposit,
        );

        let synthetic_token_amount =
            get_token_amount(env, market.synthetic.balance, market, &BalanceType::Borrow);

        validate!(
            collateral_token_amount >= synthetic_token_amount,
            Errors::SpotMarketInsufficientDeposits,
            "Market has insufficent collateral to complete withdraw: collateral ({}) synthetic ({})",
            collateral_token_amount,
            synthetic_token_amount
        );
    }
}

// pub fn transfer_spot_balances(
//     token_amount: i128,
//     spot_market: &mut SpotMarket,
//     from_spot_balance: &mut dyn SpotBalance,
//     to_spot_balance: &mut dyn SpotBalance,
// ) -> DriftResult {
//     validate!(
//         from_spot_balance.market_index() == to_spot_balance.market_index(),
//         ErrorCode::UnequalMarketIndexForSpotTransfer,
//         "transfer market indexes arent equal",
//     );

//     if token_amount == 0 {
//         return Ok(());
//     }

//     if from_spot_balance.balance_type() == &BalanceType::Deposit {
//         validate!(
//             spot_market.deposit_balance >= from_spot_balance.balance(),
//             ErrorCode::InvalidSpotMarketState,
//             "spot_market.deposit_balance={} lower than individual spot balance={}",
//             spot_market.deposit_balance,
//             from_spot_balance.balance()
//         );
//     }

//     update_spot_balances(
//         token_amount.unsigned_abs(),
//         if token_amount < 0 {
//             &BalanceType::Deposit
//         } else {
//             &BalanceType::Borrow
//         },
//         spot_market,
//         from_spot_balance,
//         false,
//     );

//     update_spot_balances(
//         token_amount.unsigned_abs(),
//         if token_amount < 0 {
//             &BalanceType::Borrow
//         } else {
//             &BalanceType::Deposit
//         },
//         spot_market,
//         to_spot_balance,
//         false,
//     );

//     Ok(())
// }

// pub fn transfer_revenue_pool_to_spot_balance(
//     token_amount: u128,
//     spot_market: &mut SpotMarket,
//     to_spot_balance: &mut dyn SpotBalance,
// )  {
//     validate!(
//         to_spot_balance.market_index() == spot_market.market_index,
//         ErrorCode::UnequalMarketIndexForSpotTransfer,
//         "transfer market indexes arent equal"
//     );

//     update_revenue_pool_balances(token_amount, &BalanceType::Borrow, spot_market);

//     update_balances(
//         token_amount,
//         &BalanceType::Deposit,
//         spot_market,
//         to_spot_balance,
//         false,
//     );
// }

// pub fn transfer_spot_balance_to_revenue_pool(
//     token_amount: u128,
//     spot_market: &mut SpotMarket,
//     from_spot_balance: &mut dyn SpotBalance,
// ) -> DriftResult {
//     validate!(
//         from_spot_balance.market_index() == spot_market.market_index,
//         ErrorCode::UnequalMarketIndexForSpotTransfer,
//         "transfer market indexes arent equal"
//     );

//     update_spot_balances(
//         token_amount,
//         &BalanceType::Borrow,
//         spot_market,
//         from_spot_balance,
//         false,
//     );

//     update_revenue_pool_balances(token_amount, &BalanceType::Deposit, spot_market);

//     Ok(())
// }

// pub fn update_spot_market_and_check_validity(
//     spot_market: &mut SpotMarket,
//     oracle_price_data: &OraclePriceData,
//     validity_guard_rails: &ValidityGuardRails,
//     now: i64,
//     action: Option<NormalAction>,
// ) -> DriftResult {
//     // update spot market EMAs with new/current data
//     update_spot_market_cumulative_interest(spot_market, Some(oracle_price_data), now);

//     if spot_market.market_index == QUOTE_SPOT_MARKET_INDEX {
//         return Ok(());
//     }

//     // 1 hour EMA
//     let risk_ema_price = spot_market.historical_oracle_data.last_oracle_price_twap;

//     let oracle_validity = oracle_validity(
//         MarketType::Spot,
//         spot_market.market_index,
//         risk_ema_price,
//         oracle_price_data,
//         validity_guard_rails,
//         spot_market.get_max_confidence_interval_multiplier(),
//         false,
//     );

//     validate!(
//         is_oracle_valid_for_action(oracle_validity, action),
//         ErrorCode::InvalidOracle,
//         "Invalid Oracle ({:} vs ema={:}) for spot market index={} and action={:}",
//         oracle_price_data,
//         risk_ema_price,
//         spot_market.market_index,
//         action
//     );

//     Ok(())
// }

fn increase_balance(env: &Env, delta: u128, market: &mut Market, balance_type: &BalanceType) {
    match balance_type {
        BalanceType::Deposit => {
            market.collateral.balance = market.collateral.balance.safe_add(delta, env);
        }
        BalanceType::Borrow => {
            market.synthetic.balance = market.synthetic.balance.safe_add(delta, env);
        }
    }
}

fn decrease_balance(env: &Env, delta: u128, market: &mut Market, balance_type: &BalanceType) {
    match balance_type {
        BalanceType::Deposit => {
            market.collateral.balance = market.collateral.balance.safe_sub(delta, env);
        }
        BalanceType::Borrow => {
            market.synthetic.balance = market.synthetic.balance.safe_sub(delta, env);
        }
    }
}
