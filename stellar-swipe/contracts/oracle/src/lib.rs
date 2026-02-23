#![no_std]

mod conversion;
mod errors;
mod storage;

use soroban_sdk::{contract, contractimpl, Address, Env};
use common::{Asset, AssetPair};
use errors::OracleError;

pub use conversion::{convert_to_base, ConversionPath};
pub use storage::{get_base_currency, set_base_currency, get_price, set_price};

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    /// Initialize oracle with base currency
    pub fn initialize(env: Env, admin: Address, base_currency: Asset) {
        storage::set_base_currency(&env, base_currency);
    }

    /// Set price for an asset pair
    pub fn set_price(env: Env, pair: AssetPair, price: i128) -> Result<(), OracleError> {
        if price <= 0 {
            return Err(OracleError::InvalidAsset);
        }
        storage::set_price(&env, &pair, price);
        storage::add_available_pair(&env, pair);
        Ok(())
    }

    /// Get price for an asset pair
    pub fn get_price(env: Env, pair: AssetPair) -> Result<i128, OracleError> {
        storage::get_price(&env, &pair)
    }

    /// Convert amount to base currency
    pub fn convert_to_base(env: Env, amount: i128, asset: Asset) -> Result<i128, OracleError> {
        // Check cache first
        let base = storage::get_base_currency(&env);
        if let Some(cached) = storage::get_cached_conversion(&env, &asset, &base) {
            return Ok(amount.checked_mul(cached.rate)
                .and_then(|v| v.checked_div(10_000_000))
                .ok_or(OracleError::ConversionOverflow)?);
        }

        // Perform conversion
        let result = conversion::convert_to_base(&env, amount, asset.clone())?;
        
        // Cache the rate
        if amount > 0 {
            let rate = result.checked_mul(10_000_000)
                .and_then(|v| v.checked_div(amount))
                .unwrap_or(0);
            if rate > 0 {
                storage::set_cached_conversion(&env, &asset, &base, rate);
            }
        }
        
        Ok(result)
    }

    /// Get base currency
    pub fn get_base_currency(env: Env) -> Asset {
        storage::get_base_currency(&env)
    }

    /// Set base currency (admin only)
    pub fn set_base_currency(env: Env, asset: Asset) {
        storage::set_base_currency(&env, asset);
    }

    /// Add available trading pair
    pub fn add_pair(env: Env, pair: AssetPair) {
        storage::add_available_pair(&env, pair);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, String};

    fn xlm(env: &Env) -> Asset {
        Asset {
            code: String::from_str(env, "XLM"),
            issuer: None,
        }
    }

    fn usdc(env: &Env) -> Asset {
        Asset {
            code: String::from_str(env, "USDC"),
            issuer: Some(Address::generate(env)),
        }
    }

    fn token(env: &Env, code: &str) -> Asset {
        Asset {
            code: String::from_str(env, code),
            issuer: Some(Address::generate(env)),
        }
    }

    #[test]
    fn test_initialize_and_get_base() {
        let env = Env::default();
        let contract_id = env.register_contract(None, OracleContract);
        let client = OracleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.initialize(&admin, &xlm(&env));
        let base = client.get_base_currency();
        assert_eq!(base.code, String::from_str(&env, "XLM"));
    }

    #[test]
    fn test_set_and_get_price() {
        let env = Env::default();
        let contract_id = env.register_contract(None, OracleContract);
        let client = OracleContractClient::new(&env, &contract_id);

        let pair = AssetPair {
            base: usdc(&env),
            quote: xlm(&env),
        };
        
        client.set_price(&pair, &10_000_000); // 1 USDC = 1 XLM
        let price = client.get_price(&pair).unwrap();
        assert_eq!(price, 10_000_000);
    }

    #[test]
    fn test_convert_to_base_direct() {
        let env = Env::default();
        let contract_id = env.register_contract(None, OracleContract);
        let client = OracleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        let xlm = xlm(&env);
        let usdc = usdc(&env);

        client.initialize(&admin, &xlm);

        // Set price: 1 USDC = 10 XLM
        let pair = AssetPair {
            base: usdc.clone(),
            quote: xlm.clone(),
        };
        client.set_price(&pair, &100_000_000); // 10 XLM per USDC

        // Convert 100 USDC to XLM
        let result = client.convert_to_base(&100_0000000, &usdc).unwrap();
        assert_eq!(result, 1000_0000000); // 1000 XLM
    }

    #[test]
    fn test_convert_same_asset() {
        let env = Env::default();
        let contract_id = env.register_contract(None, OracleContract);
        let client = OracleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        let xlm = xlm(&env);
        client.initialize(&admin, &xlm);

        let result = client.convert_to_base(&1000_0000000, &xlm).unwrap();
        assert_eq!(result, 1000_0000000);
    }

    #[test]
    fn test_base_currency_change() {
        let env = Env::default();
        let contract_id = env.register_contract(None, OracleContract);
        let client = OracleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        let xlm = xlm(&env);
        let usdc = usdc(&env);

        client.initialize(&admin, &xlm);
        assert_eq!(client.get_base_currency().code, String::from_str(&env, "XLM"));

        client.set_base_currency(&usdc);
        assert_eq!(client.get_base_currency().code, String::from_str(&env, "USDC"));
    }
}
