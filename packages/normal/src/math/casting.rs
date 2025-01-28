use soroban_sdk::{log, Env};

use crate::error::{ErrorCode, NormalResult};
use crate::math::bn::U192;
// use solana_program::msg;
// use std::convert::TryInto;
// use std::panic::Location;

pub trait Cast: Sized {
    /// Perform a casting operation with error handling.
    fn cast<T: CastFrom<Self>>(self, env: &Env) -> NormalResult<T> {
        T::cast_from(self, env)
    }
}

pub trait CastFrom<T>: Sized {
    fn cast_from(value: T, env: &Env) -> NormalResult<Self>;
}

// Implement CastFrom for primitive types
macro_rules! impl_cast {
    ($src:ty, $dst:ty) => {
        impl CastFrom<$src> for $dst {
            fn cast_from(value: $src, env: &Env) -> NormalResult<Self> {
                value.try_into().map_err(|_| {
                    log!(
                        env,
                        "Casting error: Failed to cast {} to {}",
                        stringify!($src),
                        stringify!($dst)
                    );
                    ErrorCode::CastingFailure
                })
            }
        }
    };
}

// pub trait Cast: Sized {
//     #[track_caller]
//     #[inline(always)]
//     fn cast<T: std::convert::TryFrom<Self>>(self) -> NormalResult<T> {
//         match self.try_into() {
//             Ok(result) => Ok(result),
//             Err(_) => {
//                 log!(
//                     "Casting error thrown at {}:{}",
//                     file!(),
//                     line!()
//                 );
//                 Err(ErrorCode::CastingFailure)
//             }
//         }
//     }
// }

// Implement for common casting scenarios
impl_cast!(U192, u128);
impl_cast!(u128, u64);
impl_cast!(u128, i128);
impl_cast!(u64, u32);
impl_cast!(u64, i64);
impl_cast!(u64, u128);
impl_cast!(u64, i128);
impl_cast!(u32, u16);
impl_cast!(u32, i128);
impl_cast!(u128, u32);
impl_cast!(u128, usize);

impl_cast!(i128, i64);
impl_cast!(i128, u128);
impl_cast!(i64, i32);
impl_cast!(i64, i128);
impl_cast!(i64, u64);
impl_cast!(i32, i16);
impl_cast!(i32, i128);
impl_cast!(i128, i32);

// Cast trait implementations for types
impl Cast for U192 {}
impl Cast for u128 {}
impl Cast for u64 {}
impl Cast for u32 {}
// impl Cast for u16 {}
// impl Cast for u8 {}
impl Cast for usize {}
impl Cast for i128 {}
impl Cast for i64 {}
impl Cast for i32 {}
// impl Cast for i16 {}
// impl Cast for i8 {}
impl Cast for bool {}
