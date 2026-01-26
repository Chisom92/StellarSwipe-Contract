use soroban_sdk::{contracttype, Address, Env, symbol_short};

use crate::errors::AutoTradeError;
use crate::storage::Signal;

/// ==========================
/// Types
/// ==========================
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionResult {
    pub executed_amount: i128,
    pub executed_price: i128,
}

/// ==========================
/// Balance Check
/// ==========================
pub fn has_sufficient_balance(
    env: &Env,
    user: &Address,
    _asset: &u32,
    amount: i128,
) -> bool {
    let key = (user.clone(), symbol_short!("balance"));
    let balance: i128 = env.storage().temporary().get(&key).unwrap_or(0);
    balance >= amount
}

/// ==========================
/// Market Order
/// ==========================
pub fn execute_market_order(
    env: &Env,
    _user: &Address,
    signal: &Signal,
    amount: i128,
) -> Result<ExecutionResult, AutoTradeError> {
    let now = env.ledger().timestamp();

    if now >= signal.expiry {
        return Err(AutoTradeError::SignalExpired);
    }

    let key = (symbol_short!("liquidity"), signal.signal_id);
    let available_liquidity: i128 = env
        .storage()
        .temporary()
        .get(&key)
        .unwrap_or(amount);

    if available_liquidity <= 0 {
        return Err(AutoTradeError::InsufficientLiquidity);
    }

    let executed_amount = core::cmp::min(amount, available_liquidity);

    Ok(ExecutionResult {
        executed_amount,
        executed_price: signal.price,
    })
}

/// ==========================
/// Limit Order
/// ==========================
pub fn execute_limit_order(
    env: &Env,
    _user: &Address,
    signal: &Signal,
    amount: i128,
) -> Result<ExecutionResult, AutoTradeError> {
    let now = env.ledger().timestamp();

    if now >= signal.expiry {
        return Err(AutoTradeError::SignalExpired);
    }

    let key = (symbol_short!("price"), signal.signal_id);
    let market_price: i128 = env
        .storage()
        .temporary()
        .get(&key)
        .unwrap_or(signal.price);

    if market_price > signal.price {
        return Ok(ExecutionResult {
            executed_amount: 0,
            executed_price: 0,
        });
    }

    Ok(ExecutionResult {
        executed_amount: amount,
        executed_price: signal.price,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, Address, symbol_short};

    // Create Env + contract context
    fn setup_env() -> (Env, Address) {
        let env = Env::default();
        let contract_id = Address::generate(&env);
        // Only set timestamp, no protocol version
        env.ledger().set_timestamp(1_000);
        (env, contract_id)
    }

    fn setup_signal(env: &Env, id: u64) -> Signal {
        Signal {
            signal_id: id,
            price: 100,
            expiry: env.ledger().timestamp() + 1_000,
            base_asset: 1,
        }
    }

    #[test]
    fn market_order_full_fill() {
        let (env, contract_id) = setup_env();
        let user = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Set liquidity for the signal
            env.storage()
                .temporary()
                .set(&(symbol_short!("liquidity"), 1u64), &500i128);

            let signal = setup_signal(&env, 1);
            let res = execute_market_order(&env, &user, &signal, 400).unwrap();
            assert_eq!(res.executed_amount, 400);
        });
    }

    #[test]
    fn market_order_partial_fill() {
        let (env, contract_id) = setup_env();
        let user = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Set partial liquidity
            env.storage()
                .temporary()
                .set(&(symbol_short!("liquidity"), 2u64), &100i128);

            let signal = setup_signal(&env, 2);
            let res = execute_market_order(&env, &user, &signal, 300).unwrap();
            assert_eq!(res.executed_amount, 100);
        });
    }

    #[test]
    fn limit_order_not_filled() {
        let (env, contract_id) = setup_env();
        let user = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Set market price above limit price
            env.storage()
                .temporary()
                .set(&(symbol_short!("price"), 3u64), &150i128);

            let signal = setup_signal(&env, 3);
            let res = execute_limit_order(&env, &user, &signal, 200).unwrap();
            assert_eq!(res.executed_amount, 0);
        });
    }

    #[test]
    fn expired_signal_rejected() {
        let (env, contract_id) = setup_env();
        let user = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Expired signal
            let signal = Signal {
                signal_id: 4,
                price: 100,
                expiry: env.ledger().timestamp() - 1,
                base_asset: 1,
            };

            let err = execute_market_order(&env, &user, &signal, 100).unwrap_err();
            assert_eq!(err, AutoTradeError::SignalExpired);
        });
    }
}
