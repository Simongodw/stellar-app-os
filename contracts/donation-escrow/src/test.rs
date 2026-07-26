use crate::*;
use soroban_sdk::{testutils::Address as _, vec, Address, Env, Map, Vec};

fn setup(
    env: &Env,
) -> (
    DonationEscrowClient<'_>,
    Address,
    Vec<Address>,
    Vec<Milestone>,
) {
    env.mock_all_auths();

    let admin = Address::generate(env);
    let recipient = Address::generate(env);
    let v1 = Address::generate(env);
    let v2 = Address::generate(env);
    let v3 = Address::generate(env);
    let v4 = Address::generate(env);
    let v5 = Address::generate(env);

    let verifiers = vec![env, v1, v2, v3, v4, v5];

    let milestone = Milestone {
        id: 1,
        total: 1_000,
        approvals: Map::new(env),
        released: false,
        recipient,
    };
    let milestones = vec![env, milestone];

    let contract_id = env.register(DonationEscrow, ());
    let client = DonationEscrowClient::new(env, &contract_id);
    client.initialize(&admin, &verifiers, &milestones);

    (client, admin, verifiers, milestones)
}

#[test]
fn three_of_five_releases_milestone() {
    let env = Env::default();
    let (client, _admin, verifiers, _milestones) = setup(&env);

    let v1 = verifiers.get(0).unwrap();
    let v2 = verifiers.get(1).unwrap();
    let v3 = verifiers.get(2).unwrap();

    client.approve_milestone(&v1, &1u32);
    client.approve_milestone(&v2, &1u32);
    client.approve_milestone(&v3, &1u32);
    client.release_milestone(&1u32);

    let m = client.get_milestone(&1u32);
    assert!(m.released);
    assert_eq!(m.approvals.len(), 3);
}

#[test]
fn release_before_threshold_fails() {
    let env = Env::default();
    let (client, _admin, verifiers, _milestones) = setup(&env);

    let v1 = verifiers.get(0).unwrap();
    let v2 = verifiers.get(1).unwrap();

    client.approve_milestone(&v1, &1u32);
    client.approve_milestone(&v2, &1u32);
    assert_eq!(
        client.try_release_milestone(&1u32),
        Err(Ok(Error::ThresholdNotMet))
    );
}

#[test]
fn non_verifier_approval_fails() {
    let env = Env::default();
    let (client, _admin, _verifiers, _milestones) = setup(&env);

    let stranger = Address::generate(&env);
    assert_eq!(
        client.try_approve_milestone(&stranger, &1u32),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn duplicate_approval_fails() {
    let env = Env::default();
    let (client, _admin, verifiers, _milestones) = setup(&env);

    let v1 = verifiers.get(0).unwrap();
    client.approve_milestone(&v1, &1u32);
    assert_eq!(
        client.try_approve_milestone(&v1, &1u32),
        Err(Ok(Error::AlreadyApproved))
    );
}

#[test]
fn release_already_released_fails() {
    let env = Env::default();
    let (client, _admin, verifiers, _milestones) = setup(&env);

    let v1 = verifiers.get(0).unwrap();
    let v2 = verifiers.get(1).unwrap();
    let v3 = verifiers.get(2).unwrap();

    client.approve_milestone(&v1, &1u32);
    client.approve_milestone(&v2, &1u32);
    client.approve_milestone(&v3, &1u32);
    client.release_milestone(&1u32);

    assert_eq!(
        client.try_release_milestone(&1u32),
        Err(Ok(Error::AlreadyReleased))
    );
}

#[test]
fn invalid_verifier_count_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let v1 = Address::generate(&env);
    let v2 = Address::generate(&env);
    let v3 = Address::generate(&env);

    let verifiers = vec![&env, v1, v2, v3];
    let milestone = Milestone {
        id: 1,
        total: 1_000,
        approvals: Map::new(&env),
        released: false,
        recipient: Address::generate(&env),
    };
    let milestones = vec![&env, milestone];

    let contract_id = env.register(DonationEscrow, ());
    let client = DonationEscrowClient::new(&env, &contract_id);

    assert_eq!(
        client.try_initialize(&admin, &verifiers, &milestones),
        Err(Ok(Error::InvalidVerifierCount))
    );
}

#[test]
fn double_initialize_fails() {
    let env = Env::default();
    let (client, admin, verifiers, milestones) = setup(&env);

    assert_eq!(
        client.try_initialize(&admin, &verifiers, &milestones),
        Err(Ok(Error::AlreadyInitialized))
    );
}
