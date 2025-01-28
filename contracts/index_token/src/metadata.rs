use soroban_sdk::{Env, String};
use soroban_token_sdk::{metadata::TokenMetadata, TokenUtils};

pub fn read_decimal(env: &Env) -> u32 {
    let util = TokenUtils::new(env);
    util.metadata().get_metadata().decimal
}

pub fn read_name(env: &Env) -> String {
    let util = TokenUtils::new(env);
    util.metadata().get_metadata().name
}

pub fn read_symbol(env: &Env) -> String {
    let util = TokenUtils::new(env);
    util.metadata().get_metadata().symbol
}

pub fn write_metadata(env: &Env, metadata: TokenMetadata) {
    let util = TokenUtils::new(env);
    util.metadata().set_metadata(&metadata);
}
