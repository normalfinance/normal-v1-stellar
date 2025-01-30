use normal::error::{ ErrorCode, NormalResult };
use soroban_sdk::{ contracttype, Vec };

use super::tick_array::TickArrayType;

// Max & min tick index based on sqrt(1.0001) & max.min price of 2^64
pub const MAX_TICK_INDEX: i32 = 443636;
pub const MIN_TICK_INDEX: i32 = -443636;

// We have two consts because most of our code uses it as a i32. However,
// for us to use it in tick array declarations, anchor requires it to be a usize.
pub const TICK_ARRAY_SIZE: i32 = 88;
pub const TICK_ARRAY_SIZE_USIZE: usize = 88;

#[contracttype]
#[derive(Default, Debug, PartialEq)]
pub struct Tick {
    // Total 137 bytes
    pub initialized: bool, // 1
    pub liquidity_net: i128, // 16
    pub liquidity_gross: u128, // 16

    // Q64.64
    pub fee_growth_outside_a: u128, // 16
    // Q64.64
    pub fee_growth_outside_b: u128, // 16

    // Array of Q64.64
    pub reward_growths_outside: Vec<u128>, // 48 = 16 * 3
}

impl Tick {
    /// Apply an update for this tick
    ///
    /// # Parameters
    /// - `update` - An update object to update the values in this tick
    pub fn update(&mut self, update: &TickUpdate) {
        self.initialized = update.initialized;
        self.liquidity_net = update.liquidity_net;
        self.liquidity_gross = update.liquidity_gross;
        self.fee_growth_outside_a = update.fee_growth_outside_a;
        self.fee_growth_outside_b = update.fee_growth_outside_b;
        self.reward_growths_outside = update.reward_growths_outside;
    }

    /// Check that the tick index is within the supported range of this contract
    ///
    /// # Parameters
    /// - `tick_index` - A i32 integer representing the tick index
    ///
    /// # Returns
    /// - `true`: The tick index is not within the range supported by this contract
    /// - `false`: The tick index is within the range supported by this contract
    pub fn check_is_out_of_bounds(tick_index: i32) -> bool {
        !(MIN_TICK_INDEX..=MAX_TICK_INDEX).contains(&tick_index)
    }

    /// Check that the tick index is a valid start tick for a tick array in this amm
    /// A valid start-tick-index is a multiple of tick_spacing & number of ticks in a tick-array.
    ///
    /// # Parameters
    /// - `tick_index` - A i32 integer representing the tick index
    /// - `tick_spacing` - A u8 integer of the tick spacing for this amm
    ///
    /// # Returns
    /// - `true`: The tick index is a valid start-tick-index for this amm
    /// - `false`: The tick index is not a valid start-tick-index for this amm
    ///            or the tick index not within the range supported by this contract
    pub fn check_is_valid_start_tick(tick_index: i32, tick_spacing: u32) -> bool {
        let ticks_in_array = TICK_ARRAY_SIZE * (tick_spacing as i32);

        if Tick::check_is_out_of_bounds(tick_index) {
            // Left-edge tick-array can have a start-tick-index smaller than the min tick index
            if tick_index > MIN_TICK_INDEX {
                return false;
            }

            let min_array_start_index =
                MIN_TICK_INDEX - ((MIN_TICK_INDEX % ticks_in_array) + ticks_in_array);
            return tick_index == min_array_start_index;
        }
        tick_index % ticks_in_array == 0
    }

    /// Check that the tick index is within bounds and is a usable tick index for the given tick spacing.
    ///
    /// # Parameters
    /// - `tick_index` - A i32 integer representing the tick index
    /// - `tick_spacing` - A u8 integer of the tick spacing for this amm
    ///
    /// # Returns
    /// - `true`: The tick index is within max/min index bounds for this protocol and is a usable tick-index given the tick-spacing
    /// - `false`: The tick index is out of bounds or is not a usable tick for this tick-spacing
    pub fn check_is_usable_tick(tick_index: i32, tick_spacing: u32) -> bool {
        if Tick::check_is_out_of_bounds(tick_index) {
            return false;
        }

        tick_index % (tick_spacing as i32) == 0
    }

    pub fn full_range_indexes(tick_spacing: u32) -> (i32, i32) {
        let lower_index = (MIN_TICK_INDEX / (tick_spacing as i32)) * (tick_spacing as i32);
        let upper_index = (MAX_TICK_INDEX / (tick_spacing as i32)) * (tick_spacing as i32);
        (lower_index, upper_index)
    }

    /// Bound a tick-index value to the max & min index value for this protocol
    ///
    /// # Parameters
    /// - `tick_index` - A i32 integer representing the tick index
    ///
    /// # Returns
    /// - `i32` The input tick index value but bounded by the max/min value of this protocol.
    pub fn bound_tick_index(tick_index: i32) -> i32 {
        tick_index.clamp(MIN_TICK_INDEX, MAX_TICK_INDEX)
    }
}

#[contracttype]
#[derive(Default, Debug, PartialEq)]
pub struct TickUpdate {
    pub initialized: bool,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_a: u128,
    pub fee_growth_outside_b: u128,
    pub reward_growths_outside: Vec<u128>,
}

impl TickUpdate {
    pub fn from(tick: &Tick) -> TickUpdate {
        TickUpdate {
            initialized: tick.initialized,
            liquidity_net: tick.liquidity_net,
            liquidity_gross: tick.liquidity_gross,
            fee_growth_outside_a: tick.fee_growth_outside_a,
            fee_growth_outside_b: tick.fee_growth_outside_b,
            reward_growths_outside: tick.reward_growths_outside,
        }
    }
}

#[contracttype]
pub(crate) struct ZeroedTickArray {
    pub start_tick_index: i32,
    zeroed_tick: Tick,
}

impl ZeroedTickArray {
    pub fn new(start_tick_index: i32) -> Self {
        ZeroedTickArray {
            start_tick_index,
            zeroed_tick: Tick::default(),
        }
    }
}

impl TickArrayType for ZeroedTickArray {
    fn start_tick_index(&self) -> i32 {
        self.start_tick_index
    }

    fn get_next_init_tick_index(
        &self,
        tick_index: i32,
        tick_spacing: u32,
        a_to_b: bool
    ) -> NormalResult<Option<i32>> {
        if !self.in_search_range(tick_index, tick_spacing, !a_to_b) {
            return Err(ErrorCode::InvalidTickArraySequence);
        }

        self.tick_offset(tick_index, tick_spacing)?;

        // no initialized tick
        Ok(None)
    }

    fn get_tick(&self, tick_index: i32, tick_spacing: u32) -> Result<&Tick, ErrorCode> {
        if
            !self.check_in_array_bounds(tick_index, tick_spacing) ||
            !Tick::check_is_usable_tick(tick_index, tick_spacing)
        {
            return Err(ErrorCode::TickNotFound);
        }
        let offset = self.tick_offset(tick_index, tick_spacing)?;
        if offset < 0 {
            return Err(ErrorCode::TickNotFound);
        }

        // always return the zeroed tick
        Ok(&self.zeroed_tick)
    }

    fn update_tick(
        &mut self,
        _tick_index: i32,
        _tick_spacing: u32,
        _update: &TickUpdate
    ) -> NormalResult<()> {
        panic!("ZeroedTickArray must not be updated");
    }
}
