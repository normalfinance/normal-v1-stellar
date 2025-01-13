use crate::admin::{ read_administrator, write_administrator };
use crate::allowance::{ read_allowance, spend_allowance, write_allowance };
use crate::balance::{
    read_balance,
    read_index_contract,
    receive_balance,
    spend_balance,
    write_index_contract,
};
use normal::error::ErrorCode;
use crate::metadata::{ read_decimal, read_name, read_symbol, write_metadata };
#[cfg(test)]
use crate::storage_types::{ AllowanceDataKey, AllowanceValue, DataKey };
use crate::storage_types::{
    get_last_transfer_info,
    save_last_transfer_info,
    LastTransferInfo,
    INSTANCE_BUMP_AMOUNT,
    INSTANCE_LIFETIME_THRESHOLD,
};
use soroban_sdk::token::{ self, Interface as _ };
use soroban_sdk::{ contract, contractimpl, contractmeta, panic_with_error, Address, Env, String, Symbol, Vec };
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
        env: Env,
        admin: Address,
        decimal: u32,
        name: String,
        symbol: String,
        index_contract: Address
    ) {
        if decimal > 18 {
            panic!("Decimal must not be greater than 18");
        }
        write_administrator(&env, &admin);
        write_metadata(&env, TokenMetadata {
            decimal,
            name,
            symbol,
        });
        write_index_contract(&env, &index_contract);
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        check_nonnegative_amount(amount);
        let admin = read_administrator(&env);
        admin.require_auth();

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        receive_balance(&env, to.clone(), amount);
        TokenUtils::new(&env).events().mint(admin, to, amount);
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let admin = read_administrator(&env);
        admin.require_auth();

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        write_administrator(&env, &new_admin);
        TokenUtils::new(&env).events().set_admin(admin, new_admin);
    }

    pub fn get_fees(env: &Env) -> (i128, i128, Address) {
        let index_contract_addr = read_index_contract(&env);

        let (protocol_fee, manager_fee, protocol_address) = env.invoke_contract(
            &index_contract_addr,
            &Symbol::new(&env, "query_fees"),
            Vec::new(&env)
        );
        (protocol_fee, manager_fee, protocol_address)
    }

    pub fn calculate_fees(
        env: &Env,
        owner: Address,
        transfer_amount: i128,
        protocol_fee: i128,
        manager_fee: i128
    ) -> (i128, i128) {
        let last_transfer_info = get_last_transfer_info(&env, &owner);

        if last_transfer_info.last_balance == 0 {
            // No fee if there was no prior balance
            return (0, 0);
        }

        // Calculate weighted holding time
        let time_held = env.ledger().timestamp() - last_transfer_info.last_transfer_ts;

        // Prorated fee calculation
        let protocol_fee_amount =
            (transfer_amount * protocol_fee * (time_held as i128)) / (365 * 24 * 60 * 60 * 100); // Annualized
        let manager_fee_amount =
            (transfer_amount * manager_fee * (time_held as i128)) / (365 * 24 * 60 * 60 * 100); // Annualized

        (protocol_fee_amount, manager_fee_amount)
    }

    pub fn calculate_required_fees(env: Env, from: Address, amount: i128) -> (i128, i128, i128, i128) {
        // Calculate fee and deduct it
        let (protocol_fee, manager_fee, protocol_address) = Self::get_fees(&env);

        // Check if the sender or receiver is exempt from fees
        let index_contract_addr = read_index_contract(&env);
        let fee_exempt = env.invoke_contract(
            &index_contract_addr,
            &Symbol::new(&env, "query_fee_exempt"),
            Vec::new(&env, &from)
        );

        let (protocol_fee_amount, manager_fee_amount, total_fees, net_amount) = if fee_exempt {
            (0, 0, 0, amount) // No fees applied, transfer the full amount
        } else {
            let (protocol_fee_amount, manager_fee_amount) = Self::calculate_fees(
                &env,
                from.clone(),
                amount,
                protocol_fee,
                manager_fee
            );
            let total_fees = protocol_fee_amount + manager_fee_amount;

            let net_amount = amount - total_fees;

            if net_amount <= 0 {
                panic_with_error!(&env, ErrorCode::TransferAmountTooSmallAfterFees);
            }

            (protocol_fee_amount, manager_fee_amount, total_fees, net_amount)
        };
    }

    #[cfg(test)]
    pub fn get_allowance(env: Env, from: Address, spender: Address) -> Option<AllowanceValue> {
        let key = DataKey::Allowance(AllowanceDataKey { from, spender });
        let allowance = env.storage().temporary().get::<_, AllowanceValue>(&key);
        allowance
    }
}

