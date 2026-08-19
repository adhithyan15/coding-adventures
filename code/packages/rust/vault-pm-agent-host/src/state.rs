//! The agent's in-memory passphrase retention store.
//!
//! This module owns no socket and no thread. It is deliberately pure
//! bookkeeping — a map from vault name to a retained passphrase and an
//! expiry — so the idle-bound policy (VLT-PM48 §5) can be unit-tested without
//! a real connection, a real clock, or a real process, the same way
//! `vault-pm-cli::shell::ShellSession` is tested one layer up.
//!
//! # Why `Instant`, not the advisory host clock
//!
//! `ShellSession` (VLT-PM40 §3.5) measures elapsed time against an
//! injected, advisory *wall* clock, because it must interoperate with a host
//! trait whose other methods already use wall time, and because a shell
//! foreground process has no reason to prefer anything else. This store uses
//! [`std::time::Instant`] instead: the agent is a long-lived background
//! process with no host trait to match, and `Instant` is monotonic by
//! construction, which removes an entire class of failure `ShellSession` has
//! to defend against explicitly — a wall clock that steps backwards (an NTP
//! correction, a manual clock change) can never make a retained passphrase
//! look fresher than it is.

use coding_adventures_zeroize::Zeroizing;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// One vault's retained passphrase and the policy that expires it.
struct RetainedPassphrase {
    passphrase: Zeroizing<Vec<u8>>,
    retained_at: Instant,
    idle_bound: Duration,
}

impl RetainedPassphrase {
    fn expired_at(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.retained_at) >= self.idle_bound
    }

    fn remaining(&self, now: Instant) -> Duration {
        self.idle_bound
            .saturating_sub(now.saturating_duration_since(self.retained_at))
    }
}

/// One vault's retention status, independent of the wire protocol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultStatus {
    /// The vault this entry describes.
    pub vault_name: String,
    /// Time remaining before this passphrase expires on its own.
    pub remaining: Duration,
}

/// The complete retained-passphrase state for every vault the agent has
/// unlocked since it started.
///
/// Not internally synchronized. The server wraps one instance in a
/// `std::sync::Mutex`, matching the rest of this product's convention of
/// keeping thread-safety at the boundary that needs it rather than baking a
/// lock into a type that a single-threaded unit test would then have to pay
/// for.
#[derive(Default)]
pub struct AgentState {
    retained: BTreeMap<String, RetainedPassphrase>,
}

impl AgentState {
    /// Begin with nothing retained.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retain one vault's passphrase for up to `idle_bound` of inactivity.
    ///
    /// The caller has already verified this passphrase against the real
    /// vault (VLT-PM48 §4.2) — this store performs no authentication and
    /// trusts every `unlock` it receives. Retaining unconditionally replaces
    /// any value already held for the same name, restarting its idle bound;
    /// this is the ordinary "type the passphrase again" case, not a
    /// distinguishable error.
    pub fn unlock(
        &mut self,
        vault_name: String,
        passphrase: Zeroizing<Vec<u8>>,
        idle_bound: Duration,
        now: Instant,
    ) {
        self.retained.insert(
            vault_name,
            RetainedPassphrase {
                passphrase,
                retained_at: now,
                idle_bound,
            },
        );
    }

    /// Return one vault's retained passphrase, or `None` if it was never
    /// retained or has since expired.
    ///
    /// Expiry is checked here, at the point of use, mirroring
    /// `ShellSession::authenticator`'s own double-check discipline (VLT-PM40
    /// §3.5): a background sweep (see [`Self::sweep_expired`]) removes stale
    /// entries on a timer, but a request that lands in the gap between two
    /// sweeps must not be handed a passphrase that has already expired.
    pub fn get(&mut self, vault_name: &str, now: Instant) -> Option<Zeroizing<Vec<u8>>> {
        let entry = self.retained.get(vault_name)?;
        if entry.expired_at(now) {
            self.retained.remove(vault_name);
            return None;
        }
        Some(Zeroizing::new(entry.passphrase.to_vec()))
    }

    /// Forget one vault's retained passphrase, or every vault's when `None`.
    ///
    /// Forgetting a vault that was never retained (or already expired) is
    /// not an error: repeating this is harmless, the same contract
    /// `ShellSession::lock` and the interactive `lock` verb already promise.
    pub fn lock(&mut self, vault_name: Option<&str>) {
        match vault_name {
            Some(name) => {
                self.retained.remove(name);
            }
            None => self.retained.clear(),
        }
    }

    /// Remove every entry whose idle bound has elapsed, and return the count.
    ///
    /// Called by the server's background sweep thread. This is what gives
    /// the agent *pre-emptive* auto-lock — the property `vault-pm shell`
    /// explicitly deferred to this slice (VLT-PM40 §3.5) because a foreground
    /// process blocked on a terminal read has nothing for a timer to run in.
    /// A background process has exactly that.
    pub fn sweep_expired(&mut self, now: Instant) -> usize {
        let expired: Vec<String> = self
            .retained
            .iter()
            .filter(|(_, entry)| entry.expired_at(now))
            .map(|(name, _)| name.clone())
            .collect();
        for name in &expired {
            self.retained.remove(name);
        }
        expired.len()
    }

