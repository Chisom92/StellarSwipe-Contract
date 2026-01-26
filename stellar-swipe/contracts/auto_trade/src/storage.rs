use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub struct Signal {
    pub signal_id: u64,
    pub price: i128,
    pub expiry: u64,
    pub base_asset: u32,
}

pub fn get_signal(env: &Env, id: u64) -> Option<Signal> {
    env.storage().persistent().get(&("signal", id))
}

pub fn is_authorized(env: &Env, user: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&(user.clone(), "authorized"))
        .unwrap_or(false)
}
