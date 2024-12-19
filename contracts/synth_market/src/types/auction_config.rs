use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum AuctionConfig {
    /// where collateral auctions should take place (3rd party AMM vs private)
	pub auction_location: AuctionPreference,
	/// Maximum time allowed for the auction to complete.
	pub auction_duration: u16,
	/// Determines how quickly the starting price decreases during the auction if there are no bids.
	pub auction_bid_decrease_rate: u16,
	/// May be capped to prevent overly large auctions that could affect the market price.
	pub max_auction_lot_size: u64,
}
