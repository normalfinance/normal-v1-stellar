use crate::admin::{ read_administrator, write_administrator };
use crate::allowance::{ read_allowance, spend_allowance, write_allowance };
use crate::balance::{ read_balance, receive_balance, spend_balance };
use crate::metadata::{ read_decimal, read_name, read_symbol, write_metadata };
use crate::errors::ErrorCode;
#[cfg(test)]
use crate::storage_types::{ AllowanceDataKey, AllowanceValue, DataKey };
use crate::storage_types::{ INSTANCE_BUMP_AMOUNT, INSTANCE_LIFETIME_THRESHOLD };
use soroban_sdk::token::{ self, Interface as _ };
use soroban_sdk::{ contract, contractimpl, Address, Env, String };
use soroban_token_sdk::metadata::TokenMetadata;
use soroban_token_sdk::TokenUtils;

fn check_nonnegative_amount(amount: i128) {
    if amount < 0 {
        panic!("negative amount is not allowed: {}", amount)
    }
}

contractmeta!(key = "Description", val = "Token representing ownership in an crypto index fund");

#[contract]
pub struct IndexToken;

#[contractimpl]
impl IndexToken {
    pub fn __constructor(
        e: Env,
        admin: Address,
        decimal: u32,
        name: String,
        symbol: String,
        index_contract: Address
    ) {
        if decimal > 18 {
            panic!("Decimal must not be greater than 18");
        }
        write_administrator(&e, &admin);
        write_metadata(&e, TokenMetadata {
            decimal,
            name,
            symbol,
        });

        e.storage().persistent().set("index_contract", index_contract);

        // Initialize last transfer tracking
        e.storage()
            .persistent()
            .set("last_transfer", Map::<Address, (u64, i128)>::new(&e));
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        check_nonnegative_amount(amount);
        let admin = read_administrator(&e);
        admin.require_auth();

        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        receive_balance(&e, to.clone(), amount);
        TokenUtils::new(&e).events().mint(admin, to, amount);
    }

    pub fn set_admin(e: Env, new_admin: Address) {
        let admin = read_administrator(&e);
        admin.require_auth();

        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        write_administrator(&e, &new_admin);
        TokenUtils::new(&e).events().set_admin(admin, new_admin);
    }

    fn get_fees(e: &Env) -> (i128, i128, Address, Address) {
        let index_contract: Address = e.storage().persistent().get("index_contract").unwrap();

        // Cross-contract call to get protocol and manager fees
        let (protocol_fee, manager_fee, protocol_address): (
            i128,
            i128,
            Address,
        ) = e.invoke_contract(&index_contract, &"get_fees".into(), ());
        // let _admin: Address = e.invoke_contract(&index_contract, &"get_admin".into(), ());
        (protocol_fee, manager_fee, protocol_address, index_contract)
    }

    pub fn calculate_fees(
        e: &Env,
        owner: Address,
        transfer_amount: i128,
        protocol_fee: i128,
        manager_fee: i128
    ) -> (i128, i128) {
        let mut last_transfer_map: Map<Address, (u64, i128)> = e
            .storage()
            .persistent()
            .get("last_transfer")
            .unwrap();

        let current_time = e.block().timestamp();
        let (last_transfer_time, last_balance) = last_transfer_map
            .get(owner)
            .unwrap_or((current_time, 0));

        if last_balance == 0 {
            // No fee if there was no prior balance
            return 0;
        }

        // Calculate weighted holding time
        let time_held = current_time - last_transfer_time;

        // Prorated fee calculation
        let protocol_fee_amount =
            (transfer_amount * protocol_fee * (time_held as i128)) / (365 * 24 * 60 * 60 * 100); // Annualized
        let manager_fee_amount =
            (transfer_amount * manager_fee * (time_held as i128)) / (365 * 24 * 60 * 60 * 100); // Annualized

        (protocol_fee_amount, manager_fee_amount)
    }

    fn calculate_required_fees(&e: Env, from: Address, amount: i128) -> (i128, i128, i128) {
        // Calculate fee and deduct it
        let (protocol_fee, manager_fee, protocol_address, index_contract) = Self::get_fees(&e);

        // Check if the sender or receiver is exempt from fees
        let fee_exempt = from == admin || from == protocol_address || from == index_contract;

        let (protocol_fee_amount, manager_fee_amount, total_fees, net_amount) = if fee_exempt {
            (0, 0, 0, amount) // No fees applied, transfer the full amount
        } else {
            let (protocol_fee_amount, manager_fee_amount) = Self::calculate_fees(
                &e,
                from.clone(),
                amount,
                protocol_fee,
                manager_fee
            );
            let total_fees = protocol_fee_amount + manager_fee_amount;

            let net_amount = amount - total_fees;

            if net_amount <= 0 {
                return Err(ErrorCode::TransferAmountTooSmallAfterFees);
            }

            (protocol_fee_amount, manager_fee_amount, total_fees, net_amount)
        };
    }

    fn update_last_transfer(e: &Env, owner: Address, new_balance: i128) {
        let mut last_transfer_map: Map<Address, (u64, i128)> = e
            .storage()
            .persistent()
            .get("last_transfer")
            .unwrap();

        let current_time = e.block().timestamp();
        last_transfer_map.set(owner, (current_time, new_balance));
        e.storage().persistent().set("last_transfer", last_transfer_map);
    }

