#![allow(dead_code)]

use soroban_sdk::{contracttype, Address, Env, Map, Vec};

use crate::stake::{can_submit_signal, ContractError, StakeInfo, DEFAULT_MINIMUM_STAKE, UNSTAKE_LOCK_PERIOD};

/// Action enum for trading signals
#[contracttype]
#[derive(Clone)]
pub enum Action {
    Buy,
    Sell,
    Hold,
}

/// Structure to store a signal
#[contracttype]
#[derive(Clone)]
pub struct Signal {
    pub provider: Address,
    pub asset_pair: String,
    pub action: Action,
    pub price: i128,
    pub rationale: String,
    pub timestamp: u64,
    pub expiry: u64,
}

/// Contract-level error enum
#[derive(Debug, PartialEq)]
pub enum Error {
    NoStake,
    BelowMinimumStake,
    InvalidAssetPair,
    InvalidPrice,
    EmptyRationale,
    DuplicateSignal,
}

