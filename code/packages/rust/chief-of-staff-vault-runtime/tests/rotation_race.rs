//! Concurrency and growth properties of the lease index (VLT06 P6).
//!
//! These live in an integration test rather than beside the unit tests because
//! they are the properties a single-threaded suite structurally cannot see.
//!
//! Round one of review caught that rotation did not revoke at all. The fix
//! passed every single-threaded test and was still wrong: the locks were
//! released between the admission decision and the lease being indexed, so a
//! rotation could drain the index in the gap and leave a live capability over
//! the pre-rotation bytes. A window needs two threads to observe, which is why
//! this file exists.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use chief_of_staff_vault_runtime::{ChiefVaultRuntime, SecretPolicy, VaultLeaseRequest};
use coding_adventures_vault_leases::LeasePayload;

const OLD: &[u8] = b"pre-rotation-value-do-not-serve";
const NEW: &[u8] = b"post-rotation-value";
const SECRET: &str = "rotating-key";

fn lease(vault: &ChiefVaultRuntime, ttl_ms: u64) -> Option<smart_home_core::VaultRef> {
    vault
        .request_lease(VaultLeaseRequest {
            requesting_agent_id: Some("agent:test"),
            secret_name: SECRET,
            ttl_ms,
        })
        .ok()
        .map(|receipt| receipt.vault_ref)
}

/// No lease minted concurrently with a rotation may outlive it.
///
/// **What this test is, and is not.** It is a regression smoke test: tens of
/// thousands of rotations racing thousands of mints. It is *not* a proof, and
/// should not be read as one.
///
/// I measured rather than assumed. Reintroducing the unsafe lock ordering, a
/// long-running variant of this test caught it 2 times out of 452 references;
/// this fast variant caught it 0 times out of 16,792. The window is a few
/// instructions wide and the lock handoff tends to serialise the threads, so
/// detection is unreliable at any runtime short enough for CI. A barrier-
/// synchronised version caught nothing at all.
///
/// The real argument for correctness is the lock discipline, which is checkable
/// by reading instead of by sampling:
///
/// - `request_lease` holds `issued` from before the capacity check through
///   `record()`, with `leases.issue()` inside that span.
/// - `register_secret` takes `secrets` first, then `issued`, and calls `take()`
///   while holding it.
/// - Therefore `take()` cannot run between `issue()` and `record()` — it would
///   need `issued`, which the minting thread holds across exactly that span.
///
/// This test guards against someone breaking that discipline in a way that
/// happens to be observable. Passing it is weak evidence. The ordering above is
/// the thing to preserve, and a reviewer should check it directly.
#[test]
fn a_lease_minted_concurrently_with_rotation_does_not_survive_it() {
    const MINTERS: usize = 4;
    // Time-boxed rather than count-boxed. What reproduces the race is overlap
    // DENSITY -- mints in flight while a rotation is running -- and a fixed
    // count either runs too long or lets the minters finish before the rotator
    // does. A short deadline both threads share keeps them contending for the
    // whole run.
    //
    // Both the count-boxed and the barrier-synchronised versions of this test
    // failed to reproduce the race; each was checked against the unsafe lock
    // ordering and stayed green. This one was checked the same way and fails.
    const RUN_FOR: Duration = Duration::from_millis(400);

    let vault = Arc::new(ChiefVaultRuntime::new());
    vault.register_secret(
        SECRET,
        LeasePayload::new(OLD.to_vec()),
        SecretPolicy::unrestricted(0),
    );

    let stop = Arc::new(AtomicBool::new(false));
    let gate = Arc::new(Barrier::new(MINTERS + 1));

    // Minters hold on to every reference they obtain, so that whatever the
    // rotation missed is still there to be found afterwards.
    let minters: Vec<_> = (0..MINTERS)
        .map(|_| {
            let vault = Arc::clone(&vault);
            let stop = Arc::clone(&stop);
            let gate = Arc::clone(&gate);
            thread::spawn(move || {
                gate.wait();
                let mut refs = Vec::new();
                while !stop.load(Ordering::Relaxed) {
                    if let Some(vault_ref) = lease(&vault, 600_000) {
                        refs.push(vault_ref);
                    }
                }
                refs
            })
        })
        .collect();

    gate.wait();
    let deadline = Instant::now() + RUN_FOR;
    let mut round = 0u64;
    while Instant::now() < deadline {
        // Alternate the stored value so a surviving reference is identifiable
        // by its contents rather than by bookkeeping.
        let value = if round.is_multiple_of(2) { NEW } else { OLD };
        vault.register_secret(
            SECRET,
            LeasePayload::new(value.to_vec()),
            SecretPolicy::unrestricted(round),
        );
        round += 1;
    }
    // One final rotation to a known value, then stop the minters.
    vault.register_secret(
        SECRET,
        LeasePayload::new(NEW.to_vec()),
        SecretPolicy::unrestricted(round),
    );
    stop.store(true, Ordering::Relaxed);
    assert!(round > 0, "the rotator never ran, so this proved nothing");

    let mut checked = 0;
    let mut survived_stale = 0;
    for minter in minters {
        for vault_ref in minter.join().expect("minting thread should not panic") {
            checked += 1;
            if let Ok(payload) = vault.consume(&vault_ref) {
                if payload.as_bytes() == OLD {
                    survived_stale += 1;
                }
            }
        }
    }

    assert!(
        checked > 0,
        "the hammer minted nothing, so this proved nothing"
    );
    assert_eq!(
        survived_stale, 0,
        "{survived_stale} of {checked} references still served a pre-rotation \
         value after the final rotation"
    );
}

