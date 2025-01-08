use anchor_lang::prelude::*;
use std::{ cell::{ Ref, RefMut }, collections::VecDeque };

use crate::{
	errors::ErrorCode,
	state::{
		amm::AMM,
		synth_market::SynthMarket,
		Tick,
		TickArray,
		TickArrayType,
		TickUpdate,
		ZeroedTickArray,
		TICK_ARRAY_SIZE,
	},
	util::SwapTickSequence,
};

// In the case of an uninitialized TickArray, ZeroedTickArray is used to substitute TickArray behavior.
// Since all Tick are not initialized, it can be substituted by returning Tick::default().
pub(crate) enum ProxiedTickArray<'a> {
	Initialized(RefMut<'a, TickArray>),
	Uninitialized(ZeroedTickArray),
}

impl<'a> ProxiedTickArray<'a> {
	pub fn new_initialized(refmut: RefMut<'a, TickArray>) -> Self {
		ProxiedTickArray::Initialized(refmut)
	}

	pub fn new_uninitialized(start_tick_index: i32) -> Self {
		ProxiedTickArray::Uninitialized(ZeroedTickArray::new(start_tick_index))
	}

	pub fn start_tick_index(&self) -> i32 {
		self.as_ref().start_tick_index()
	}

	pub fn get_next_init_tick_index(
		&self,
		tick_index: i32,
		tick_spacing: u16,
		synthetic_to_quote: bool
	) -> Result<Option<i32>> {
		self
			.as_ref()
			.get_next_init_tick_index(tick_index, tick_spacing, synthetic_to_quote)
	}

	pub fn get_tick(&self, tick_index: i32, tick_spacing: u16) -> Result<&Tick> {
		self.as_ref().get_tick(tick_index, tick_spacing)
	}

	pub fn update_tick(
		&mut self,
		tick_index: i32,
		tick_spacing: u16,
		update: &TickUpdate
	) -> Result<()> {
		self.as_mut().update_tick(tick_index, tick_spacing, update)
	}

	pub fn is_min_tick_array(&self) -> bool {
		self.as_ref().is_min_tick_array()
	}

	pub fn is_max_tick_array(&self, tick_spacing: u16) -> bool {
		self.as_ref().is_max_tick_array(tick_spacing)
	}

	pub fn tick_offset(
		&self,
		tick_index: i32,
		tick_spacing: u16
	) -> Result<isize> {
		self.as_ref().tick_offset(tick_index, tick_spacing)
	}
}

impl<'a> AsRef<dyn TickArrayType + 'a> for ProxiedTickArray<'a> {
	fn as_ref(&self) -> &(dyn TickArrayType + 'a) {
		match self {
			ProxiedTickArray::Initialized(ref array) => &**array,
			ProxiedTickArray::Uninitialized(ref array) => array,
		}
	}
}

impl<'a> AsMut<dyn TickArrayType + 'a> for ProxiedTickArray<'a> {
	fn as_mut(&mut self) -> &mut (dyn TickArrayType + 'a) {
		match self {
			ProxiedTickArray::Initialized(ref mut array) => &mut **array,
			ProxiedTickArray::Uninitialized(ref mut array) => array,
		}
	}
}

enum TickArrayAccount<'info> {
	Initialized {
		tick_array_market: Pubkey,
		start_tick_index: i32,
		account_info: AccountInfo<'info>,
	},
	Uninitialized {
		pubkey: Pubkey,
		start_tick_index: Option<i32>,
	},
}

pub struct SparseSwapTickSequenceBuilder<'info> {
	// AccountInfo ownership must be kept while using RefMut.
	// This is why try_from and build are separated and SparseSwapTickSequenceBuilder struct is used.
	tick_array_accounts: Vec<TickArrayAccount<'info>>,
}

