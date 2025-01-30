use normal::{
    constants::{ PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD },
    error::{ ErrorCode, NormalResult },
};
use soroban_sdk::{ contracttype, Address, Env, Vec };

use crate::{
    position::PositionInfo,
    storage::Pool,
    tick::{
        Tick,
        TickUpdate,
        MAX_TICK_INDEX,
        MIN_TICK_INDEX,
        TICK_ARRAY_SIZE,
        TICK_ARRAY_SIZE_USIZE,
    },
};

pub trait TickArrayType {
    fn start_tick_index(&self) -> i32;

    fn get_next_init_tick_index(
        &self,
        tick_index: i32,
        tick_spacing: u32,
        a_to_b: bool
    ) -> NormalResult<Option<i32>>;

    fn get_tick(&self, tick_index: i32, tick_spacing: u32) -> NormalResult<&Tick>;

    fn update_tick(
        &mut self,
        tick_index: i32,
        tick_spacing: u32,
        update: &TickUpdate
    ) -> NormalResult<()>;

    /// Checks that this array holds the next tick index for the current tick index, given the pool's tick spacing & search direction.
    ///
    /// unshifted checks on [start, start + TICK_ARRAY_SIZE * tick_spacing)
    /// shifted checks on [start - tick_spacing, start + (TICK_ARRAY_SIZE - 1) * tick_spacing) (adjusting range by -tick_spacing)
    ///
    /// shifted == !a_to_b
    ///
    /// For a_to_b swaps, price moves left. All searchable ticks in this tick-array's range will end up in this tick's usable ticks.
    /// The search range is therefore the range of the tick-array.
    ///
    /// For b_to_a swaps, this tick-array's left-most ticks can be the 'next' usable tick-index of the previous tick-array.
    /// The right-most ticks also points towards the next tick-array. The search range is therefore shifted by 1 tick-spacing.
    fn in_search_range(&self, tick_index: i32, tick_spacing: u32, shifted: bool) -> bool {
        let mut lower = self.start_tick_index();
        let mut upper = self.start_tick_index() + TICK_ARRAY_SIZE * (tick_spacing as i32);
        if shifted {
            lower -= tick_spacing as i32;
            upper -= tick_spacing as i32;
        }
        tick_index >= lower && tick_index < upper
    }

    fn check_in_array_bounds(&self, tick_index: i32, tick_spacing: u32) -> bool {
        self.in_search_range(tick_index, tick_spacing, false)
    }

    fn is_min_tick_array(&self) -> bool {
        self.start_tick_index() <= MIN_TICK_INDEX
    }

    fn is_max_tick_array(&self, tick_spacing: u32) -> bool {
        self.start_tick_index() + TICK_ARRAY_SIZE * (tick_spacing as i32) > MAX_TICK_INDEX
    }

    fn tick_offset(&self, tick_index: i32, tick_spacing: u32) -> Result<isize, ErrorCode> {
        if tick_spacing == 0 {
            return Err(ErrorCode::InvalidTickSpacing);
        }

        Ok(get_offset(tick_index, self.start_tick_index(), tick_spacing))
    }
}

fn get_offset(tick_index: i32, start_tick_index: i32, tick_spacing: u32) -> isize {
    // TODO: replace with i32.div_floor once not experimental
    let lhs = tick_index - start_tick_index;
    let rhs = tick_spacing as i32;
    let d = lhs / rhs;
    let r = lhs % rhs;
    let o = if (r > 0 && rhs < 0) || (r < 0 && rhs > 0) { d - 1 } else { d };
    o as isize
}

#[contracttype]
#[repr(C, packed)]
pub struct TickArray {
    pub start_tick_index: i32,
    pub ticks: Vec<Tick>, // [Tick; TICK_ARRAY_SIZE_USIZE],
    pub pool: Address,
}

impl TickArray {
    pub fn new(env: &Env, pool: Address, start_tick_index: i32, tick_spacing: u32) -> Self {
        if !Tick::check_is_valid_start_tick(start_tick_index, tick_spacing) {
            // return Err(ErrorCode::InvalidStartTick.into());
        }

        TickArray {
            pool,
            ticks: Vec::new(env), // [Tick::default(); TICK_ARRAY_SIZE_USIZE],
            start_tick_index,
        }
    }
}

