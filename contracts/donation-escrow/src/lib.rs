#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Map, Vec};

/// Number of independent verifier approvals required to unlock a milestone.
const APPROVAL_THRESHOLD: u32 = 3;
/// Number of independent verifiers that must be configured at initialization.
const REQUIRED_VERIFIER_COUNT: u32 = 5;

/// TTL management constants, expressed in ledgers.
const INSTANCE_TTL_LEDGERS: u32 = 103_680; // ~6 days at 5s per ledger
const INSTANCE_TTL_THRESHOLD: u32 = 17_280; // ~1 day
const PERSISTENT_TTL_LEDGERS: u32 = 518_400; // ~30 days
const PERSISTENT_TTL_THRESHOLD: u32 = 120_960; // ~7 days

/// Donation escrow enforcing 3-of-5 independent verifier consensus per milestone.
#[contract]
pub struct DonationEscrow;

/// Contract-level error codes.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    InvalidVerifierCount = 3,
    Unauthorized = 4,
    MilestoneNotFound = 5,
    AlreadyApproved = 6,
    AlreadyReleased = 7,
    ThresholdNotMet = 8,
}

/// Storage keys.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Verifiers,
    Milestone(u32),
}

/// Milestone record tracked by the escrow.
#[contracttype]
#[derive(Clone)]
pub struct Milestone {
    pub id: u32,
    pub total: i128,
    pub approvals: Map<Address, ()>,
    pub released: bool,
    pub recipient: Address,
}

#[contractimpl]
impl DonationEscrow {
    /// Initialize the escrow with an admin, exactly five independent verifiers, and a set of milestones.
    ///
    /// # Authorization
    /// The provided `admin` address must authorize this call.
    pub fn initialize(
        env: Env,
        admin: Address,
        verifiers: Vec<Address>,
        milestones: Vec<Milestone>,
    ) -> Result<(), Error> {
        admin.require_auth();

        if Self::is_initialized(&env) {
            return Err(Error::AlreadyInitialized);
        }
        if verifiers.len() != REQUIRED_VERIFIER_COUNT {
            return Err(Error::InvalidVerifierCount);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Verifiers, &verifiers);
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_LEDGERS);

        for i in 0..milestones.len() {
            let m = milestones.get(i).unwrap();
            let key = DataKey::Milestone(m.id);
            env.storage().persistent().set(&key, &m);
            env.storage().persistent().extend_ttl(
                &key,
                PERSISTENT_TTL_THRESHOLD,
                PERSISTENT_TTL_LEDGERS,
            );
        }

        Ok(())
    }

    /// Submit an independent verifier approval for a milestone.
    ///
    /// # Authorization
    /// The provided `verifier` address must authorize this call and be one of the configured verifiers.
    pub fn approve_milestone(
        env: Env,
        verifier: Address,
        milestone_id: u32,
    ) -> Result<(), Error> {
        verifier.require_auth();

        if !Self::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }
        if !Self::is_verifier(&env, &verifier) {
            return Err(Error::Unauthorized);
        }

        let key = DataKey::Milestone(milestone_id);
        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.released {
            return Err(Error::AlreadyReleased);
        }
        if milestone.approvals.get(verifier.clone()).is_some() {
            return Err(Error::AlreadyApproved);
        }

        milestone.approvals.set(verifier, ());
        env.storage().persistent().set(&key, &milestone);
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_LEDGERS,
        );
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_LEDGERS);

        Ok(())
    }

    /// Release a milestone once at least three of the five verifiers have approved it.
    ///
    /// No further authorization is required because the 3-of-5 consensus itself authorizes release.
    pub fn release_milestone(env: Env, milestone_id: u32) -> Result<(), Error> {
        if !Self::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        let key = DataKey::Milestone(milestone_id);
        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::MilestoneNotFound)?;

        if milestone.released {
            return Err(Error::AlreadyReleased);
        }
        if milestone.approvals.len() < APPROVAL_THRESHOLD {
            return Err(Error::ThresholdNotMet);
        }

        milestone.released = true;
        env.storage().persistent().set(&key, &milestone);
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_LEDGERS,
        );
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_LEDGERS);

        Ok(())
    }

    /// Return the current state of a milestone.
    pub fn get_milestone(env: Env, milestone_id: u32) -> Result<Milestone, Error> {
        if !Self::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        let key = DataKey::Milestone(milestone_id);
        env.storage().persistent().get(&key).ok_or(Error::MilestoneNotFound)
    }

    fn is_initialized(env: &Env) -> bool {
        env.storage().instance().has(&DataKey::Admin)
    }

    fn is_verifier(env: &Env, addr: &Address) -> bool {
        let verifiers: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Verifiers)
            .unwrap_or_else(|| Vec::new(env));
        for i in 0..verifiers.len() {
            if verifiers.get(i).unwrap().eq(addr) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod test;
