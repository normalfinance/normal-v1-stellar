use soroban_sdk::{ contracttype, Address };

pub(crate) const MAX_FEE_BASIS_POINTS: u32 = 1000; // Maximum fee: 10% (in basis points)

pub const ADMIN: Symbol = symbol_short!("ADMIN");
pub const INDEX: Symbol = symbol_short!("INDEX");

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Index,
    Factory,
    Admin,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Swap {
    pub ask_asset: Address,
    pub offer_asset: Address,
    pub ask_asset_min_amount: Option<i128>,
}

// ################################################################
//                             INDEX
// ################################################################

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexAsset {
    /// Address of the synth market
    pub market_address: Address,
    /// The portfolio allocation of the asset
    pub weight: i128,
    pub last_updated_ts: i64,
}

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
    pub oracle: Address,
    pub oracle_source: OracleSource,
    /// Display name for the index
    pub name: String,
    /// Index token symbol
    pub symbol: String,
    /// Index token contract address
    pub index_token: Option<Address>,
    /// Address of the supported asset used to mint/redeem
    pub quote_asset: Address,
    /// Private indexes can be updated, but are only mintable by the admin and whitelist
    /// Public indexes cannot be updated, but can be minted by anyone
    pub is_public: bool,
    /// Whether a market is active, reduce only, expired, etc
    /// Affects whether users can open/close positions
    pub status: MarketStatus,
    pub paused_operations: u8,
    pub manager_fee_bps: i64,
    pub revenue_share_bps: i64,
    /// List of accounts explicitly allowed to mint the index
    pub whitelist: Option<Vec<Address>>,
    /// List of accounts blocked from minting the index
    pub blacklist: Option<Vec<Address>>,

    pub rebalance_threshold: i64,

    // NAV

    /// The NAV at the inception of the index - what the creator deposits (e.g. $1,000)
    pub base_nav: i64,
    /// The price assigned to the index at inception (e.g. $100)
    pub initial_price: i32,

    pub component_balances: Map<Address, u128>, // Token address > balance
    pub component_balance_update_ts: i64,

    pub component_weights: Vec<IndexAsset>,
    pub rebalance_ts: i64,

    pub last_updated_ts: i64,

    /// The ts when the index will be expired. Only set if index is in reduce only mode
    pub expiry_ts: i64,
    /// The price at which tokens will be redeemed. Only set if index is expired
    /// precision = PRICE_PRECISION
    pub expiry_price: i64,

    // pub total_minted: u64,
    // pub total_redeemed: i64,
}

impl Index {
    pub fn can_invest(&self, account: Address) -> bool {
        self.whitelist.contains(&account)
    }

    pub fn can_rebalance(&self) -> bool {
        self.rebalanced_ts > self.min_rebalance_ts
    }

    pub fn time_since_last_rebalance(&self) -> bool {
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

    pub fn update_asset_weight(&mut self, asset: Pubkey, new_weight: u8) -> NormalResult<> {
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

pub fn get_index(env: &Env) -> Index {
    let index = env.storage().persistent().get(&INDEX).expect("Index: Index not set");
    env.storage()
        .persistent()
        .extend_ttl(&INDEX, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);

    index
}

pub fn save_index(env: &Env, index: Index) {
    env.storage().persistent().set(&INDEX, &index);
    env.storage()
        .persistent()
        .extend_ttl(&INDEX, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

// ################################################################

#[derive(Clone, Copy, PartialEq, Debug, Eq, contracttype)]
pub enum Operation {
    Mint,
    Redeem,
    Rebalance,
    Update,
}
