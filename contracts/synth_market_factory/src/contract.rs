use soroban_sdk::{ assert_with_error, contract, contractimpl, Address, Env };

contractmeta!(key = "Description", val = "Factory for creating new Synth Markets");

#[contract]
pub struct SynthFactory;

#[contractimpl]
impl ISynthFactory for SynthFactory {}
