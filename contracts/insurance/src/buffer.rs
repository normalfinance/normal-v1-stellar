use soroban_sdk::{Address, Env};

pub trait BufferTrait {
    // ################################################################
    //                             ADMIN
    // ################################################################

    fn update_buffer_max_balance(env: Env, sender: Address, max_balance: i128);

    fn deposit_into_buffer(env: Env, sender: Address, amount: i128);

    // ################################################################
    //                             QUERIES
    // ################################################################

    // fn query_buffer(env: Env);

    // fn query_buffer_auctions(env: Env);

    // fn query_buffe_balance(env: Env);
}
