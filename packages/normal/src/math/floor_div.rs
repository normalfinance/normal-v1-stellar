// use num_traits::{One, Zero};

pub trait CheckedFloorDiv: Sized {
    /// Perform floor division
    fn checked_floor_div(&self, rhs: Self) -> Option<Self>;
}

macro_rules! checked_impl {
    ($t:ty) => {
        impl CheckedFloorDiv for $t {
            #[track_caller]
            #[inline]
            fn checked_floor_div(&self, rhs: $t) -> Option<$t> {
                if rhs == 0 {
                    return None; // Division by zero
                }

                let quotient = self.checked_div(rhs)?;
                let remainder = self.checked_rem(rhs)?;

                // if remainder != <$t>::zero() {
                //     quotient.checked_sub(<$t>::one())
                if remainder != 0 && (*self < 0 || rhs < 0) {
                    quotient.checked_sub(1)
                } else {
                    Some(quotient)
                }
            }
        }
    };
}

checked_impl!(i128);
checked_impl!(i64);
checked_impl!(i32);

#[cfg(test)]
mod test {
    use crate::math::floor_div::CheckedFloorDiv;

    #[test]
    fn test() {
        let x = -3_i128;

        assert_eq!(x.checked_floor_div(2), Some(-2));
        assert_eq!(x.checked_floor_div(0), None);
    }
}