    /// Every currently retained, unexpired vault, in name order.
    ///
    /// Sweeps first, so a status report never lists a vault whose passphrase
    /// has already expired even if the background sweep has not yet caught
    /// up to it.
    pub fn status(&mut self, now: Instant) -> Vec<VaultStatus> {
        self.sweep_expired(now);
        self.retained
            .iter()
            .map(|(name, entry)| VaultStatus {
                vault_name: name.clone(),
                remaining: entry.remaining(now),
            })
            .collect()
    }

    /// Whether anything is currently retained. Used by the server to decide
    /// whether an idle sweep tick has any work to do.
    pub fn is_empty(&self) -> bool {
        self.retained.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passphrase(value: &[u8]) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(value.to_vec())
    }

    #[test]
    fn unlock_then_get_returns_the_same_bytes() {
        let mut state = AgentState::new();
        let now = Instant::now();
        state.unlock(
            "personal".to_owned(),
            passphrase(b"correct horse"),
            Duration::from_secs(300),
            now,
        );
        assert_eq!(
            state
                .get("personal", now)
                .as_ref()
                .map(|value| value.as_slice()),
            Some(b"correct horse".as_slice())
        );
        assert!(
            state.get("work", now).is_none(),
            "unrelated vault stays absent"
        );
    }

    #[test]
    fn get_after_the_idle_bound_returns_none_and_forgets_it() {
        let mut state = AgentState::new();
        let start = Instant::now();
        state.unlock(
            "personal".to_owned(),
            passphrase(b"correct horse"),
            Duration::from_millis(10),
            start,
        );
        let later = start + Duration::from_millis(11);
        assert!(state.get("personal", later).is_none());
        // Forgotten, not merely reported as expired: a second read at the
        // same instant still finds nothing, proving the entry was removed
        // rather than left for a caller to keep tripping over.
        assert!(state.status(later).is_empty());
    }

    #[test]
    fn a_second_unlock_replaces_the_first_and_restarts_the_bound() {
        let mut state = AgentState::new();
        let start = Instant::now();
        state.unlock(
            "personal".to_owned(),
            passphrase(b"first"),
            Duration::from_millis(10),
            start,
        );
        let almost_expired = start + Duration::from_millis(9);
        state.unlock(
            "personal".to_owned(),
            passphrase(b"second"),
            Duration::from_millis(10),
            almost_expired,
        );
        // The bound restarted, so a moment that would have expired the first
        // value does not expire the second.
        let just_after_original_bound = start + Duration::from_millis(11);
        assert_eq!(
            state
                .get("personal", just_after_original_bound)
                .as_ref()
                .map(|value| value.as_slice()),
            Some(b"second".as_slice())
        );
    }

    #[test]
    fn lock_forgets_one_vault_or_every_vault() {
        let mut state = AgentState::new();
        let now = Instant::now();
        state.unlock(
            "personal".to_owned(),
            passphrase(b"p"),
            Duration::from_secs(300),
            now,
        );
        state.unlock(
            "work".to_owned(),
            passphrase(b"w"),
            Duration::from_secs(300),
            now,
        );

        state.lock(Some("personal"));
        assert!(state.get("personal", now).is_none());
        assert!(state.get("work", now).is_some());

        // Repeating a lock on an already-forgotten vault is harmless.
        state.lock(Some("personal"));

        state.lock(None);
        assert!(state.get("work", now).is_none());
        assert!(state.is_empty());
    }

    #[test]
    fn sweep_expired_removes_only_what_has_actually_expired() {
        let mut state = AgentState::new();
        let start = Instant::now();
        state.unlock(
            "short".to_owned(),
            passphrase(b"s"),
            Duration::from_millis(10),
            start,
        );
        state.unlock(
            "long".to_owned(),
            passphrase(b"l"),
            Duration::from_secs(300),
            start,
        );
        let later = start + Duration::from_millis(11);
        assert_eq!(state.sweep_expired(later), 1);
        let remaining = state.status(later);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].vault_name, "long");
    }

    #[test]
    fn status_reports_names_in_order_with_remaining_time() {
        let mut state = AgentState::new();
        let now = Instant::now();
        state.unlock(
            "work".to_owned(),
            passphrase(b"w"),
            Duration::from_secs(300),
            now,
        );
        state.unlock(
            "personal".to_owned(),
            passphrase(b"p"),
            Duration::from_secs(100),
            now,
        );
        let report = state.status(now + Duration::from_secs(10));
        assert_eq!(report.len(), 2);
        assert_eq!(report[0].vault_name, "personal");
        assert_eq!(report[0].remaining, Duration::from_secs(90));
        assert_eq!(report[1].vault_name, "work");
        assert_eq!(report[1].remaining, Duration::from_secs(290));
    }
}
