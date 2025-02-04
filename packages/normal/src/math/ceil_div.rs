use num_traits::{One, Zero};

pub trait CheckedCeilDiv: Sized {
    /// Perform ceiling division
    fn checked_ceil_div(&self, rhs: Self) -> Option<Self>;
}

macro_rules! checked_impl {
    ($t:ty) => {
        impl CheckedCeilDiv for $t {
            #[track_caller]
            #[inline]
            fn checked_ceil_div(&self, rhs: $t) -> Option<$t> {
                let quotient = self.checked_div(rhs)?;

                let remainder = self.checked_rem(rhs)?;

                if remainder > <$t>::zero() {
                    quotient.checked_add(<$t>::one())
                } else {
                    Some(quotient)
                }
            }
        }
    };
}

// macro_rules! checked_impl {
//     ($t:ty) => {
//         impl CheckedCeilDiv for $t {
//             #[track_caller]
//             #[inline]
//             fn checked_ceil_div(&self, rhs: $t) -> Option<$t> {
//                 if rhs == 0 {
//                     return None; // Division by zero
//                 }

//                 let quotient = self.checked_div(rhs)?;
//                 let remainder = self.checked_rem(rhs)?;

//                 // if remainder > <$t>::zero() {
//                 //     quotient.checked_add(<$t>::one())
//                 if remainder > 0 {
//                     quotient.checked_add(1)
//                 } else {
//                     Some(quotient)
//                 }
//             }
//         }
//     };
// }

checked_impl!(u128);
checked_impl!(u64);
checked_impl!(u32);
checked_impl!(i128);
checked_impl!(i64);
checked_impl!(i32);
