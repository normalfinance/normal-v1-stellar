use soroban_sdk::{contracterror, log, panic_with_error, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    FailedUnwrap = 1,
}

pub trait SafeUnwrap {
    type Item;

    fn safe_unwrap(self, env: &Env) -> Self::Item; // instead of Result<Self, ()> since it either returns Self or panics (no return)
}

impl<T> SafeUnwrap for Option<T> {
    type Item = T;

    #[track_caller]
    #[inline(always)]
    fn safe_unwrap(self, env: &Env) -> T {
        match self {
            Some(v) => v,
            None => {
                log!(env, "Unwrap error thrown at {}:{}", file!(), line!());
                panic_with_error!(env, ErrorCode::FailedUnwrap);
                // Err(ErrorCode::FailedUnwrap)
            }
        }
    }
}

impl<T, U> SafeUnwrap for Result<T, U> {
    type Item = T;

    #[track_caller]
    #[inline(always)]
    fn safe_unwrap(self, env: &Env) -> T {
        match self {
            Ok(v) => v,
            Err(_) => {
                log!(env, "Unwrap error thrown at {}:{}", file!(), line!());
                panic_with_error!(env, ErrorCode::FailedUnwrap);
                // Err(ErrorCode::FailedUnwrap)
            }
        }
    }
}
