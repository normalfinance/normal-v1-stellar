use soroban_decimal::Decimal;
use soroban_sdk::{ contracttype, Address };

// Validate if int value is bigger then 0
#[macro_export]
macro_rules! validate_int_parameters {
    ($($arg:expr),*) => {
        {
            $(
                let value: Option<i128> = Into::<Option<_>>::into($arg);
                if let Some(val) = value {
                    if val <= 0 {
                        panic!("value cannot be less than or equal zero")
                    }
                }
            )*
        }
    };
}

// Validate all bps to be between the range 0..10_000
#[macro_export]
macro_rules! validate_bps {
    ($($value:expr),+) => {
        const MIN_BPS: i64 = 0;
        const MAX_BPS: i64 = 10_000;
        $(
            // if $value < MIN_BPS || $value > MAX_BPS {
            //     panic!("The value {} is out of range. Must be between {} and {} bps.", $value, MIN_BPS, MAX_BPS);
            // }
            assert!((MIN_BPS..=MAX_BPS).contains(&$value), "The value {} is out of range. Must be between {} and {} bps.", $value, MIN_BPS, MAX_BPS);
        )+
    };
}

pub fn is_approx_ratio(a: Decimal, b: Decimal, tolerance: Decimal) -> bool {
    let diff = (a - b).abs();
    diff <= tolerance
}

pub fn convert_i128_to_u128(input: i128) -> u128 {
    if input < 0 {
        panic!("Cannot convert i128 to u128");
    } else {
        input as u128
    }
}

pub fn convert_u128_to_i128(input: u128) -> i128 {
    if input > (i128::MAX as u128) {
        panic!("Cannot convert u128 to i128");
    } else {
        input as i128
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenInitInfo {
    pub token_a: Address,
    pub token_b: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AMMParams {
    pub admin: Address,
    tick_spacing: u16,
    initial_sqrt_price: u128,
    fee_rate: u16,
    protocol_fee_rate: u16,
    pub swap_fee_bps: i64,
    pub max_allowed_slippage_bps: i64,
    pub default_slippage_bps: i64,
    pub max_allowed_spread_bps: i64,
    pub token_init_info: TokenInitInfo,
}
