#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{token, Address, Env, IntoVal};

fn create_test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn setup_contract(env: &Env) -> Address {
    let contract_id = env.register_contract(None, FeeCollectorContract);
    let admin = Address::generate(env);

    let client = FeeCollectorContractClient::new(env, &contract_id);
    client.initialize(&admin);

    contract_id
}

#[test]
fn test_normal_trade() {
    let env = create_test_env();
    let contract_id = setup_contract(&env);
    let client = FeeCollectorContractClient::new(&env, &contract_id);

    let trade_amount = 10_000_000_000; // 1000 XLM
    let calculated_fee = 10_000_000;   // 1 XLM

    let result = client.collect_fee(&trade_amount, &calculated_fee);
    assert_eq!(result, 10_000_000); // Should return the calculated fee as is
}

#[test]
fn test_large_trade_cap() {
    let env = create_test_env();
    let contract_id = setup_contract(&env);
    let client = FeeCollectorContractClient::new(&env, &contract_id);

    let trade_amount = 1_000_000_000_000; // 100,000 XLM
    let calculated_fee = 2_000_000_000;   // 200 XLM (above max)

    let result = client.collect_fee(&trade_amount, &calculated_fee);
    assert_eq!(result, 1_000_000_000); // Should be capped at max_fee_per_trade (100 XLM)
}

#[test]
fn test_small_trade_floor() {
    let env = create_test_env();
    let contract_id = setup_contract(&env);
    let client = FeeCollectorContractClient::new(&env, &contract_id);

    let trade_amount = 1_000_000_000; // 100 XLM
    let calculated_fee = 10_000;       // 0.001 XLM (below min)

    let result = client.collect_fee(&trade_amount, &calculated_fee);
    assert_eq!(result, 100_000); // Should be floored at min_fee_per_trade (0.01 XLM)
}

#[test]
fn test_tiny_trade_reject() {
    let env = create_test_env();
    let contract_id = setup_contract(&env);
    let client = FeeCollectorContractClient::new(&env, &contract_id);

    let trade_amount = 50_000; // 0.005 XLM (below min_fee_per_trade)
    let calculated_fee = 5_000;

    let result = client.try_collect_fee(&trade_amount, &calculated_fee);
    assert_eq!(result, Err(Ok(FeeCollectorError::TradeTooSmall)));
}

#[test]
fn test_set_fee_config() {
    let env = create_test_env();
    let contract_id = setup_contract(&env);
    let client = FeeCollectorContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_config = FeeConfig {
        max_fee_per_trade: 2_000_000_000, // 200 XLM
        min_fee_per_trade: 200_000,       // 0.02 XLM
    };

    client.set_fee_config(&admin, &new_config);

    let retrieved_config = client.get_fee_config();
    assert_eq!(retrieved_config, new_config);
}

#[test]
fn test_claim_fees_normal() {
    let env = create_test_env();
    let contract_id = setup_contract(&env);
    let client = FeeCollectorContractClient::new(&env, &contract_id);

    let provider = Address::generate(&env);
    let amount: i128 = 1_000_000; // 0.1 XLM

    // Register a real token contract and mint `amount` to the fee_collector contract
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);
    token_admin_client.mint(&contract_id, &amount);

    // Seed pending fees in storage
    let key = StorageKey::ProviderPendingFees(provider.clone(), token_id.clone());
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key, &amount);
    });

    let claimed = client.claim_fees(&provider, &token_id);
    assert_eq!(claimed, amount);

    // Pending balance must be reset
    let remaining: i128 = env.as_contract(&contract_id, || {
        env.storage().persistent().get(&key).unwrap_or(0)
    });
    assert_eq!(remaining, 0);

    // Provider must have received the tokens
    let token_client = token::Client::new(&env, &token_id);
    assert_eq!(token_client.balance(&provider), amount);
}

#[test]
fn test_claim_fees_zero_balance() {
    let env = create_test_env();
    let contract_id = setup_contract(&env);
    let client = FeeCollectorContractClient::new(&env, &contract_id);

    let provider = Address::generate(&env);
    let token = Address::generate(&env);

    // No pending fees set, should return 0
    let claimed = client.claim_fees(&provider, &token);
    assert_eq!(claimed, 0);
}

#[test]
fn test_claim_fees_unauthorized() {
    let env = Env::default(); // No mock_all_auths — auth is enforced
    let contract_id = env.register_contract(None, FeeCollectorContract);
    let admin = Address::generate(&env);

    // initialize requires admin auth; mock only that call
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (admin.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let client = FeeCollectorContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let provider = Address::generate(&env);
    let token = Address::generate(&env);

    // Attempt to claim as `provider` without providing auth — must fail
    let result = client.try_claim_fees(&provider, &token);
    assert!(result.is_err());
}