#[contractimpl]
impl token::Interface for IndexToken {
    fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_allowance(&env, from, spender).amount
    }

    fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        check_nonnegative_amount(amount);
        from.require_auth();

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        write_allowance(&env, from.clone(), spender.clone(), amount, expiration_ledger);
        TokenUtils::new(&env).events().approve(from, spender, amount, expiration_ledger);
    }

    fn balance(env: Env, id: Address) -> i128 {
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
        read_balance(&env, id)
    }

    fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        check_nonnegative_amount(amount);
        from.require_auth();

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // Calculate fee and deduct it
        let (protocol_fee_amount, manager_fee_amount, total_fees, net_amount) =
            Self::calculate_required_fees(env, from, amount);

        let admin = read_administrator(&env);

        // spend_balance(&env, from.clone(), amount);
        // receive_balance(&env, to.clone(), net_amount);
        // receive_balance(&env, admin.clone(), fee);

        // Deduct from sender
        // let mut from_balance: i128 = env.storage().persistent().get(from.clone()).unwrap_or(0);
        let from_balance = read_balance(&env, from);
        if from_balance < amount {
            panic!("Insufficient balance");
        }
        from_balance -= amount;

        // Add to recipient
        // let mut to_balance: i128 = env.storage().persistent().get(to.clone()).unwrap_or(0);
        let tp_balance = read_balance(&env, to);
        to_balance += net_amount;

        // Add protocol fee to protocol address if applicable
        if protocol_fee_amount > 0 {
            let mut protocol_balance: i128 = e
                .storage()
                .persistent()
                .get(protocol_address.clone())
                .unwrap_or(0);
            protocol_balance += protocol_fee_amount;
            env.storage().persistent().set(protocol_address.clone(), protocol_balance);
        }

        // Add manager fee to Index contract if applicable
        if manager_fee_amount > 0 {
            let mut index_balance: i128 = e
                .storage()
                .persistent()
                .get(index_contract.clone())
                .unwrap_or(0);
            index_balance += manager_fee_amount;
            env.storage().persistent().set(index_contract.clone(), index_balance);

            // Notify the Index contract about the manager fee
            env.invoke_contract::<()>(&index_contract, &"handle_manager_fee".into(), (
                manager_fee_amount,
                from.clone(),
            ));
        }

        TokenUtils::new(&env).events().transfer(from.clone(), to.clone(), net_amount);

        // Log fee events if applicable
        if protocol_fee_amount > 0 {
            TokenUtils::new(&env).events().fee(from.clone(), admin, protocol_fee_amount);
        }
        if manager_fee_amount > 0 {
            TokenUtils::new(&env).events().fee(from.clone(), index_contract, manager_fee_amount);
        }

        // Update last transfer timestamps and amounts
        Self::update_last_transfer(&env, from.clone(), from_balance);
        Self::update_last_transfer(&env, to.clone(), to_balance);

        // Notify Index contract about the fee
        // TODO: do we need this?
        env.invoke_contract::<()>(&index_contract, &"handle_manager_fee".into(), (
            manager_fee_amount,
            from.clone(),
        ));
    }

    fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();

        check_nonnegative_amount(amount);

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        // Calculate fee and deduct it
        let (protocol_fee_amount, manager_fee_amount, total_fees, net_amount) =
            Self::calculate_required_fees(&env, from, amount);

        let admin = read_administrator(&env);

        spend_allowance(&env, from.clone(), spender, amount);
        spend_balance(&env, from.clone(), amount);
        receive_balance(&env, to.clone(), net_amount);
        receive_balance(&env, protocol_address.clone(), protocol_fee_amount);
        receive_balance(&env, index_contract.clone(), manager_fee_amount);

        TokenUtils::new(&env).events().transfer(from, to, net_amount);

        // Log fee events if applicable
        if protocol_fee_amount > 0 {
            TokenUtils::new(&env).events().fee(from.clone(), admin, protocol_fee_amount);
        }
        if manager_fee_amount > 0 {
            TokenUtils::new(&env).events().fee(from.clone(), index_contract, manager_fee_amount);
        }

        // Update last transfer timestamps and amounts
        let x = LastTransferInfo {
            last_transfer_ts: env.ledger().timestamp(),
            last_balance: from_balance,
        };
        save_last_transfer_info(&env, &from, &x);
        // save_last_transfer_info(&env, &to, LastTransferInfo {
        //     last_transfer_ts: env.ledger().timestamp(),
        //     last_balance: to_balance,
        // });

        // Notify Index contract about the fee
        // TODO: do we need this?
        env.invoke_contract::<()>(&read_index_contract(&env), &"handle_manager_fee".into(), (
            manager_fee_amount,
            from.clone(),
        ));
    }

    fn burn(env: Env, from: Address, amount: i128) {
        from.require_auth();

        check_nonnegative_amount(amount);

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        spend_balance(&env, from.clone(), amount);
        TokenUtils::new(&env).events().burn(from, amount);
    }

    fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();

        check_nonnegative_amount(amount);

        env.storage().instance().extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);

        spend_allowance(&env, from.clone(), spender, amount);
        spend_balance(&env, from.clone(), amount);
        TokenUtils::new(&env).events().burn(from, amount)
    }

    fn decimals(env: Env) -> u32 {
        read_decimal(&env)
    }

    fn name(env: Env) -> String {
        read_name(&env)
    }

    fn symbol(env: Env) -> String {
        read_symbol(&env)
    }
}