impl<'info> SparseSwapTickSequenceBuilder<'info> {
	/// Create a new SparseSwapTickSequenceBuilder from the given tick array accounts.
	///
	/// static_tick_array_account_infos and supplemental_tick_array_account_infos will be merged,
	/// and deduplicated by key. TickArray accounts can be provided in any order.
	///
	/// Even if over three tick arrays are provided, only three tick arrays are used in the single swap.
	/// The extra TickArray acts as a fallback in case the current price moves.
	///
	/// # Parameters
	/// - `amm` - AMM account
	/// - `synthetic_to_quote` - Direction of the swap
	/// - `static_tick_array_account_infos` - TickArray accounts provided through required accounts
	/// - `supplemental_tick_array_account_infos` - TickArray accounts provided through remaining accounts
	///
	/// # Errors
	/// - `DifferentAMMTickArrayAccount` - If the provided TickArray account is not for the AMM
	/// - `InvalidTickArraySequence` - If no valid TickArray account for the swap is found
	/// - `AccountNotMutable` - If the provided TickArray account is not mutable
	/// - `AccountOwnedByWrongProgram` - If the provided initialized TickArray account is not owned by this program
	/// - `AccountDiscriminatorNotFound` - If the provided TickArray account does not have a discriminator
	/// - `AccountDiscriminatorMismatch` - If the provided TickArray account has a mismatched discriminator
	pub fn try_from(
		amm: &Account<'info, AMM>,
		synthetic_to_quote: bool,
		static_tick_array_account_infos: Vec<AccountInfo<'info>>,
		supplemental_tick_array_account_infos: Option<Vec<AccountInfo<'info>>>
	) -> Result<Self> {
		let mut tick_array_account_infos = static_tick_array_account_infos;
		if
			let Some(supplemental_tick_array_account_infos) =
				supplemental_tick_array_account_infos
		{
			tick_array_account_infos.extend(supplemental_tick_array_account_infos);
		}

		// dedup by key
		tick_array_account_infos.sort_by_key(|a| a.key());
		tick_array_account_infos.dedup_by_key(|a| a.key());

		let mut initialized = vec![];
		let mut uninitialized = vec![];
		for account_info in tick_array_account_infos.into_iter() {
			let state = peek_tick_array(account_info)?;

			match &state {
				TickArrayAccount::Initialized {
					tick_array_market,
					start_tick_index,
					..
				} => {
					// has_one constraint equivalent check
					if *tick_array_market != market.key() {
						return Err(ErrorCode::DifferentAMMTickArrayAccount.into());
					}

					// TickArray accounts in initialized have been verified as:
					//   - Owned by this program
					//   - Initialized as TickArray account
					//   - Writable account
					//   - TickArray account for this AMM
					// So we can safely use these accounts.
					initialized.push((*start_tick_index, state));
				}
				TickArrayAccount::Uninitialized { pubkey: account_address, .. } => {
					// TickArray accounts in uninitialized have been verified as:
					//   - Owned by System program
					//   - Data size is zero
					//   - Writable account
					// But we are not sure if these accounts are valid TickArray PDA for this AMM,
					// so we need to check it later.
					uninitialized.push((*account_address, state));
				}
			}
		}

		let start_tick_indexes = get_start_tick_indexes(market, synthetic_to_quote);

		let mut tick_array_accounts: Vec<TickArrayAccount> = vec![];
		for start_tick_index in start_tick_indexes.iter() {
			// PDA calculation is expensive (3000 CU ~ / PDA),
			// so PDA is calculated only if not found in start_tick_index comparison.

			// find from initialized tick arrays
			if
				let Some(pos) = initialized
					.iter()
					.position(|t| t.0 == *start_tick_index)
			{
				let state = initialized.remove(pos).1;
				tick_array_accounts.push(state);
				continue;
			}

			// find from uninitialized tick arrays
			let tick_array_pda = derive_tick_array_pda(amm, *start_tick_index);
			if
				let Some(pos) = uninitialized.iter().position(|t| t.0 == tick_array_pda)
			{
				let state = uninitialized.remove(pos).1;
				if let TickArrayAccount::Uninitialized { pubkey, .. } = state {
					tick_array_accounts.push(TickArrayAccount::Uninitialized {
						pubkey,
						start_tick_index: Some(*start_tick_index),
					});
				} else {
					unreachable!("state in uninitialized must be Uninitialized");
				}
				continue;
			}

			// no more valid tickarrays for this swap
			break;
		}

		if tick_array_accounts.is_empty() {
			return Err(crate::errors::ErrorCode::InvalidTickArraySequence.into());
		}

		Ok(Self {
			tick_array_accounts,
		})
	}

