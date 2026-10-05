use crate::{RevenuePool, RevenuePoolClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_balance_uninitialized_panics() {
    let env = Env::default();
    let addr = env.register(RevenuePool, ());
    let client = RevenuePoolClient::new(&env, &addr);

    // Calling balance before init should panic with NotInitialized (#3).
    client.balance();
}

#[test]
#[should_panic(expected = "Error(Contract, #26)")]
fn test_receive_payment_unauthorized_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let usdc = Address::generate(&env);
    let attacker = Address::generate(&env);

    let addr = env.register(RevenuePool, ());
    let client = RevenuePoolClient::new(&env, &addr);

    client.init(&admin, &usdc);
    // The configured vault is the admin, so any other caller is rejected with
    // UnauthorizedCaller (#26) before any token transfer is attempted.
    client.set_vault(&admin, &admin);

    client.receive_payment(&attacker, &100, &false);
}
