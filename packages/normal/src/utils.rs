use core::fmt;
use soroban_decimal::Decimal;

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
