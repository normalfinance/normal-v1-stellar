// Max & min tick index based on sqrt(1.0001) & max.min price of 2^64
pub(crate) const MAX_TICK_INDEX: i32 = 443636;
pub(crate) const MIN_TICK_INDEX: i32 = -443636;

// We have two consts because most of our code uses it as a i32. However,
// for us to use it in tick array declarations, anchor requires it to be a usize.
pub(crate) const TICK_ARRAY_SIZE: i32 = 88;
pub(crate) const TICK_ARRAY_SIZE_USIZE: usize = 88;

// Number of rewards supported by AMMs
pub(crate) const MAX_REWARDS: usize = 3;
