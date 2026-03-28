#![no_std]

use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, Address,
    Env, MuxedAddress,
};

/// Protocol fee rate: 0.1% = 10 basis points.
const FEE_RATE_BPS: i128 = 10;
const BPS_DENOM: i128 = 10_000;
/// When `trade_amount * FEE_RATE_BPS < BPS_DENOM`, charge at least one stroop.
const MIN_FEE_STROOPS: i128 = 1;

/// Default share of each fee credited to the signal provider (basis points). 5000 = 50%.
const DEFAULT_PROVIDER_FEE_SHARE_BPS: u32 = 5_000;

#[contract]
pub struct FeeCollector;

/// Composite storage key for per-provider, per-token pending balances.
#[contracttype]
#[derive(Clone)]
pub struct ProviderPendingKey {
    pub provider: Address,
    pub token: Address,
}

#[contracttype]
#[derive(Clone)]
pub enum StorageKey {
    AccumulatedFees(Address),
    FeeExempt(Address),
    Admin,
    Treasury,
    ProviderFeeShareBps,
    /// Running total of `provider_share` not yet claimed (per provider + token).
    ProviderPendingFees(ProviderPendingKey),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    NonPositiveTrade = 3,
    Overflow = 4,
    /// `provider_fee_share_bps` must be in `0..=10_000`.
    InvalidProviderFeeShareBps = 5,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollected {
    #[topic]
    pub payer: Address,
    #[topic]
    pub token: Address,
    pub amount: i128,
    pub trade_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeDistributed {
    #[topic]
    pub provider: Address,
    #[topic]
    pub token: Address,
    pub provider_share: i128,
    pub treasury_share: i128,
}

#[contractimpl]
impl FeeCollector {
    pub fn initialize(env: Env, admin: Address, treasury: Address) {
        if env.storage().instance().has(&StorageKey::Admin) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        env.storage().instance().set(&StorageKey::Admin, &admin);
        env.storage().instance().set(&StorageKey::Treasury, &treasury);
        env.storage()
            .instance()
            .set(&StorageKey::ProviderFeeShareBps, &DEFAULT_PROVIDER_FEE_SHARE_BPS);
    }

    pub fn set_fee_exempt(env: Env, account: Address, exempt: bool) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        admin.require_auth();
        if exempt {
            env.storage()
                .persistent()
                .set(&StorageKey::FeeExempt(account), &true);
        } else {
            env.storage().persistent().remove(&StorageKey::FeeExempt(account));
        }
    }

    /// Sets the provider’s share of each fee in basis points (0–10_000 = 0–100%).
    pub fn set_provider_fee_share_bps(env: Env, provider_fee_share_bps: u32) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        admin.require_auth();
        if provider_fee_share_bps > 10_000 {
            panic_with_error!(&env, Error::InvalidProviderFeeShareBps);
        }
        env.storage()
            .instance()
            .set(&StorageKey::ProviderFeeShareBps, &provider_fee_share_bps);
    }