    #[cfg(test)]
    pub fn get_allowance(e: Env, from: Address, spender: Address) -> Option<AllowanceValue> {
        let key = DataKey::Allowance(AllowanceDataKey { from, spender });
        let allowance = e.storage().temporary().get::<_, AllowanceValue>(&key);
        allowance
    }
}

#[contractimpl]
impl token::Interface for IndexToken {
    fn allowance(e: Env, from: Address, spender: Address) -> i128 {
        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_allowance(&e, from, spender).amount
    }

    fn approve(e: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        from.require_auth();

        check_nonnegative_amount(amount);

        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        write_allowance(&e, from.clone(), spender.clone(), amount, expiration_ledger);
        TokenUtils::new(&e).events().approve(from, spender, amount, expiration_ledger);
    }

    fn balance(e: Env, id: Address) -> i128 {
        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_balance(&e, id)
    }

    fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        check_nonnegative_amount(amount);

        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // Calculate fee and deduct it
        let (protocol_fee_amount, manager_fee_amount, total_fees, net_amount) =
            Self::calculate_required_fees(&e, from, amount);

        let admin = read_administrator(&e);

        // spend_balance(&e, from.clone(), amount);
        // receive_balance(&e, to.clone(), net_amount);
        // receive_balance(&e, admin.clone(), fee);

        // Deduct from sender
        let mut from_balance: i128 = e.storage().persistent().get(from.clone()).unwrap_or(0);
        if from_balance < amount {
            panic!("Insufficient balance");
        }
        from_balance -= amount;

        // Add to recipient
        let mut to_balance: i128 = e.storage().persistent().get(to.clone()).unwrap_or(0);
        to_balance += net_amount;

        // Add protocol fee to protocol address if applicable
        if protocol_fee_amount > 0 {
            let mut protocol_balance: i128 = e
                .storage()
                .persistent()
                .get(protocol_address.clone())
                .unwrap_or(0);
            protocol_balance += protocol_fee_amount;
            e.storage().persistent().set(protocol_address.clone(), protocol_balance);
        }

        // Add manager fee to Index contract if applicable
        if manager_fee_amount > 0 {
            let mut index_balance: i128 = e
                .storage()
                .persistent()
                .get(index_contract.clone())
                .unwrap_or(0);
            index_balance += manager_fee_amount;
            e.storage().persistent().set(index_contract.clone(), index_balance);

            // Notify the Index contract about the manager fee
            e.invoke_contract::<()>(&index_contract, &"handle_manager_fee".into(), (
                manager_fee_amount,
                from.clone(),
            ));
        }

        TokenUtils::new(&e).events().transfer(from.clone(), to.clone(), net_amount);

        // Log fee events if applicable
        if protocol_fee_amount > 0 {
            TokenUtils::new(&e).events().fee(from.clone(), admin, protocol_fee_amount);
        }
        if manager_fee_amount > 0 {
            TokenUtils::new(&e).events().fee(from.clone(), index_contract, manager_fee_amount);
        }

        // Update last transfer timestamps and amounts
        Self::update_last_transfer(&e, from.clone(), from_balance);
        Self::update_last_transfer(&e, to.clone(), to_balance);

        // Notify Index contract about the fee
        // TODO: do we need this?
        e.invoke_contract::<()>(&index_contract, &"handle_manager_fee".into(), (
            manager_fee_amount,
            from.clone(),
        ));
    }

    fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();

        check_nonnegative_amount(amount);

        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // Calculate fee and deduct it
        let (protocol_fee_amount, manager_fee_amount, total_fees, net_amount) =
            Self::calculate_required_fees(&e, from, amount);

        let admin = read_administrator(&e);

        spend_allowance(&e, from.clone(), spender, amount);
        spend_balance(&e, from.clone(), amount);
        receive_balance(&e, to.clone(), net_amount);
        receive_balance(&e, protocol_address.clone(), protocol_fee_amount);
        receive_balance(&e, index_contract.clone(), manager_fee_amount);

        TokenUtils::new(&e).events().transfer(from, to, net_amount);

        // Log fee events if applicable
        if protocol_fee_amount > 0 {
            TokenUtils::new(&e).events().fee(from.clone(), admin, protocol_fee_amount);
        }
        if manager_fee_amount > 0 {
            TokenUtils::new(&e).events().fee(from.clone(), index_contract, manager_fee_amount);
        }

        // Update last transfer timestamps and amounts
        Self::update_last_transfer(&e, from.clone(), from_balance);
        Self::update_last_transfer(&e, to.clone(), to_balance);

        // Notify Index contract about the fee
        // TODO: do we need this?
        e.invoke_contract::<()>(&index_contract, &"handle_manager_fee".into(), (
            manager_fee_amount,
            from.clone(),
        ));
    }

    fn burn(e: Env, from: Address, amount: i128) {
        from.require_auth();

        check_nonnegative_amount(amount);

        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        spend_balance(&e, from.clone(), amount);
        TokenUtils::new(&e).events().burn(from, amount);
    }

    fn burn_from(e: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();

        check_nonnegative_amount(amount);

        e.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        spend_allowance(&e, from.clone(), spender, amount);
        spend_balance(&e, from.clone(), amount);
        TokenUtils::new(&e).events().burn(from, amount)
    }

    fn decimals(e: Env) -> u32 {
        read_decimal(&e)
    }

    fn name(e: Env) -> String {
        read_name(&e)
    }

    fn symbol(e: Env) -> String {
        read_symbol(&e)
    }

    fn index_contract(e: Env) -> Address {
        read_index_contract(&e)
    }
}
