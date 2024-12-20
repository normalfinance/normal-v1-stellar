#![no_std]
use soroban_sdk::{ contract, contractimpl, contracttype, symbol_short, Env, Symbol };

#[contracttype]
pub enum ExchangeStatus {
    Active,
    DepositPaused,
    WithdrawPaused,
    LendPaused,
    AmmPaused,
    LiqPaused,
    ScheduleFillPaused,
    Paused,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub admin: Address,
    // validations to ensure oracle prices are accurate and reliable
    pub oracle_guard_rails: OracleGuardRails,
    // set of elected keepers who can freeze/update oracles in an emergency
    pub emergency_oracles: Vec<>,
    // the current status of the protocol
    pub exchange_status: u8,
    // the total number of markets live on the protocol
    pub number_of_markets: u16,
    // the total number of index markets live on the protocol
    pub number_of_index_markets: u16,
    pub default_index_oracle: Address,
    pub max_index_assets: u16,
    pub protocol_index_fee: u16,
    pub protocol_index_fee_vault: Address,
    pub insurance_fund: Address,
    pub total_debt_ceiling: u64,
   
    // tracks the number of User delegate authorities
    pub number_of_authorities: u64,
    // tracks the number of User sub-accounts used to partition Vaults
    pub number_of_sub_accounts: u64,
    // the maximum number of sub-accounts the protocol is willing to support
    pub max_number_of_sub_accounts: u16,
    /// The maximum percent of the collateral that can be sent to the AMM as liquidity
    // pub max_amm_liquidity_utilization: u64,
    pub liquidation_margin_buffer_ratio: u32,
    pub liquidation_duration: u8,
    pub initial_pct_to_liquidate: u16,
    pub debt_auction_config: AuctionConfig,
    pub dca_order_padding: u16,
}

const STATE: Symbol = symbol_short!("STATE");

#[contract]
pub struct State;

#[contractimpl]
impl State {
    pub fn update_admin(e: Env, admin: Address) -> u32 {}

    pub fn update_status(e: Env, status: ExchangeStatus) -> ExchangeStatus {
        let mut state = Self::get_state(e.clone());
        state.status = status;
        e.storage().instance().set(&STATE, &state);
        state.status
    }

    pub fn get_state(e: Env) -> State {
        e.storage().instance().get(&STATE).unwrap_or(State {
            count: 0,
            last_incr: 0,
        })
    }
}

mod test;
