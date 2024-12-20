#![no_std]

// Imports
use soroban_sdk::{contracttype, Address};

// Define the `Ownable` trait
trait Ownable {
    fn is_owner(&self, owner: &Address) -> bool;
}

// Implement the `Ownable` trait for the `OwnableContract` struct
impl Ownable for OwnableContract {
    fn is_owner(&self, owner: &Address) -> bool {
        self.owner == *owner
    }
}

// Define a modifier that requires the caller to be the owner of the contract
fn only_owner(contract: &OwnableContract, owner: &Address) -> bool {
    contract.is_owner(owner)
}