#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn setup(env: &Env) -> (LiquidityPoolContractClient, Address, Address, Address) {
    let admin = Address::generate(env);
    let token_a = env.register_stellar_asset_contract(admin.clone());
    let token_b = env.register_stellar_asset_contract(admin.clone());
    let contract_id = env.register_contract(None, LiquidityPoolContract);
    let client = LiquidityPoolContractClient::new(env, &contract_id);
    client.initialize(&admin, &token_a, &token_b);
    (client, admin, token_a, token_b)
}

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, _token_a, _token_b) = setup(&env);
    let stats = client.get_stats();
    assert_eq!(stats.reserve_a, 0);
    assert_eq!(stats.reserve_b, 0);
    assert_eq!(stats.total_shares, 0);
}

#[test]
fn test_add_liquidity() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_a, token_b) = setup(&env);
    let provider = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token_a).mint(&provider, &10_000);
    token::StellarAssetClient::new(&env, &token_b).mint(&provider, &10_000);
    let shares = client.add_liquidity(&provider, &10_000, &10_000);
    assert!(shares > 0);
}

#[test]
fn test_swap() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_a, token_b) = setup(&env);
    let provider = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token_a).mint(&provider, &10_000);
    token::StellarAssetClient::new(&env, &token_b).mint(&provider, &10_000);
    client.add_liquidity(&provider, &10_000, &10_000);
    let trader = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token_a).mint(&trader, &10_000);
    let out = client.swap(&trader, &10_000, &1, &true);
    assert!(out > 0);
}

#[test]
fn test_swap_slippage_protection() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_a, token_b) = setup(&env);
    let provider = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token_a).mint(&provider, &10_000);
    token::StellarAssetClient::new(&env, &token_b).mint(&provider, &10_000);
    client.add_liquidity(&provider, &10_000, &10_000);
    let trader = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token_a).mint(&trader, &10_000);
    let result = client.try_swap(&trader, &10_000, &999_999, &true);
    assert!(result.is_err());
}

#[test]
fn test_initialize_rejects_identical_tokens() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_a = env.register_stellar_asset_contract(admin.clone());
    let contract_id = env.register_contract(None, LiquidityPoolContract);
    let client = LiquidityPoolContractClient::new(&env, &contract_id);
    let err = client
        .try_initialize(&admin, &token_a, &token_a)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::IdenticalTokens);
}

#[test]
fn test_admin_rotation_and_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_a, token_b) = setup(&env);
    let new_admin = Address::generate(&env);

    // Rotation flow
    client.propose_admin(&new_admin);
    client.accept_admin(&new_admin);

    // New admin pauses pool
    client.set_paused(&true);
    assert!(client.is_paused());

    // Swapping while paused fails with Error::Paused
    let trader = Address::generate(&env);
    let err = client
        .try_swap(&trader, &100i128, &90i128, &true)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::Paused);

    // Unpause
    client.set_paused(&false);
    assert!(!client.is_paused());
}

#[test]
fn test_sync_reconciles_reserves() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token_a, _token_b) = setup(&env);
    let stats = client.sync();
    assert_eq!(stats.reserve_a, 0);
    assert_eq!(stats.reserve_b, 0);
}
