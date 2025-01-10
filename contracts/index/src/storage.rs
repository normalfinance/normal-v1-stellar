use soroban_sdk::{ contracttype, Address };

pub(crate) const MAX_FEE_BASIS_POINTS: u32 = 1000; // Maximum fee: 10% (in basis points)

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Index,
    Admin,
}

// ################################################################
//                             INDEX
// ################################################################

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Index {
    /// The owner/authority of the index
    pub admin: Address,
    /// An address that can control the index on the admin's behalf. Has limited power, cant withdraw
    pub delegate: Option<Address>,
    /// An address that has the limited power to manage fees (such as updating and collecting them)
    pub fee_authority: Option<Address>,
    /// An address that has the limited power to update access control (such as the whitelist and blacklist)
    pub access_authority: Option<Address>,
    /// An address that has the limited power to manage assets and weights
    pub rebalance_authority: Option<Address>,
    /// Display name for the index
    pub name: String,
    /// Index token symbol
    pub symbol: String,
    /// Index token contract address
    pub token: Address,
    /// Private indexes can be updated, but are only mintable by the admin and whitelist
    /// Public indexes cannot be updated, but can be minted by anyone
    pub is_public: bool,
    pub assets: Vec<IndexAssetInfo>,
    /// Whether a market is active, reduce only, expired, etc
    /// Affects whether users can open/close positions
    pub status: MarketStatus,
    pub paused_operations: u8,
    pub manager_fee_bps: i64,
    pub revenue_share_bps: i64,
    /// List of accounts explicitly allowed to mint the index
    pub whitelist: Option<Vec<Pubkey>>,
    /// List of accounts blocked from minting the index
    pub blacklist: Option<Vec<Pubkey>>,
    /// The ts when the index will be expired. Only set if index is in reduce only mode
    pub expiry_ts: i64,
    /// The price at which tokens will be redeemed. Only set if index is expired
    /// precision = PRICE_PRECISION
    pub expiry_price: i64,

    pub total_minted: u64,
	pub total_redeemed: i64,
}

impl Index {
    fn new() -> Self {
		Index {
            admin: 
			authority: Pubkey::default(),
			delegate: Pubkey::default(),
			
		
			status: MarketStatus::default(),
			paused_operations: 0,

			oracle: Pubkey::default(),
			oracle_source: OracleSource::default(),

			assets: [],
			is_public: false,
			whitelist: [],
            blacklist: [],
			manager_fee: 0,
			total_manager_fees: 0,
			min_rebalance_ts: 0,
			rebalanced_ts: 0,
			updated_ts: 0,

			expiry_ts: 0,
			expiry_price: 0,
		}
	}

    pub fn can_invest(&self, account: Address) -> bool {
		self.whitelist.contains(&account)
	}

	pub fn can_rebalance(&self) -> bool {
		self.rebalanced_ts > self.min_rebalance_ts
	}

	pub fn time_since_last_rebalance(&self) -> bool {
		let clock = Clock::get()?;
		let now = clock.unix_timestamp;

		self.rebalanced_ts.safe_sub(now)
	}

	pub fn total_assets(&self) -> u8 {
		self.assets.len()
	}

	pub fn get_total_weight(&self) -> u8 {
		self.assets
			.values()
			.map(|asset| asset.weight)
			.sum::<u8>()
	}

	pub fn update_visibility(
		&mut self,
		new_visibility: IndexFundVisibility
	) -> bool {
		// TODO:
		let third_party_investors = true;

		if self.visbility && third_party_investors {
			msg!("Publc index funds cannot be updated");
			return Ok(false);
		}
		self.visbility = new_visibility;
	}

	pub fn update_asset_weight(
		&mut self,
		asset: Pubkey,
		new_weight: u8
	) -> NormalResult<> {
		if self.public {
			msg!("Publc index funds cannot be updated");
			return Ok(());
		}

		if let Some(asset) = self.assets.get_mut(asset) {
			asset.weight = new_weight;
		} else {
			msg!("Failed to update asset weight");
		}

		return Ok(());
	}
}

// ################################################################

#[derive(Clone, Copy, PartialEq, Debug, Eq, contracttype)]
pub enum Operation {
    Mint,
    Redeem,
    Rebalance,
    Update,
}

pub struct IndexAsset {
    pub market_index: u16,
    pub weight: u16, // The asset's allocation (in basis points)
    pub last_updated_ts: i64,
}
