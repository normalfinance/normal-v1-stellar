use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum SyntheticTier {
    /// max insurance capped at A level
	A,
	/// max insurance capped at B level
	B,
	/// max insurance capped at C level
	C,
	/// no insurance
	Speculative,
	/// no insurance, another tranches below
	#[default]
	HighlySpeculative,
	/// no insurance, only single position allowed
	Isolated,
}
