extern crate std;

use super::*;
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::token;
use soroban_sdk::{Address, Env, IntoVal, Symbol, TryFromVal};

/// Register a pool + USDC token, initialise them, point `receive_payment` at a
/// dedicated `vault` address and fund that vault with `vault_funds` USDC.
fn setup<'a>(
    env: &'a Env,
    vault_funds: i128,
) -> (
    Address,
    Address,
    Address,
    RevenuePoolClient<'a>,
    token::Client<'a>,
    token::StellarAssetClient<'a>,
) {
    let admin = Address::generate(env);
    let vault = Address::generate(env);
    let pool_addr = env.register(RevenuePool, ());
    let client = RevenuePoolClient::new(env, &pool_addr);
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let usdc_address = sac.address();
    let usdc = token::Client::new(env, &usdc_address);
    let usdc_admin = token::StellarAssetClient::new(env, &usdc_address);

    client.init(&admin, &usdc_address);
    client.set_vault(&admin, &vault);
    usdc_admin.mint(&vault, &vault_funds);

    (admin, vault, pool_addr, client, usdc, usdc_admin)
}

#[test]
fn receive_payment_moves_usdc_from_vault_into_pool() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, vault, pool_addr, client, usdc, _usdc_admin) = setup(&env, 1_000_000);

    client.receive_payment(&vault, &250_000, &true);

    assert_eq!(usdc.balance(&pool_addr), 250_000);
    assert_eq!(usdc.balance(&vault), 750_000);
    assert_eq!(client.balance(), 250_000);
}

#[test]
fn receive_payment_emits_event_with_actual_transferred_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, vault, _pool_addr, client, _usdc, _usdc_admin) = setup(&env, 6_000_000);

    client.receive_payment(&vault, &5_000_000, &true);

    let events = env.events().all();
    let ev = events.last().unwrap();

    let topic0 = Symbol::try_from_val(&env, &ev.1.get(0).unwrap()).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "receive_payment"));
    let topic1 = Address::try_from_val(&env, &ev.1.get(1).unwrap()).unwrap();
    assert_eq!(topic1, vault);

    let (amount, from_vault): (i128, bool) = ev.2.into_val(&env);
    assert_eq!(amount, 5_000_000);
    assert!(from_vault);
}

#[test]
fn receive_payment_event_shape_is_two_topics_and_amount_first() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, vault, _pool_addr, client, _usdc, _usdc_admin) = setup(&env, 1_000_000);

    client.receive_payment(&vault, &1_000, &false);

    let ev = env.events().all().last().unwrap();
    assert_eq!(ev.1.len(), 2, "receive_payment must have exactly 2 topics");
    let t0: Symbol = ev.1.get(0).unwrap().into_val(&env);
    assert_eq!(t0, Symbol::new(&env, "receive_payment"));
    let t1: Address = ev.1.get(1).unwrap().into_val(&env);
    assert_eq!(t1, vault);
    let (amount, from_vault): (i128, bool) = ev.2.into_val(&env);
    assert_eq!(amount, 1_000);
    assert!(!from_vault);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn receive_payment_rejects_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, vault, _pool_addr, client, _usdc, _usdc_admin) = setup(&env, 1_000_000);
    client.receive_payment(&vault, &0, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn receive_payment_rejects_negative_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, vault, _pool_addr, client, _usdc, _usdc_admin) = setup(&env, 1_000_000);
    client.receive_payment(&vault, &-1, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #26)")]
fn receive_payment_rejects_caller_that_is_not_the_vault() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _vault, _pool_addr, client, _usdc, _usdc_admin) = setup(&env, 1_000_000);
    let attacker = Address::generate(&env);
    client.receive_payment(&attacker, &250, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #26)")]
fn receive_payment_rejects_the_admin_when_admin_is_not_the_vault() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _vault, _pool_addr, client, _usdc, _usdc_admin) = setup(&env, 1_000_000);
    // Admin keeps config powers but no longer bypasses the vault gate.
    client.receive_payment(&admin, &250, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn receive_payment_before_set_vault_rejects() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let pool_addr = env.register(RevenuePool, ());
    let client = RevenuePoolClient::new(&env, &pool_addr);
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    client.init(&admin, &sac.address());
    client.receive_payment(&admin, &250, &true);
}

#[test]
#[should_panic]
fn receive_payment_without_balance_fails_and_moves_nothing() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, vault, pool_addr, client, usdc, _usdc_admin) = setup(&env, 10);

    // Vault only holds 10 USDC, so a 250 stroop pull must abort.
    client.receive_payment(&vault, &250, &true);

    assert_eq!(usdc.balance(&pool_addr), 0);
}

#[test]
fn get_vault_returns_configured_address_and_defaults_to_none() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let pool_addr = env.register(RevenuePool, ());
    let client = RevenuePoolClient::new(&env, &pool_addr);
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    client.init(&admin, &sac.address());

    assert_eq!(client.get_vault(), None);
    client.set_vault(&admin, &vault);
    assert_eq!(client.get_vault(), Some(vault));
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn set_vault_requires_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _vault, _pool_addr, client, _usdc, _usdc_admin) = setup(&env, 1_000);
    let attacker = Address::generate(&env);
    let other = Address::generate(&env);
    client.set_vault(&attacker, &other);
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn set_vault_rejects_the_pool_itself() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _vault, pool_addr, client, _usdc, _usdc_admin) = setup(&env, 1_000);
    client.set_vault(&admin, &pool_addr);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn set_vault_rejects_the_usdc_token() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let pool_addr = env.register(RevenuePool, ());
    let client = RevenuePoolClient::new(&env, &pool_addr);
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let usdc_address = sac.address();
    client.init(&admin, &usdc_address);
    client.set_vault(&admin, &usdc_address);
}
