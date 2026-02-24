// contracts/oracle/src/staleness.rs
use soroban_sdk::{contracttype, Env, Address, symbol_short, Symbol};
use common::AssetPair;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StalenessLevel {
    Fresh,    // < 2m
    Aging,    // 2-5m
    Stale,    // 5-15m
    Critical, // > 15m
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PriceMetadata {
    pub last_update: u64,
    pub update_count_24h: u32,
    pub avg_update_interval: u64,
    pub staleness_level: StalenessLevel,
    pub is_paused: bool,
}