impl TickArray {
    /// Initialize the TickArray object
    ///
    /// # Parameters
    /// - `whirlpool` - the tick index the desired Tick object is stored in
    /// - `start_tick_index` - A u8 integer of the tick spacing for this whirlpool
    ///
    /// # Errors
    /// - `InvalidStartTick`: - The provided start-tick-index is not an initializable tick index in this Whirlpool w/ this tick-spacing.
    pub fn initialize(&mut self, pool: &Pool, pool_addr: Address, start_tick_index: i32) -> Result<(), ()> {
        if !Tick::check_is_valid_start_tick(start_tick_index, pool.tick_spacing) {
            return Err(ErrorCode::InvalidStartTick);
        }

        self.pool = pool_addr;
        self.start_tick_index = start_tick_index;
        Ok(())
    }
}

impl TickArrayType for TickArray {
    fn start_tick_index(&self) -> i32 {
        self.start_tick_index
    }

    /// Search for the next initialized tick in this array.
    ///
    /// # Parameters
    /// - `tick_index` - A i32 integer representing the tick index to start searching for
    /// - `tick_spacing` - A u8 integer of the tick spacing for this amm
    /// - `a_to_b` - If the trade is from a_to_b, the search will move to the left and the starting search tick is inclusive.
    ///              If the trade is from b_to_a, the search will move to the right and the starting search tick is not inclusive.
    ///
    /// # Returns
    /// - `Some(i32)`: The next initialized tick index of this array
    /// - `None`: An initialized tick index was not found in this array
    /// - `InvalidTickArraySequence` - error if `tick_index` is not a valid search tick for the array
    /// - `InvalidTickSpacing` - error if the provided tick spacing is 0
    fn get_next_init_tick_index(
        &self,
        tick_index: i32,
        tick_spacing: u32,
        a_to_b: bool
    ) -> NormalResult<Option<i32>> {
        if !self.in_search_range(tick_index, tick_spacing, !a_to_b) {
            return Err(ErrorCode::InvalidTickArraySequence);
        }

        let mut curr_offset = match self.tick_offset(tick_index, tick_spacing) {
            Ok(value) => value as i32,
            Err(e) => {
                return Err(e);
            }
        };

        // For a_to_b searches, the search moves to the left. The next possible init-tick can be the 1st tick in the current offset
        // For b_to_a searches, the search moves to the right. The next possible init-tick cannot be within the current offset
        if !a_to_b {
            curr_offset += 1;
        }

        while (0..TICK_ARRAY_SIZE).contains(&curr_offset) {
            let curr_tick = self.ticks[curr_offset as usize];
            if curr_tick.initialized {
                return Ok(Some(curr_offset * (tick_spacing as i32) + self.start_tick_index));
            }

            curr_offset = if a_to_b { curr_offset - 1 } else { curr_offset + 1 };
        }

        Ok(None)
    }

    /// Get the Tick object at the given tick-index & tick-spacing
    ///
    /// # Parameters
    /// - `tick_index` - the tick index the desired Tick object is stored in
    /// - `tick_spacing` - A u8 integer of the tick spacing for this amm
    ///
    /// # Returns
    /// - `&Tick`: A reference to the desired Tick object
    /// - `TickNotFound`: - The provided tick-index is not an initializable tick index in this amm w/ this tick-spacing.
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
        Ok(&self.ticks[offset as usize])
    }

    /// Updates the Tick object at the given tick-index & tick-spacing
    ///
    /// # Parameters
    /// - `tick_index` - the tick index the desired Tick object is stored in
    /// - `tick_spacing` - A u8 integer of the tick spacing for this amm
    /// - `update` - A reference to a TickUpdate object to update the Tick object at the given index
    ///
    /// # Errors
    /// - `TickNotFound`: - The provided tick-index is not an initializable tick index in this amm w/ this tick-spacing.
    fn update_tick(
        &mut self,
        tick_index: i32,
        tick_spacing: u32,
        update: &TickUpdate
    ) -> Result<(), ErrorCode> {
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
        // self.ticks
        //     .get_mut(offset as usize)
        //     .unwrap()
        //     .update(update);
        self.ticks.get(offset).unwrap().update(update);
        Ok(())
    }
}

pub fn get_tick_arrays(env: &Env, key: &Address) -> PositionInfo {
    let position_info = match env.storage().persistent().get::<_, PositionInfo>(key) {
        Some(info) => info,
        None =>
            PositionInfo {
                positions: Vec::new(env),
            },
    };
    env.storage()
        .persistent()
        .has(&key)
        .then(|| {
            env.storage()
                .persistent()
                .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
        });

    position_info
}

pub fn save_position_info(env: &Env, key: &Address, position_info: &PositionInfo) {
    env.storage().persistent().set(key, position_info);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}
