use soroban_sdk::{ Address, Env };

pub trait BufferTrait {
    fn initialize(env: Env);

    fn update_max_balance(env: Env);

    fn deposit(env: Env, amount: i128);

    fn buy_back_and_burn(env: Env, sender: Address, amount: i128);

    fn mint_and_sell(env: Env, sender: Address, amount: i128);

    // QUERIES
}
