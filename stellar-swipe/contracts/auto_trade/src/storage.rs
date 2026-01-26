#![allow(dead_code)]
use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub struct Signal {
    pub signal_id: u64,
    pub price: i128,
    pub expiry: u64,
    pub base_asset: u32,
}

#[contracttype]
pub enum DataKey {
    Trades(Address, u64),
    Signal(u64),
    Authorized(Address),
}

/// Get a signal by ID
pub fn get_signal(env: &Env, id: u64) -> Option<Signal> {
    env.storage().persistent().get(&DataKey::Signal(id))
}

/// Set a signal
pub fn set_signal(env: &Env, id: u64, signal: &Signal) {
    env.storage().persistent().set(&DataKey::Signal(id), signal);
}

/// Check if user is authorized
pub fn is_authorized(env: &Env, user: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::Authorized(user.clone()))
        .unwrap_or(false)
}

/// Authorize a user
pub fn authorize_user(env: &Env, user: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::Authorized(user.clone()), &true);
}