	pub fn build<'a>(&'a self) -> Result<SwapTickSequence<'a>> {
		let mut proxied_tick_arrays = VecDeque::with_capacity(3);
		for tick_array_account in self.tick_array_accounts.iter() {
			match tick_array_account {
				TickArrayAccount::Initialized { account_info, .. } => {
					use std::ops::DerefMut;

					let data = account_info.try_borrow_mut_data()?;
					let tick_array_refmut = RefMut::map(data, |data| {
						bytemuck::from_bytes_mut(
							&mut data.deref_mut()[8..std::mem::size_of::<TickArray>() + 8]
						)
					});
					proxied_tick_arrays.push_back(
						ProxiedTickArray::new_initialized(tick_array_refmut)
					);
				}
				TickArrayAccount::Uninitialized { start_tick_index, .. } => {
					proxied_tick_arrays.push_back(
						ProxiedTickArray::new_uninitialized(start_tick_index.unwrap())
					);
				}
			}
		}

		Ok(
			SwapTickSequence::<'a>::new_with_proxy(
				proxied_tick_arrays.pop_front().unwrap(),
				proxied_tick_arrays.pop_front(),
				proxied_tick_arrays.pop_front()
			)
		)
	}
}

fn peek_tick_array(
	account_info: AccountInfo<'_>
) -> Result<TickArrayAccount<'_>> {
	use anchor_lang::Discriminator;

	// following process is ported from anchor-lang's AccountLoader::try_from and AccountLoader::load_mut
	// AccountLoader can handle initialized account and partially initialized (owner program changed) account only.
	// So we need to handle uninitialized account manually.

	// account must be writable
	if !account_info.is_writable {
		return Err(anchor_lang::error::ErrorCode::AccountNotMutable.into());
	}

	// uninitialized writable account (owned by system program and its data size is zero)
	if account_info.owner == &System::id() && account_info.data_is_empty() {
		return Ok(TickArrayAccount::Uninitialized {
			pubkey: *account_info.key,
			start_tick_index: None,
		});
	}

	// To avoid problems with the lifetime of the reference requested by AccountLoader (&'info AccountInfo<'info>),
	// AccountLoader is not used even after the account is found to be initialized.

	// owner program check
	if account_info.owner != &TickArray::owner() {
		return Err(
			Error::from(
				anchor_lang::error::ErrorCode::AccountOwnedByWrongProgram
			).with_pubkeys((*account_info.owner, TickArray::owner()))
		);
	}

	let data = account_info.try_borrow_data()?;
	if data.len() < TickArray::discriminator().len() {
		return Err(
			anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into()
		);
	}

	let disc_bytes = arrayref::array_ref![data, 0, 8];
	if disc_bytes != &TickArray::discriminator() {
		return Err(
			anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into()
		);
	}

	let tick_array: Ref<TickArray> = Ref::map(data, |data| {
		bytemuck::from_bytes(&data[8..std::mem::size_of::<TickArray>() + 8])
	});

	let start_tick_index = tick_array.start_tick_index;
	let market = tick_array.market;
	drop(tick_array);

	Ok(TickArrayAccount::Initialized {
		tick_array_market: market,
		start_tick_index,
		account_info,
	})
}

fn get_start_tick_indexes(
	market: &Account<Market>,
	synthetic_to_quote: bool
) -> Vec<i32> {
	let tick_current_index = amm.tick_current_index;
	let tick_spacing_u16 = amm.tick_spacing;
	let tick_spacing_i32 = amm.tick_spacing as i32;
	let ticks_in_array = TICK_ARRAY_SIZE * tick_spacing_i32;

	let start_tick_index_base =
		floor_division(tick_current_index, ticks_in_array) * ticks_in_array;
	let offset = if synthetic_to_quote {
		[0, -1, -2]
	} else {
		let shifted =
			tick_current_index + tick_spacing_i32 >=
			start_tick_index_base + ticks_in_array;
		if shifted {
			[1, 2, 3]
		} else {
			[0, 1, 2]
		}
	};

	let start_tick_indexes = offset
		.iter()
		.filter_map(|&o| {
			let start_tick_index = start_tick_index_base + o * ticks_in_array;
			if Tick::check_is_valid_start_tick(start_tick_index, tick_spacing_u16) {
				Some(start_tick_index)
			} else {
				None
			}
		})
		.collect::<Vec<i32>>();

	start_tick_indexes
}

fn floor_division(dividend: i32, divisor: i32) -> i32 {
	assert!(divisor != 0, "Divisor cannot be zero.");
	if dividend % divisor == 0 || dividend.signum() == divisor.signum() {
		dividend / divisor
	} else {
		dividend / divisor - 1
	}
}

fn derive_tick_array_pda(
	market: &Account<Market>,
	start_tick_index: i32
) -> Pubkey {
	Pubkey::find_program_address(
		&[
			b"tick_array",
			market.key().as_ref(),
			start_tick_index.to_string().as_bytes(),
		],
		&TickArray::owner()
	).0
}
