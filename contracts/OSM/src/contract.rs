use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

use crate::{ errors, storage::{ get_admin }, storage_types::{ DataKey }, events::OracleSecurityModuleEvents };

contractmeta!(
    key = "Description",
    val = "Middleware protecting against malformed or invalid oracle price data"
);

#[contract]
struct OracleSecurityModule;

#[contractimpl]
impl OracleSecurityModule {
    pub fn __constructor(e: Env, oracle_contract_id: Address) {
        

        put_oracle_contract_id(&e, oracle_contract_id);
    }

    // pub fn set_fee_rate(e: Env, fee_rate: u128) -> u128 {
    //     let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
    //     admin.require_auth();
    //     set_fee_rate(&e, fee_rate);

    //     publish_updated_event(&e, &symbol_short!("fee"), fee);
    // }

    pub fn update_reflector_oracle(e: Env) -> u128 {}

    pub fn freeze_oracle(e: Env) -> u128 {

        OracleSecurityModuleEvents::update_oracle_status(
            &e,
            to,
            '',
            amount,
        )
    }

    pub fn unfreeze_oracle(e: Env) -> u128 {

        OracleSecurityModuleEvents::update_oracle_status(
            &e,
            to,
            '',
            amount,
        )
    }

    pub fn update_oracle_guard_rails(e: Env) -> u128 {}

    pub fn update_twap(e: Env) -> u128 {}

    pub fn update_emergency_oracles(e: Env) -> u128 {

        OracleSecurityModuleEvents::update_emergency_oracles(
            &e,
            to,
            '',
            amount,
        )
    }

    pub fn max_oracle_twap_5min_percent_divergence(&e) -> u64 {
        e.price_divergence.oracle_twap_5min_percent_divergence.max(PERCENTAGE_PRECISION_U64 / 2)
    }

    pub fn get_oracle_price(
        e: Env,
        oracle_source: &OracleSource,
        price_oracle: &AccountInfo,
        clock_slot: u64
    ) -> NormalResult<OraclePriceData> {
        match oracle_source {
            OracleSource::Reflector => Self::get_reflector_price(price_oracle, clock_slot, 1, false),
            OracleSource::QuoteAsset =>
                Ok(OraclePriceData {ƒ
                    price: PRICE_PRECISION_I64,
                    confidence: 1,
                    delay: 0,
                    has_sufficient_number_of_data_points: true,
                }),
        }
    }

    fn get_reflector_price(
        e: Env,
        price_oracle: &AccountInfo,
        clock_slot: u64,
        multiple: u128
    ) -> NormalResult<OraclePriceData> {
        // create the price oracle client instance
        let reflector_contract = PriceOracleClient::new(&env, &reflector_contract_id);

        // get oracle prcie precision
        let decimals = reflector_contract.decimals();

        // get the price
        let price = reflector_contract.lastprice(&loan.collateral_asset).unwrap(); // Asset::Other(Symbol::new(&env, "BTC"))
        // let reference_price = reflector_contract.twap(&coin, &5).unwrap();

        // --------

        let mut pyth_price_data: &[u8] = &price_oracle
            .try_borrow_data()
            .or(Err(crate::error::ErrorCode::UnableToLoadOracle))?;

        let oracle_price: i64;
        let oracle_conf: u64;
        let mut has_sufficient_number_of_data_points: bool = true;
        let mut oracle_precision: u128;
        let published_slot: u64;

        if is_pull_oracle {
            let price_message = pyth_solana_receiver_sdk::price_update::PriceUpdateV2
                ::try_deserialize(&mut pyth_price_data)
                .unwrap();
            oracle_price = price_message.price_message.price;
            oracle_conf = price_message.price_message.conf;
            oracle_precision = (10_u128).pow(price_message.price_message.exponent.unsigned_abs());
            published_slot = price_message.posted_slot;
        } else {
            let price_data = pyth_client::cast::<pyth_client::Price>(pyth_price_data);
            oracle_price = price_data.agg.price;
            oracle_conf = price_data.agg.conf;
            let min_publishers = price_data.num.min(3);
            let publisher_count = price_data.num_qt;

            #[cfg(feature = "mainnet-beta")]
            {
                has_sufficient_number_of_data_points = publisher_count >= min_publishers;
            }
            #[cfg(not(feature = "mainnet-beta"))]
            {
                has_sufficient_number_of_data_points = true;
            }

            oracle_precision = (10_u128).pow(price_data.expo.unsigned_abs());
            published_slot = price_data.valid_slot;
        }

        if oracle_precision <= multiple {
            msg!("Multiple larger than oracle precision");
            return Err(crate::error::ErrorCode::InvalidOracle);
        }
        oracle_precision = oracle_precision.safe_div(multiple)?;

        let mut oracle_scale_mult = 1;
        let mut oracle_scale_div = 1;

        if oracle_precision > PRICE_PRECISION {
            oracle_scale_div = oracle_precision.safe_div(PRICE_PRECISION)?;
        } else {
            oracle_scale_mult = PRICE_PRECISION.safe_div(oracle_precision)?;
        }

        let oracle_price_scaled = oracle_price
            .cast::<i128>()?
            .safe_mul(oracle_scale_mult.cast()?)?
            .safe_div(oracle_scale_div.cast()?)?
            .cast::<i64>()?;

        let oracle_conf_scaled = oracle_conf
            .cast::<u128>()?
            .safe_mul(oracle_scale_mult)?
            .safe_div(oracle_scale_div)?
            .cast::<u64>()?;

        let oracle_delay: i64 = clock_slot.cast::<i64>()?.safe_sub(published_slot.cast()?)?;

        Ok(OraclePriceData {
            price: oracle_price_scaled,
            confidence: oracle_conf_scaled,
            delay: oracle_delay,
            has_sufficient_number_of_data_points,
        })
    }
}
