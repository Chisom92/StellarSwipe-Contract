use soroban_sdk::{contracttype, Address, Env};

use crate::errors::AutoTradeError;
use crate::storage::Signal;

/// Result returned by SDEX adapter
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionResult {
    pub executed_amount: i128,
    pub executed_price: i128,
}

/// Simulated on-chain balance check
/// In production: asset contract / trustline verification
pub fn has_sufficient_balance(
    env: &Env,
    user: &Address,
    _asset: &u32,
    amount: i128,
) -> bool {
    let key = (user.clone(), "balance");
    let balance: i128 = env
        .storage()
        .temporary()
        .get(&key)
        .unwrap_or(0);

    balance >= amount
}

/// Mock MARKET order execution
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

    // Simulated orderbook depth
    let available_liquidity: i128 = env
        .storage()
        .temporary()
        .get(&("liquidity", signal.signal_id))
        .unwrap_or(amount);

    if available_liquidity <= 0 {
        return Err(AutoTradeError::InsufficientLiquidity);
    }

    let executed_amount = core::cmp::min(amount, available_liquidity);

    Ok(ExecutionResult {
        executed_amount,
        executed_price: signal.price, // aggressive crossing price
    })
}

/// Mock LIMIT order execution
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

    let market_price: i128 = env
        .storage()
        .temporary()
        .get(&("market_price", signal.signal_id))
        .unwrap_or(signal.price);

    // Limit condition not met
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
    use crate::errors::AutoTradeError;
    use crate::storage::Signal;

    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn setup_env() -> (Env, Address) {
        let env = Env::default();
        let contract_id = Address::generate(&env);

        // Initialize ledger timestamp safely
        env.ledger().set_timestamp(1_000);

        (env, contract_id)
    }

    fn setup_signal(env: &Env, id: u64) -> Signal {
        let signal = Signal {
            signal_id: id,
            price: 100,
            expiry: env.ledger().timestamp() + 1_000,
            base_asset: 1,
        };

        env.storage()
            .persistent()
            .set(&("signal", id), &signal);

        signal
    }

    #[test]
    fn market_order_full_fill() {
        let (env, contract_id) = setup_env();
        let user = Address::generate(&env);

        env.as_contract(&contract_id, || {
            env.storage()
                .temporary()
                .set(&(user.clone(), "balance"), &1_000);

            env.storage()
                .temporary()
                .set(&("liquidity", 1u64), &500);

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
            env.storage()
                .temporary()
                .set(&(user.clone(), "balance"), &1_000);

            env.storage()
                .temporary()
                .set(&("liquidity", 2u64), &100);

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
            env.storage()
                .temporary()
                .set(&("market_price", 3u64), &150);

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
