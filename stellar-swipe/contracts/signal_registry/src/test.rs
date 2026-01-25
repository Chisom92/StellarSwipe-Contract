#![cfg(test)]

extern crate std;

use super::*;
use soroban_sdk::{testutils::Address as _, Env};

#[test]
fn test_initialize_and_admin() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, admin);
}

#[test]
fn test_admin_cannot_initialize_twice() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1);
    let result = client.try_initialize(&admin2);
    assert!(result.is_err());
}

#[test]
fn create_and_read_signal() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let provider = Address::generate(&env);
    let expiry = env.ledger().timestamp() + 60;

    let signal_id = client.create_signal(
        &provider,
        &String::from_str(&env, "XLM/USDC"),
        &SignalAction::Buy,
        &100_000,
        &String::from_str(&env, "Breakout confirmed"),
        &expiry,
    );

    let signal = client.get_signal(&signal_id).unwrap();
    assert_eq!(signal.id, signal_id);
    assert_eq!(signal.status, SignalStatus::Active);
}

#[test]
fn test_pause_blocks_signals() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let provider = Address::generate(&env);
    let expiry = env.ledger().timestamp() + 60;

    // Pause trading
    client.pause_trading(&admin);
    assert!(client.is_paused());

    // Try to create signal - should fail
    let result = client.try_create_signal(
        &provider,
        &String::from_str(&env, "XLM/USDC"),
        &SignalAction::Buy,
        &100_000,
        &String::from_str(&env, "Test"),
        &expiry,
    );

    assert!(result.is_err());

    // Unpause
    client.unpause_trading(&admin);
    assert!(!client.is_paused());

    // Now should work
    let signal_id = client.create_signal(
        &provider,
        &String::from_str(&env, "XLM/USDC"),
        &SignalAction::Buy,
        &100_000,
        &String::from_str(&env, "Test"),
        &expiry,
    );

    assert!(signal_id > 0);
}

#[test]
fn test_pause_auto_expires() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Pause trading
    client.pause_trading(&admin);
    assert!(client.is_paused());

    // Move time forward past 48 hours
    use soroban_sdk::testutils::Ledger;
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 48 * 60 * 60 + 1);

    // Should be auto-unpaused
    assert!(!client.is_paused());
}

#[test]
fn test_admin_config_updates() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Update min stake
    client.set_min_stake(&admin, &200_000_000);

    // Update trade fee
    client.set_trade_fee(&admin, &20);

    // Update risk defaults
    client.set_risk_defaults(&admin, &20, &25);

    let config = client.get_config();
    assert_eq!(config.min_stake, 200_000_000);
    assert_eq!(config.trade_fee_bps, 20);
    assert_eq!(config.default_stop_loss, 20);
    assert_eq!(config.default_position_limit, 25);
}

#[test]
fn test_unauthorized_admin_actions() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.initialize(&admin);

    // Attacker tries to update min stake
    let result = client.try_set_min_stake(&attacker, &500_000_000);
    assert!(result.is_err());

    // Attacker tries to pause
    let result = client.try_pause_trading(&attacker);
    assert!(result.is_err());

    // Attacker tries to transfer admin
    let new_admin = Address::generate(&env);
    let result = client.try_transfer_admin(&attacker, &new_admin);
    assert!(result.is_err());
}

#[test]
fn test_transfer_admin() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1);
    client.transfer_admin(&admin1, &admin2);

    let current_admin = client.get_admin();
    assert_eq!(current_admin, admin2);

    // Old admin should no longer work
    let result = client.try_pause_trading(&admin1);
    assert!(result.is_err());

    // New admin should work
    client.pause_trading(&admin2);
    assert!(client.is_paused());
}

#[test]
fn test_multisig_enable_and_use() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);

    client.initialize(&admin);

    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());
    signers.push_back(signer3.clone());

    // Enable multi-sig with 2-of-3 threshold
    client.enable_multisig(&admin, &signers, &2);

    assert!(client.is_multisig_enabled());
    assert_eq!(client.get_multisig_threshold(), 2);

    let returned_signers = client.get_multisig_signers();
    assert_eq!(returned_signers.len(), 3);

    // Any signer should be able to pause
    client.pause_trading(&signer1);
    assert!(client.is_paused());
}

#[test]
fn test_multisig_add_remove_signers() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer4 = Address::generate(&env);

    client.initialize(&admin);

    let mut signers = Vec::new(&env);
    signers.push_back(admin.clone());
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.enable_multisig(&admin, &signers, &2);

    assert_eq!(client.get_multisig_signers().len(), 3);

    // Add one more, then we can remove
    client.add_multisig_signer(&admin, &signer4);
    client.remove_multisig_signer(&admin, &signer1);
    assert_eq!(client.get_multisig_signers().len(), 3);
}

#[test]
fn test_invalid_parameter_updates() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Invalid min stake (negative)
    let result = client.try_set_min_stake(&admin, &-100);
    assert!(result.is_err());

    // Invalid trade fee (> 100 bps)
    let result = client.try_set_trade_fee(&admin, &150);
    assert!(result.is_err());

    // Invalid risk parameters (> 100%)
    let result = client.try_set_risk_defaults(&admin, &150, &20);
    assert!(result.is_err());
}

#[test]
fn provider_stats_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    #[allow(deprecated)]
    let contract_id = env.register_contract(None, SignalRegistry);
    let client = SignalRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let provider = Address::generate(&env);
    let expiry = env.ledger().timestamp() + 120;

    client.create_signal(
        &provider,
        &String::from_str(&env, "XLM/BTC"),
        &SignalAction::Sell,
        &200_000,
        &String::from_str(&env, "Resistance hit"),
        &expiry,
    );

    let stats = client.get_provider_stats(&provider).unwrap();
    assert_eq!(stats.total_copies, 0);
}
