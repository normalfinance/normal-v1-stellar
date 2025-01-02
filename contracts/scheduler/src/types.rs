use soroban_sdk::{ contracttype, Address, BytesN, String, Symbol, Val, Vec };

#[contracttype]
pub struct Asset {
    pub contract_id: Option<Address>, // `None` for native XLM, `Some(Address)` for custom tokens
    pub symbol: Symbol, // Symbol for display or tracking purposes
}

#[derive(Clone)]
#[contracttype]
pub enum OrderDirection {
    Buy,
    Sell,
}

#[derive(Clone)]
#[contracttype]
pub struct ScheduleData {
    pub base_asset_amount_per_interval: u64,
    pub direction: OrderDirection,
    pub active: bool,
    pub interval_seconds: u64,
    pub total_orders: u16,
    pub min_price: Option<u16>,
    pub max_price: Option<u16>,
    pub executed_orders: u16,
    pub total_executed: u64,
    pub last_updated_ts: u64,
    pub last_order_ts: u64,
}

/// The schedule (asset) object
#[derive(Clone)]
#[contracttype]
pub struct AssetSchedule {
    pub id: u32,
    // pub config: ProposalConfig,
    pub amm_id: Address,
    pub data: ScheduleData,
}

/// The schedule (index) object
#[derive(Clone)]
#[contracttype]
pub struct IndexSchedule {
    pub id: u32,
    // pub config: ProposalConfig,
    pub index_id: u32,
    pub data: ScheduleData,
}
