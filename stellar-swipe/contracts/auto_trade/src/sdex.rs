#![allow(dead_code)]

use soroban_sdk::{contracttype, symbol_short, Address, Env, Map};
use crate::errors::AutoTradeError;

/// ==========================
/// Types
/// ==========================
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Signal {
    pub signal_id: u64,
    pub price: i128,
    pub expiry: u64,
    pub base_asset: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Trade {
    pub signal_id: u64,
    pub user: Address,
    pub amount: i128,
    pub executed_price: i128,
    pub timestamp: u64,
    pub status: TradeStatus,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum TradeStatus {
    FullFill,
    PartialFill,
    NotFilled,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionResult {
    pub executed_amount: i128,
    pub executed_price: i128,
}

/// ==========================
/// Storage Keys
/// ==========================
pub type TradeMap = Map<(Address, u64), Trade>;
pub type BalanceMap = Map<Address, i128>;
pub type AuthMap = Map<Address, bool>;

/// ==========================
/// Authorization & Balance
/// ==========================
pub fn has_sufficient_balance(
    balance_map: &BalanceMap,
    user: &Address,
    amount: i128,
) -> bool {
    balance_map.get(user.clone()).unwrap_or(0) >= amount
}

pub fn is_authorized(auth_map: &AuthMap, user: &Address) -> bool {
    auth_map.get(user.clone()).unwrap_or(false)
}

/// ==========================
/// Core Trade Execution
/// ==========================
pub fn execute_trade(
    env: &Env,
    user: &Address,
    signal: &Signal,
    order_type: OrderType,
    amount: i128,
    balance_map: &mut BalanceMap,
    auth_map: &AuthMap,
    trade_map: &mut TradeMap,
) -> Result<ExecutionResult, AutoTradeError> {
    let now = env.ledger().timestamp();

    // --------------------------
    // 1. Signal expiry check
    // --------------------------
    if now >= signal.expiry {
        return Err(AutoTradeError::SignalExpired);
    }

    // --------------------------
    // 2. Authorization check
    // --------------------------
    if !is_authorized(auth_map, user) {
        return Err(AutoTradeError::Unauthorized);
    }

    // --------------------------
    // 3. Balance check
    // --------------------------
    if !has_sufficient_balance(balance_map, user, amount) {
        return Err(AutoTradeError::InsufficientBalance);
    }

    // --------------------------
    // 4. Mock SDEX liquidity
    // --------------------------
    let key_liquidity = (symbol_short!("liquidity"), signal.signal_id);
    let available_liquidity: i128 = env
        .storage()
        .temporary()
        .get(&key_liquidity)
        .unwrap_or(amount); // mock: full requested amount available

    let executed_amount = match order_type {
        OrderType::Market => core::cmp::min(amount, available_liquidity),
        OrderType::Limit => {
            let key_price = (symbol_short!("price"), signal.signal_id);
            let market_price: i128 = env
                .storage()
                .temporary()
                .get(&key_price)
                .unwrap_or(signal.price);
            if market_price > signal.price {
                0
            } else {
                amount
            }
        }
    };

    let status = if executed_amount == 0 {
        TradeStatus::NotFilled
    } else if executed_amount < amount {
        TradeStatus::PartialFill
    } else {
        TradeStatus::FullFill
    };

    // --------------------------
    // 5. Deduct user balance (mock)
    // --------------------------
    let user_balance = balance_map.get(user.clone()).unwrap_or(0);
    balance_map.set(user.clone(), user_balance - executed_amount);

    // --------------------------
    // 6. Store executed trade
    // --------------------------
    trade_map.set(
        (user.clone(), signal.signal_id),
        Trade {
            signal_id: signal.signal_id,
            user: user.clone(),
            amount: executed_amount,
            executed_price: signal.price,
            timestamp: now,
            status: status.clone(),
        },
    );

    // --------------------------
    // 7. Return execution result
    // --------------------------
    Ok(ExecutionResult {
        executed_amount,
        executed_price: signal.price,
    })
}

/// ==========================
/// Unit Tests
/// ==========================
#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Ledger, Address as TestAddress};
    use soroban_sdk::Env;

    fn setup_env() -> Env {
        let env = Env::default();
        env.ledger().set_timestamp(1_000);
        env
    }

    fn setup_signal(env: &Env, id: u64) -> Signal {
        Signal {
            signal_id: id,
            price: 100,
            expiry: env.ledger().timestamp() + 1_000,
            base_asset: 1,
        }
    }

    fn test_user(env: &Env, n: u8) -> Address {
        <Address as TestAddress>::generate(env)
    }

    #[test]
    fn market_order_full_fill() {
        let env = setup_env();
        let user = test_user(&env, 1);
        let mut balance_map: BalanceMap = Map::new(&env);
        let auth_map: AuthMap = Map::new(&env);
        let mut trade_map: TradeMap = Map::new(&env);

        balance_map.set(user.clone(), 500);
        auth_map.set(user.clone(), true);

        let signal = setup_signal(&env, 1);

        let res = execute_trade(
            &env,
            &user,
            &signal,
            OrderType::Market,
            400,
            &mut balance_map,
            &auth_map,
            &mut trade_map,
        )
        .unwrap();

        assert_eq!(res.executed_amount, 400);
        assert_eq!(res.executed_price, 100);

        let trade = trade_map.get((user.clone(), 1)).unwrap();
        assert_eq!(trade.status, TradeStatus::FullFill);
    }

    #[test]
    fn market_order_partial_fill() {
        let env = setup_env();
        let user = test_user(&env, 2);
        let mut balance_map: BalanceMap = Map::new(&env);
        let auth_map: AuthMap = Map::new(&env);
        let mut trade_map: TradeMap = Map::new(&env);

        balance_map.set(user.clone(), 100);
        auth_map.set(user.clone(), true);

        let signal = setup_signal(&env, 2);

        let res = execute_trade(
            &env,
            &user,
            &signal,
            OrderType::Market,
            300,
            &mut balance_map,
            &auth_map,
            &mut trade_map,
        )
        .unwrap();

        assert_eq!(res.executed_amount, 100);
        assert_eq!(res.executed_price, 100);

        let trade = trade_map.get((user.clone(), 2)).unwrap();
        assert_eq!(trade.status, TradeStatus::PartialFill);
    }

    #[test]
    fn limit_order_not_filled() {
        let env = setup_env();
        let user = test_user(&env, 3);
        let mut balance_map: BalanceMap = Map::new(&env);
        let auth_map: AuthMap = Map::new(&env);
        let mut trade_map: TradeMap = Map::new(&env);

        balance_map.set(user.clone(), 200);
        auth_map.set(user.clone(), true);

        let signal = setup_signal(&env, 3);
        // Set mock market price higher than signal price
        env.storage()
            .temporary()
            .set(&(symbol_short!("price"), 3u64), &150i128);

        let res = execute_trade(
            &env,
            &user,
            &signal,
            OrderType::Limit,
            200,
            &mut balance_map,
            &auth_map,
            &mut trade_map,
        )
        .unwrap();

        assert_eq!(res.executed_amount, 0);
        assert_eq!(res.executed_price, 100);

        let trade = trade_map.get((user.clone(), 3)).unwrap();
        assert_eq!(trade.status, TradeStatus::NotFilled);
    }

    #[test]
    fn expired_signal_rejected() {
        let env = setup_env();
        let user = test_user(&env, 4);
        let mut balance_map: BalanceMap = Map::new(&env);
        let auth_map: AuthMap = Map::new(&env);
        let mut trade_map: TradeMap = Map::new(&env);

        balance_map.set(user.clone(), 100);
        auth_map.set(user.clone(), true);

        let signal = Signal {
            signal_id: 4,
            price: 100,
            expiry: env.ledger().timestamp() - 1, // expired
            base_asset: 1,
        };

        let err = execute_trade(
            &env,
            &user,
            &signal,
            OrderType::Market,
            100,
            &mut balance_map,
            &auth_map,
            &mut trade_map,
        )
        .unwrap_err();

        assert_eq!(err, AutoTradeError::SignalExpired);
    }
}
