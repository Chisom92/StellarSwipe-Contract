#![allow(dead_code)]
//! Multi-asset SDEX support for Stellar trading.
//!
//! Handles native (XLM), issued assets, and supports manage_buy_offer/manage_sell_offer
//! for any valid Stellar asset pair. All Stellar assets use 7 decimal precision.

use soroban_sdk::{Address, Env};

use crate::errors::AutoTradeError;
use crate::sdex::ExecutionResult;
use crate::storage::Signal;

/// Stellar asset decimal precision (all assets)
pub const STELLAR_DECIMALS: u32 = 7;

/// Scale factor for 7 decimals (10^7)
pub const STELLAR_SCALE: i128 = 10_000_000;

/// Execute market order for any asset pair.
/// Delegates to SDEX; handles 7-decimal precision consistently.
pub fn execute_multi_asset_market_order(
    env: &Env,
    user: &Address,
    signal: &Signal,
    amount: i128,
) -> Result<ExecutionResult, AutoTradeError> {
    if amount <= 0 {
        return Err(AutoTradeError::InvalidAmount);
    }
    crate::sdex::execute_market_order(env, user, signal, amount)
}

/// Execute limit order for any asset pair.
pub fn execute_multi_asset_limit_order(
    env: &Env,
    user: &Address,
    signal: &Signal,
    amount: i128,
) -> Result<ExecutionResult, AutoTradeError> {
    if amount <= 0 {
        return Err(AutoTradeError::InvalidAmount);
    }
    crate::sdex::execute_limit_order(env, user, signal, amount)
}
