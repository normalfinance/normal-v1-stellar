use soroban_sdk::{log, Env};

use crate::error::{ErrorCode, NormalResult};
use crate::math::ceil_div::CheckedCeilDiv;
use crate::math::floor_div::CheckedFloorDiv;

pub trait SafeMath: Sized {
    fn safe_add(self, rhs: Self, env: &Env) -> NormalResult<Self>;
    fn safe_sub(self, rhs: Self, env: &Env) -> NormalResult<Self>;
    fn safe_mul(self, rhs: Self, env: &Env) -> NormalResult<Self>;
    fn safe_div(self, rhs: Self, env: &Env) -> NormalResult<Self>;
    fn safe_div_ceil(self, rhs: Self, env: &Env) -> NormalResult<Self>;
}

macro_rules! checked_impl {
    ($t:ty) => {
        impl SafeMath for $t {
            #[track_caller]
            #[inline(always)]
            fn safe_add(self, v: $t, env: &Env) -> NormalResult<$t> {
                match self.checked_add(v) {
                    Some(result) => Ok(result),
                    None => {
                        log!(env, "Math error thrown at {}:{}", file!(), line!());
                        Err(ErrorCode::MathError)
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_sub(self, v: $t, env: &Env) -> NormalResult<$t> {
                match self.checked_sub(v) {
                    Some(result) => Ok(result),
                    None => {
                        log!(env, "Math error thrown at {}:{}", file!(), line!());
                        Err(ErrorCode::MathError)
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_mul(self, v: $t, env: &Env) -> NormalResult<$t> {
                match self.checked_mul(v) {
                    Some(result) => Ok(result),
                    None => {
                        log!(env, "Math error thrown at {}:{}", file!(), line!());
                        Err(ErrorCode::MathError)
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_div(self, v: $t, env: &Env) -> NormalResult<$t> {
                match self.checked_div(v) {
                    Some(result) => Ok(result),
                    None => {
                        log!(env, "Math error thrown at {}:{}", file!(), line!());
                        Err(ErrorCode::MathError)
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_div_ceil(self, v: $t, env: &Env) -> NormalResult<$t> {
                match self.checked_ceil_div(v) {
                    Some(result) => Ok(result),
                    None => {
                        log!(env, "Math error thrown at {}:{}", file!(), line!());
                        Err(ErrorCode::MathError)
                    }
                }
            }
        }
    };
}

checked_impl!(u128);
checked_impl!(u64);
checked_impl!(u32);
checked_impl!(i128);
checked_impl!(i64);
checked_impl!(i32);

pub trait SafeDivFloor: Sized {
    /// Perform floor division
    fn safe_div_floor(self, rhs: Self, env: &Env) -> NormalResult<Self>;
}

macro_rules! div_floor_impl {
    ($t:ty) => {
        impl SafeDivFloor for $t {
            #[track_caller]
            #[inline(always)]
            fn safe_div_floor(self, v: $t, env: &Env) -> NormalResult<$t> {
                match self.checked_floor_div(v) {
                    Some(result) => Ok(result),
                    None => {
                        log!(env, "Math error thrown at {}:{}", file!(), line!());
                        Err(ErrorCode::MathError)
                    }
                }
            }
        }
    };
}

div_floor_impl!(i128);
div_floor_impl!(i64);
div_floor_impl!(i32);
// div_floor_impl!(i16);
// div_floor_impl!(i8);

#[cfg(test)]
mod test {
    use crate::error::ErrorCode;
    use crate::math::safe_math::{SafeDivFloor, SafeMath};
    use soroban_sdk::Env;

    #[test]
    fn safe_add() {
        let env = Env::default();
        assert_eq!((1_u128).safe_add(1, &env).unwrap(), 2);
        assert_eq!(
            (1_u128).safe_add(u128::MAX, &env),
            Err(ErrorCode::MathError)
        );
    }

    #[test]
    fn safe_sub() {
        let env = Env::default();
        assert_eq!((1_u128).safe_sub(1, &env).unwrap(), 0);
        assert_eq!((0_u128).safe_sub(1, &env), Err(ErrorCode::MathError));
    }

    #[test]
    fn safe_mul() {
        let env = Env::default();
        assert_eq!((8_u128).safe_mul(80, &env).unwrap(), 640);
        assert_eq!((1_u128).safe_mul(1, &env).unwrap(), 1);
        assert_eq!(
            (2_u128).safe_mul(u128::MAX, &env),
            Err(ErrorCode::MathError)
        );
    }

    #[test]
    fn safe_div() {
        let env = Env::default();
        assert_eq!((155_u128).safe_div(8, &env).unwrap(), 19);
        assert_eq!((159_u128).safe_div(8, &env).unwrap(), 19);
        assert_eq!((160_u128).safe_div(8, &env).unwrap(), 20);

        assert_eq!((1_u128).safe_div(1, &env).unwrap(), 1);
        assert_eq!((1_u128).safe_div(100, &env).unwrap(), 0);
        assert_eq!((1_u128).safe_div(0, &env), Err(ErrorCode::MathError));
    }

    #[test]
    fn safe_div_floor() {
        let env = Env::default();
        assert_eq!((-155_i128).safe_div_floor(8, &env).unwrap(), -20);
        assert_eq!((-159_i128).safe_div_floor(8, &env).unwrap(), -20);
        assert_eq!((-160_i128).safe_div_floor(8, &env).unwrap(), -20);
    }
}
