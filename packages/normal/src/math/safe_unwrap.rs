use soroban_sdk::{log, Env};

use crate::error::{ErrorCode, NormalResult};
// use solana_program::msg;
// use std::panic::Location;

pub trait SafeUnwrap {
    type Item;

    fn safe_unwrap(self, env: &Env) -> NormalResult<Self::Item>;
}

impl<T> SafeUnwrap for Option<T> {
    type Item = T;

    #[track_caller]
    #[inline(always)]
    fn safe_unwrap(self, env: &Env) -> NormalResult<T> {
        match self {
            Some(v) => Ok(v),
            None => {
                log!(env, "Unwrap error thrown at {}:{}", file!(), line!());
                Err(ErrorCode::FailedUnwrap)
            }
        }
    }
}

impl<T, U> SafeUnwrap for Result<T, U> {
    type Item = T;

    #[track_caller]
    #[inline(always)]
    fn safe_unwrap(self, env: &Env) -> NormalResult<T> {
        match self {
            Ok(v) => Ok(v),
            Err(_) => {
                log!(env, "Unwrap error thrown at {}:{}", file!(), line!());
                Err(ErrorCode::FailedUnwrap)
            }
        }
    }
}
