use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum AuctionType {
    /// selling collateral from a Vault liquidation
	Collateral,
	/// selling newly minted NORM to cover Protocol Debt (the deficit from Collateral Auctions)
	Debt,
	/// selling excess synthetic token proceeds over the Insurance Fund max limit for NORM to be burned
	Surplus,
}
