mod token;
mod votes;
pub use votes::Client as TokenClient;
pub use token::WASM as TOKEN_WASM;
pub use votes::Client as VotesClient;
pub use votes::WASM as VOTES_WASM;