    pub fn get_provider_fee_share_bps(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&StorageKey::ProviderFeeShareBps)
            .unwrap_or(DEFAULT_PROVIDER_FEE_SHARE_BPS)
    }

    /// Pulls fee from `payer`, sends treasury portion to treasury, accrues provider portion for `claim_provider_fees`.
    /// Returns total fee charged (0 if `payer` is fee-exempt).
    pub fn collect_fee(
        env: Env,
        payer: Address,
        trade_amount: i128,
        token: Address,
        provider: Address,
    ) -> i128 {
        payer.require_auth();

        if trade_amount <= 0 {
            panic_with_error!(&env, Error::NonPositiveTrade);
        }

        let fee_exempt: bool = env
            .storage()
            .persistent()
            .get(&StorageKey::FeeExempt(payer.clone()))
            .unwrap_or(false);

        let fee = if fee_exempt {
            0
        } else {
            let product = trade_amount
                .checked_mul(FEE_RATE_BPS)
                .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));
            if product < BPS_DENOM {
                MIN_FEE_STROOPS
            } else {
                product / BPS_DENOM
            }
        };

        if fee > 0 {
            let share_bps = Self::get_provider_fee_share_bps(env.clone()) as i128;
            let provider_share = fee
                .checked_mul(share_bps)
                .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow))
                / BPS_DENOM;
            let treasury_share = fee
                .checked_sub(provider_share)
                .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));

            let token_client = TokenClient::new(&env, &token);
            let this = env.current_contract_address();
            token_client.transfer(&payer, &MuxedAddress::from(&this), &fee);

            let treasury: Address = env
                .storage()
                .instance()
                .get(&StorageKey::Treasury)
                .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));

            if treasury_share > 0 {
                token_client.transfer(&this, &MuxedAddress::from(&treasury), &treasury_share);
            }

            if provider_share > 0 {
                let pending_key = ProviderPendingKey {
                    provider: provider.clone(),
                    token: token.clone(),
                };
                let prev: i128 = env
                    .storage()
                    .persistent()
                    .get(&StorageKey::ProviderPendingFees(pending_key.clone()))
                    .unwrap_or(0);
                let next = prev
                    .checked_add(provider_share)
                    .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));
                env.storage()
                    .persistent()
                    .set(&StorageKey::ProviderPendingFees(pending_key), &next);
            }

            let prev_total: i128 = env
                .storage()
                .persistent()
                .get(&StorageKey::AccumulatedFees(token.clone()))
                .unwrap_or(0);
            let next_total = prev_total
                .checked_add(fee)
                .unwrap_or_else(|| panic_with_error!(&env, Error::Overflow));
            env.storage()
                .persistent()
                .set(&StorageKey::AccumulatedFees(token.clone()), &next_total);

            FeeDistributed {
                provider: provider.clone(),
                token: token.clone(),
                provider_share,
                treasury_share,
            }
            .publish(&env);
        }

        FeeCollected {
            payer: payer.clone(),
            token: token.clone(),
            amount: fee,
            trade_amount,
        }
        .publish(&env);

        fee
    }

    /// Transfers this contract’s pending provider share for `token` to `provider`.
    pub fn claim_provider_fees(env: Env, provider: Address, token: Address) -> i128 {
        provider.require_auth();
        let pending_key = ProviderPendingKey {
            provider: provider.clone(),
            token: token.clone(),
        };
        let pending: i128 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProviderPendingFees(pending_key.clone()))
            .unwrap_or(0);
        if pending <= 0 {
            return 0;
        }

        let token_client = TokenClient::new(&env, &token);
        let this = env.current_contract_address();
        token_client.transfer(&this, &MuxedAddress::from(&provider), &pending);
        env.storage()
            .persistent()
            .remove(&StorageKey::ProviderPendingFees(pending_key));

        pending
    }

    pub fn get_provider_pending_fees(env: Env, provider: Address, token: Address) -> i128 {
        let pending_key = ProviderPendingKey { provider, token };
        env.storage()
            .persistent()
            .get(&StorageKey::ProviderPendingFees(pending_key))
            .unwrap_or(0)
    }

    pub fn get_accumulated_fees(env: Env, token: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&StorageKey::AccumulatedFees(token))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::token::{Client as TokenClient, StellarAssetClient};

    /// 7 decimals (e.g. XLM-style) for “100 XLM” style tests.
    const STROOPS_PER_UNIT: i128 = 10_000_000;

    fn setup_fee_collector(
        env: &Env,
    ) -> (
        Address,
        Address,
        Address,
        Address,
        FeeCollectorClient<'_>,
    ) {
        env.mock_all_auths();
        let admin = Address::generate(env);
        let treasury = Address::generate(env);
        let contract_id = env.register(FeeCollector, ());
        let client = FeeCollectorClient::new(env, &contract_id);
        client.initialize(&admin, &treasury);
        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let token = sac.address();
        let payer = Address::generate(env);
        StellarAssetClient::new(env, &token).mint(&payer, &100_000_000_000i128);
        (admin, treasury, token, payer, client)
    }

    #[test]
    fn normal_trade_charges_exactly_10_bps() {
        let env = Env::default();
        let (_admin, treasury, token, payer, client) = setup_fee_collector(&env);
        let provider = Address::generate(&env);
        let trade_amount = 1_000_000i128;
        let fee = client.collect_fee(&payer, &trade_amount, &token, &provider);
        assert_eq!(fee, 1_000);
        let contract_id = client.address.clone();
        assert_eq!(TokenClient::new(&env, &token).balance(&contract_id), 500);
        assert_eq!(TokenClient::new(&env, &token).balance(&treasury), 500);
        assert_eq!(client.get_provider_pending_fees(&provider, &token), 500);
        assert_eq!(client.get_accumulated_fees(&token), 1_000);
    }

    #[test]
    fn dust_trade_charges_minimum_one_stroop() {
        let env = Env::default();
        let (_admin, treasury, token, payer, client) = setup_fee_collector(&env);
        let provider = Address::generate(&env);
        let trade_amount = 999i128;
        assert!(trade_amount * FEE_RATE_BPS < BPS_DENOM);
        let fee = client.collect_fee(&payer, &trade_amount, &token, &provider);
        assert_eq!(fee, 1);
        let contract_id = client.address.clone();
        assert_eq!(TokenClient::new(&env, &token).balance(&contract_id), 0);
        assert_eq!(TokenClient::new(&env, &token).balance(&treasury), 1);
        assert_eq!(client.get_provider_pending_fees(&provider, &token), 0);
        assert_eq!(client.get_accumulated_fees(&token), 1);
    }

    #[test]
    fn fee_exempt_address_pays_no_fee() {
        let env = Env::default();
        let (_admin, treasury, token, payer, client) = setup_fee_collector(&env);
        let provider = Address::generate(&env);
        client.set_fee_exempt(&payer, &true);
        let trade_amount = 1_000_000i128;
        let fee = client.collect_fee(&payer, &trade_amount, &token, &provider);
        assert_eq!(fee, 0);
        let contract_id = client.address.clone();
        assert_eq!(TokenClient::new(&env, &token).balance(&contract_id), 0);
        assert_eq!(TokenClient::new(&env, &token).balance(&treasury), 0);
        assert_eq!(client.get_provider_pending_fees(&provider, &token), 0);
        assert_eq!(client.get_accumulated_fees(&token), 0);
    }

    #[test]
    fn hundred_xlm_trade_50_50_split() {
        let env = Env::default();
        let (_admin, treasury, token, payer, client) = setup_fee_collector(&env);
        let provider = Address::generate(&env);
        assert_eq!(client.get_provider_fee_share_bps(), 5_000);

        let trade_amount = 100 * STROOPS_PER_UNIT;
        let fee = client.collect_fee(&payer, &trade_amount, &token, &provider);
        // 0.1% of 100 XLM = 0.1 XLM = 1_000_000 stroops; 50/50 → 0.05 XLM = 500_000 stroops each.
        let expected_fee = 1_000_000i128;
        let expected_half = 500_000i128;
        assert_eq!(fee, expected_fee);
        assert_eq!(
            TokenClient::new(&env, &token).balance(&treasury),
            expected_half,
            "treasury should receive 0.05 XLM"
        );
        assert_eq!(
            client.get_provider_pending_fees(&provider, &token),
            expected_half,
            "provider pending should be 0.05 XLM before claim"
        );

        let claimed = client.claim_provider_fees(&provider, &token);
        assert_eq!(claimed, expected_half);
        assert_eq!(
            TokenClient::new(&env, &token).balance(&provider),
            expected_half,
            "provider should receive 0.05 XLM on claim"
        );
        assert_eq!(client.get_provider_pending_fees(&provider, &token), 0);
        let contract_id = client.address.clone();
        assert_eq!(TokenClient::new(&env, &token).balance(&contract_id), 0);
    }

    #[test]
    fn set_provider_fee_share_bps_rejects_over_100_percent() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        let contract_id = env.register(FeeCollector, ());
        let client = FeeCollectorClient::new(&env, &contract_id);
        client.initialize(&admin, &treasury);
        let res = client.try_set_provider_fee_share_bps(&10_001u32);
        assert!(res.is_err());
    }
}
