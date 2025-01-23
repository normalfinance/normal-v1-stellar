#[macro_export]
macro_rules! get_struct_values {
    ($struct:expr, $($property:ident),+) => {
        {
        ($(
            $struct.$property,
        )+)
        }
    };
}

#[macro_export]
macro_rules! get_then_update_id {
    ($struct:expr, $property:ident) => {{
        let current_id = $struct.$property;
        $struct.$property = current_id.checked_add(1).or(Some(1)).unwrap();
        current_id
    }};
}
#[macro_export]
macro_rules! validate {
    ($env:expr, $assert:expr, $err:expr) => {
        {
            if ($assert) {
                Ok(())
            } else {
                let error_code: ErrorCode = $err;
                log!($env, "Error {} thrown at {}:{}", error_code, file!(), line!());
                Err(error_code)
            }
        }
    };
    (
        $env:expr,
        $assert:expr,
        $err:expr,
        $($arg:tt)+
    ) => {
        {
        if ($assert) {
            Ok(())
        } else {
            let error_code: ErrorCode = $err;
            log!($env, "Error {} thrown at {}:{}", error_code, file!(), line!());
            log!($env, $($arg)*);
            Err(error_code)
        }
        }
    };
}

#[macro_export]
macro_rules! dlog {
    ($($variable: expr),+) => {{
        $(
            log!("{}: {}", stringify!($variable), $variable);
        )+
    }};
    ($($arg:tt)+) => {{
            #[cfg(not(feature = "mainnet-beta"))]
            log!($($arg)+);
    }};
}

// #[macro_export]
// macro_rules! load_mut {
//     ($account_loader:expr) => {{
//         $account_loader.load_mut().map_err(|e| {
//             msg!("e {:?}", e);
//             let error_code = ErrorCode::UnableToLoadAccountLoader;
//             msg!("Error {} thrown at {}:{}", error_code, file!(), line!());
//             error_code
//         })
//     }};
// }

// #[macro_export]
// macro_rules! load {
//     ($account_loader:expr) => {{
//         $account_loader.load().map_err(|_| {
//             let error_code = ErrorCode::UnableToLoadAccountLoader;
//             msg!("Error {} thrown at {}:{}", error_code, file!(), line!());
//             error_code
//         })
//     }};
// }

#[macro_export]
macro_rules! safe_increment {
    ($struct:expr, $value:expr) => {{
        $struct = $struct.checked_add($value).ok_or_else(math_error!())?
    }};
}

#[macro_export]
macro_rules! safe_decrement {
    ($struct:expr, $value:expr) => {{
        $struct = $struct.checked_sub($value).ok_or_else(math_error!())?
    }};
}

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
