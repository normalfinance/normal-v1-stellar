use soroban_sdk::panic_with_error;

#[macro_export]
macro_rules! get_then_update_id {
    ($struct:expr, $property:ident) => {{
        let current_id = $struct.$property;
        $struct.$property = current_id.checked_add(1).or(Some(1)).unwrap();
        current_id
    }};
}

/// A macro that validates a condition, logs a message, and panics with a specific error if the condition is false
#[macro_export]
macro_rules! validate {
    ($env:expr, $condition:expr, $error:expr, $message:expr) => {
        if !$condition {
            // Log the validation failure message
            #[cfg(debug_assertions)]
            $env.log($message);
            // Panic with the specified error
            #[cfg(debug_assertions)]
            panic_with_error!($env, $error)
        }
    };
    // Version with format string and single data parameter
    ($env:expr, $condition:expr, $error:expr, $message:expr, $data:expr) => {
        {
        if !$condition {{
            #[cfg(debug_assertions)]
            $env.log(&format!($message, $data));
            #[cfg(debug_assertions)]
            panic_with_error!($env, $error)
        }}
        }
    };
    // Version with format string and multiple data parameters
    ($env:expr, $condition:expr, $error:expr, $message:expr, $($data:expr),+ $(,)?) => {
        {
        if !$condition {{
            #[cfg(debug_assertions)]
            $env.log(&format!($message, $($data),+));
            #[cfg(debug_assertions)]
            panic_with_error!($env, $error)
        }}
        }
    };
    // Variant without logging for cases where logging isn't needed
    ($env:expr, $condition:expr, $error:expr) => {
        if !$condition {
            #[cfg(debug_assertions)]
            panic_with_error!($env, $error)
        }
    };
}

#[macro_export]
macro_rules! safe_increment {
    ($struct:expr, $value:expr, $env:expr) => {{
        $struct = $struct.checked_add($value).unwrap_or_else(|| {
            #[cfg(debug_assertions)]
            panic_with_error!($env, $crate::safe_math::Error::MathError);
            $struct
        });
    }};
}

#[macro_export]
macro_rules! safe_decrement {
    ($struct:expr, $value:expr, $env:expr) => {{
        $struct = $struct.checked_sub($value).unwrap_or_else(|| {
            #[cfg(debug_assertions)]
            panic_with_error!($env, $crate::safe_math::Error::MathError);
            $struct
        });
    }};
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