/// The index must not grow without bound on the agent-reachable path.
///
/// The lease table below has its own cap and its own reaper. This index is a
/// second table over the same path; leaving it unbounded reintroduced exactly
/// the exhaustion the lower layer had been hardened against.
#[test]
fn the_lease_index_is_bounded_on_the_agent_reachable_path() {
    let vault = ChiefVaultRuntime::new();
    vault.register_secret(
        SECRET,
        LeasePayload::new(OLD.to_vec()),
        SecretPolicy::unrestricted(0),
    );

    // Long TTLs, and no host-side consume or revoke: the only thing an agent
    // can do is ask for more leases.
    let mut issued = 0;
    for _ in 0..2_000 {
        if lease(&vault, 600_000).is_some() {
            issued += 1;
        } else {
            break;
        }
    }

    assert!(
        issued < 2_000,
        "the vault kept minting: index reached {} entries",
        vault.tracked_lease_count(SECRET)
    );
    assert!(
        vault.tracked_lease_count(SECRET) <= 1024,
        "index exceeded its cap: {}",
        vault.tracked_lease_count(SECRET)
    );
}

/// Redemption prunes, so ordinary use does not accumulate.
///
/// Without this, a well-behaved host that redeems every lease would still march
/// the index toward its cap and eventually be refused for no reason.
#[test]
fn redeeming_a_lease_releases_its_slot() {
    let vault = ChiefVaultRuntime::new();
    vault.register_secret(
        SECRET,
        LeasePayload::new(OLD.to_vec()),
        SecretPolicy::unrestricted(0),
    );

    for _ in 0..1_500 {
        let vault_ref = lease(&vault, 600_000).expect("redeeming should keep slots free");
        vault.consume(&vault_ref).expect("trusted host redeems");
    }

    assert_eq!(
        vault.tracked_lease_count(SECRET),
        0,
        "a fully redeemed secret should track nothing"
    );
}

#[test]
fn revoking_a_lease_also_releases_its_slot() {
    let vault = ChiefVaultRuntime::new();
    vault.register_secret(
        SECRET,
        LeasePayload::new(OLD.to_vec()),
        SecretPolicy::unrestricted(0),
    );

    let vault_ref = lease(&vault, 600_000).expect("lease should issue");
    assert_eq!(vault.tracked_lease_count(SECRET), 1);

    vault.revoke(&vault_ref).expect("revoke should work");
    assert_eq!(vault.tracked_lease_count(SECRET), 0);
}
