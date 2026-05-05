//! # `coding_adventures_vault_revisions` — VLT12 revision history
//!
//! ## What this crate is
//!
//! The **revision history** layer of the Vault stack. Every
//! `put` at the application tier archives the *prior* ciphertext
//! to a sibling history list keyed by `(namespace, key)`, so a
//! user can:
//!
//! ```text
//!   $ vault put kv/login/github 'newpassword'
//!   $ vault put kv/login/github 'oops-typo'
//!   $ vault put kv/login/github 'newpassword'
//!   $ vault revisions kv/login/github
//!     rev    archived_at
//!     ----   ---------------------
//!     1      2026-05-04T16:00:00Z
//!     2      2026-05-04T16:01:00Z
//!     3      2026-05-04T16:02:00Z
//!   $ vault restore kv/login/github --rev 1
//! ```
//!
//! `restore(ns, key, rev)` is *not* a rollback — it appends the
//! restored ciphertext as a new revision. The history list is
//! append-only; old revisions disappear only via the documented
//! per-namespace retention policy. This matches HashiCorp Vault
//! KV-v2's semantics, 1Password's "version history", and
//! Bitwarden's (premium) password history.
//!
//! ## Storage-agnostic by construction
//!
//! Like VLT10 sync, this crate sees only **ciphertext bytes**.
//! The sealing happens above (VLT01 sealed-store). A
//! storage-backed sibling crate (`vault-revisions-fs`,
//! `vault-revisions-postgres`) reads the ciphertext payload and
//! persists it through a `StorageBackend` — and at no point does
//! the storage layer see plaintext.
//!
//! ## Retention policy
//!
//! Two complementary caps, applied per-namespace:
//!
//!   * `max_revisions_per_key` — keep at most N revisions per
//!     `(namespace, key)`. Older revisions are evicted (oldest
//!     first) on every archive.
//!   * `max_age_ms` — drop revisions older than `now -
//!     max_age_ms` whenever `purge_due(now_ms)` is called by the
//!     host (caller-driven so the crate stays clock-pure).
//!
//! Either or both may be `None` to disable that bound. A
//! reasonable default is 32 revisions per key, 90 days.
//!
//! ## Where it fits
//!
//! ```text
//!   ┌──────────────────────────────────────────────────┐
//!   │  application (Bitwarden / 1Password / Vault)     │
//!   └────────────────────┬─────────────────────────────┘
//!                        │ "put X" (new ciphertext)
//!   ┌────────────────────▼─────────────────────────────┐
//!   │  RevisionStore::archive(prior ciphertext)        │  (this crate)
//!   │   - assigns next monotonic revision id            │
//!   │   - applies retention policy                     │
//!   └────────────────────┬─────────────────────────────┘
//!                        │
//!   ┌────────────────────▼─────────────────────────────┐
//!   │  storage backend (in-memory / fs / postgres)     │
//!   │   sees ciphertext only                            │
//!   └───────────────────────────────────────────────────┘
//! ```
//!
//! ## Threat model
//!
//! * **Storage tampering**: detected one tier above (VLT09 audit
//!   log). This crate's `archive` simply produces a new revision
//!   record; if a storage layer rewrites it, the integrity
//!   check fails when the audit log is verified.
//! * **Replay**: a stale ciphertext re-submitted as the "next"
//!   archive will simply create another revision row — that's
//!   the design (history is append-only). The dedup safeguard
//!   is a per-key `max_revisions_per_key` cap so an attacker
//!   cannot blow up storage with replay floods.
//! * **Information leakage**: every variable-length field is
//!   bounded (`MAX_*_LEN` constants); validation runs on every
//!   archive.
//! * **`restore` exposure**: `restore` returns the historical
//!   ciphertext to the caller, who is responsible for either
//!   re-encrypting (recommended — re-wrap under the current key)
//!   or carrying it forward as-is. The crate doesn't decide;
//!   that's an application policy choice (see VLT01 sealed-store
//!   for re-wrap helpers).
//!
//! ## What this crate is NOT
//!
//! * Not encryption — ciphertext is opaque to this layer.
//! * Not authentication — VLT05 above gates `archive`/`restore`.
//! * Not authorization — VLT06 above applies policy.
//! * Not audit — the chain of archive events is recorded by
//!   VLT09; this crate doesn't write its own audit trail.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Best-effort lock that recovers from a poisoned mutex rather
/// than panicking. Same pattern as `vault-audit::lock_recover`.
fn lock_recover<'a, T>(m: &'a Mutex<T>) -> MutexGuard<'a, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

// === Section 1. Bounds =====================================================

/// Maximum size (bytes) for a single ciphertext payload.
/// Matches `vault-sync`'s cap so a record can flow through both
/// layers identically. Larger payloads should be chunked at
/// VLT14 attachments.
pub const MAX_CIPHERTEXT_LEN: usize = 1024 * 1024;
/// Maximum bytes for a namespace.
pub const MAX_NAMESPACE_LEN: usize = 128;
/// Maximum bytes for a key.
pub const MAX_KEY_LEN: usize = 512;

// === Section 2. Vocabulary types ===========================================

/// One archived revision. Server-visible fields only — no
/// plaintext, no record type, no application metadata.
///
/// `Debug` is hand-rolled to redact `ciphertext` (lengths only).
/// Even though the bytes are encrypted, logging them is a
/// fingerprint surface and the derived `Vec<u8>` Debug would
/// dump every byte.
#[derive(Clone, PartialEq, Eq)]
pub struct Revision {
    /// Monotonic per-`(namespace, key)` revision id. Starts at 1
    /// for the first archive at that path.
    pub id: u64,
    /// Wall-clock time of the archive, ms since UNIX epoch.
    /// Caller-supplied so the crate stays clock-pure.
    pub archived_at_ms: u64,
    /// The encrypted record body (opaque). Bounded by
    /// [`MAX_CIPHERTEXT_LEN`].
    pub ciphertext: Vec<u8>,
}

impl core::fmt::Debug for Revision {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Revision")
            .field("id", &self.id)
            .field("archived_at_ms", &self.archived_at_ms)
            .field(
                "ciphertext",
                &format_args!("<{} bytes redacted>", self.ciphertext.len()),
            )
            .finish()
    }
}

/// Read-only metadata view of a revision. No ciphertext —
/// callers opt explicitly into [`RevisionStore::get_revision`]
/// to obtain the bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionMeta {
    /// Echo of the revision id.
    pub id: u64,
    /// Echo of the archived-at timestamp.
    pub archived_at_ms: u64,
    /// Length in bytes of the underlying ciphertext.
    pub ciphertext_len: usize,
}

/// Per-namespace retention policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetentionPolicy {
    /// Maximum number of revisions kept per key. Older revisions
    /// are evicted oldest-first whenever a new one is archived.
    /// `None` → unbounded by count.
    pub max_revisions_per_key: Option<usize>,
    /// Drop revisions older than `now - max_age_ms` whenever
    /// `purge_due(now_ms)` is called. `None` → unbounded by age.
    pub max_age_ms: Option<u64>,
}

impl RetentionPolicy {
    /// A reasonable default: 32 revisions per key, 90 days max
    /// age. Both bounds are caller-overridable.
    pub fn default_password_manager() -> Self {
        Self {
            max_revisions_per_key: Some(32),
            max_age_ms: Some(90 * 24 * 60 * 60 * 1_000),
        }
    }

    /// "Never evict" — useful for compliance-driven workflows
    /// where audit retention is explicit.
    pub fn unbounded() -> Self {
        Self {
            max_revisions_per_key: None,
            max_age_ms: None,
        }
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::default_password_manager()
    }
}

/// All errors produced by the revision-history layer.
#[derive(Debug)]
pub enum RevisionError {
    /// Caller supplied a malformed argument (oversize / empty /
    /// control char).
    InvalidParameter(&'static str),
    /// `(namespace, key)` exists but the requested revision id
    /// does not (never archived, or already evicted).
    UnknownRevision,
    /// `(namespace, key)` has no archived revisions.
    NotFound,
    /// Backend-level failure (durable store unavailable, etc).
    /// Reference in-memory impl never produces this.
    Backend(String),
    /// Revision id counter overflow on a 64-bit chain
    /// (unreachable in practice, included for correctness).
    Overflow,
}

impl core::fmt::Display for RevisionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidParameter(why) => write!(f, "invalid parameter: {}", why),
            Self::UnknownRevision => write!(f, "unknown revision id"),
            Self::NotFound => write!(f, "no revisions found at this path"),
            Self::Backend(why) => write!(f, "backend error: {}", why),
            Self::Overflow => write!(f, "revision id counter overflow"),
        }
    }
}

impl std::error::Error for RevisionError {}

// === Section 3. Validation =================================================

fn validate_namespace(ns: &str) -> Result<(), RevisionError> {
    if ns.is_empty() {
        return Err(RevisionError::InvalidParameter("namespace must not be empty"));
    }
    if ns.len() > MAX_NAMESPACE_LEN {
        return Err(RevisionError::InvalidParameter(
            "namespace exceeds MAX_NAMESPACE_LEN",
        ));
    }
    if !is_safe_id_string(ns) {
        return Err(RevisionError::InvalidParameter(
            "namespace contains forbidden characters",
        ));
    }
    Ok(())
}

fn validate_retention(p: &RetentionPolicy) -> Result<(), RevisionError> {
    if let Some(0) = p.max_revisions_per_key {
        return Err(RevisionError::InvalidParameter(
            "max_revisions_per_key must be >= 1; use None to disable",
        ));
    }
    Ok(())
}

fn validate_namespace_key(ns: &str, key: &str) -> Result<(), RevisionError> {
    validate_namespace(ns)?;
    if key.is_empty() {
        return Err(RevisionError::InvalidParameter("key must not be empty"));
    }
    if key.len() > MAX_KEY_LEN {
        return Err(RevisionError::InvalidParameter("key exceeds MAX_KEY_LEN"));
    }
    if !is_safe_id_string(key) {
        return Err(RevisionError::InvalidParameter(
            "key contains forbidden characters",
        ));
    }
    Ok(())
}

fn validate_ciphertext(ct: &[u8]) -> Result<(), RevisionError> {
    if ct.is_empty() {
        return Err(RevisionError::InvalidParameter(
            "ciphertext must not be empty",
        ));
    }
    if ct.len() > MAX_CIPHERTEXT_LEN {
        return Err(RevisionError::InvalidParameter(
            "ciphertext exceeds MAX_CIPHERTEXT_LEN",
        ));
    }
    Ok(())
}

/// Reject identifiers containing control characters, whitespace,
/// or Unicode bidi-override / zero-width codepoints. Same defence
/// as `vault-sync` and `vault-transport-cli`.
fn is_safe_id_string(s: &str) -> bool {
    for c in s.chars() {
        if c.is_control() || c.is_whitespace() {
            return false;
        }
        let cp = c as u32;
        if matches!(cp, 0x202A..=0x202E | 0x2066..=0x2069)
            || matches!(cp, 0x200B..=0x200D | 0xFEFF)
        {
            return false;
        }
    }
    true
}

// === Section 4. Trait =====================================================

/// The contract every revision store implements. `Send + Sync`
/// + object-safe so a single `Arc<dyn RevisionStore>` can serve
/// the whole stack.
pub trait RevisionStore: Send + Sync {
    /// Archive a ciphertext. Returns the resulting [`Revision`]
    /// metadata (including the freshly-allocated id). On a key
    /// with no prior history this allocates id `1`.
    fn archive(
        &self,
        namespace: &str,
        key: &str,
        ciphertext: Vec<u8>,
        archived_at_ms: u64,
    ) -> Result<Revision, RevisionError>;

    /// List metadata for every revision at `(namespace, key)`,
    /// sorted by id ascending. Empty list ⇒ no revisions.
    /// Does *not* return ciphertext bytes — callers opt in via
    /// [`Self::get_revision`].
    fn list(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Vec<RevisionMeta>, RevisionError>;

    /// Fetch one revision by id, with its ciphertext.
    fn get_revision(
        &self,
        namespace: &str,
        key: &str,
        id: u64,
    ) -> Result<Revision, RevisionError>;

    /// "Restore" — fetch revision `id`, then archive its
    /// ciphertext as a *new* revision. Returns the new revision
    /// metadata. Called by upper layers after the policy /
    /// audit checks succeed; this crate doesn't gate it.
    fn restore(
        &self,
        namespace: &str,
        key: &str,
        id: u64,
        archived_at_ms: u64,
    ) -> Result<Revision, RevisionError>;

    /// Evict revisions older than `now_ms - retention.max_age_ms`
    /// for a given namespace. Returns the count of evicted rows.
    /// Callers drive this from a maintenance loop / cron; the
    /// crate has no built-in timer.
    fn purge_due(
        &self,
        namespace: &str,
        retention: &RetentionPolicy,
        now_ms: u64,
    ) -> Result<usize, RevisionError>;

    /// Effective policy in force for a namespace. Used by
    /// `archive` to decide eviction.
    ///
    /// Returns `RetentionPolicy::default()` for an unknown
    /// (or invalid) namespace string. The lookup is infallible
    /// by design — call sites are happy to fall through to the
    /// default when no policy is set.
    fn policy_for(&self, namespace: &str) -> RetentionPolicy;

    /// Set the policy for a namespace. Replaces any prior
    /// policy. Validates that the namespace is well-formed and
    /// that the policy is sane (in particular,
    /// `max_revisions_per_key = Some(0)` is rejected — use
    /// `None` to disable that bound).
    fn set_policy(
        &self,
        namespace: &str,
        policy: RetentionPolicy,
    ) -> Result<(), RevisionError>;
}

// === Section 5. In-memory reference =======================================

struct History {
    revisions: Vec<Revision>,
    next_id: u64,
}

impl History {
    fn new() -> Self {
        Self {
            revisions: Vec::new(),
            next_id: 1,
        }
    }
}

struct InMemoryInner {
    /// `(namespace, key) → History`. `BTreeMap` for deterministic
    /// listing order across replicas.
    histories: BTreeMap<(String, String), History>,
    /// Per-namespace retention policy. `None` ⇒ default.
    policies: HashMap<String, RetentionPolicy>,
}

/// Threadsafe in-memory `RevisionStore`. Suitable for unit tests
/// of upstream layers and for single-process tools whose history
/// doesn't need to outlive the process.
pub struct InMemoryRevisionStore {
    inner: Mutex<InMemoryInner>,
}

impl Default for InMemoryRevisionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryRevisionStore {
    /// Construct a fresh, empty store.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryInner {
                histories: BTreeMap::new(),
                policies: HashMap::new(),
            }),
        }
    }

    /// Apply `max_revisions_per_key` to a freshly-mutated
    /// history vector. Eviction is oldest-first.
    fn enforce_max(history: &mut History, retention: &RetentionPolicy) {
        if let Some(cap) = retention.max_revisions_per_key {
            while history.revisions.len() > cap {
                history.revisions.remove(0);
            }
        }
    }
}

impl RevisionStore for InMemoryRevisionStore {
    fn archive(
        &self,
        namespace: &str,
        key: &str,
        ciphertext: Vec<u8>,
        archived_at_ms: u64,
    ) -> Result<Revision, RevisionError> {
        validate_namespace_key(namespace, key)?;
        validate_ciphertext(&ciphertext)?;
        let mut g = lock_recover(&self.inner);
        let policy = g
            .policies
            .get(namespace)
            .copied()
            .unwrap_or_default();
        let history = g
            .histories
            .entry((namespace.to_string(), key.to_string()))
            .or_insert_with(History::new);
        let id = history.next_id;
        history.next_id = id
            .checked_add(1)
            .ok_or(RevisionError::Overflow)?;
        let rev = Revision {
            id,
            archived_at_ms,
            ciphertext,
        };
        history.revisions.push(rev.clone());
        Self::enforce_max(history, &policy);
        Ok(rev)
    }

    fn list(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Vec<RevisionMeta>, RevisionError> {
        validate_namespace_key(namespace, key)?;
        let g = lock_recover(&self.inner);
        let history = g
            .histories
            .get(&(namespace.to_string(), key.to_string()));
        let metas: Vec<RevisionMeta> = match history {
            None => Vec::new(),
            Some(h) => h
                .revisions
                .iter()
                .map(|r| RevisionMeta {
                    id: r.id,
                    archived_at_ms: r.archived_at_ms,
                    ciphertext_len: r.ciphertext.len(),
                })
                .collect(),
        };
        Ok(metas)
    }

    fn get_revision(
        &self,
        namespace: &str,
        key: &str,
        id: u64,
    ) -> Result<Revision, RevisionError> {
        validate_namespace_key(namespace, key)?;
        let g = lock_recover(&self.inner);
        let history = g
            .histories
            .get(&(namespace.to_string(), key.to_string()))
            .ok_or(RevisionError::NotFound)?;
        history
            .revisions
            .iter()
            .find(|r| r.id == id)
            .cloned()
            .ok_or(RevisionError::UnknownRevision)
    }

    fn restore(
        &self,
        namespace: &str,
        key: &str,
        id: u64,
        archived_at_ms: u64,
    ) -> Result<Revision, RevisionError> {
        // Fetch the historical ciphertext, then re-archive it as
        // a new revision under a fresh id. This is intentionally
        // *not* a rollback — the history list stays append-only.
        let old = self.get_revision(namespace, key, id)?;
        self.archive(namespace, key, old.ciphertext, archived_at_ms)
    }

    fn purge_due(
        &self,
        namespace: &str,
        retention: &RetentionPolicy,
        now_ms: u64,
    ) -> Result<usize, RevisionError> {
        // `purge_due` is a privileged maintenance call: `now_ms`
        // is treated as trusted input (the host sources it from a
        // monotonic clock, not from a remote caller). A
        // u64::MAX `now_ms` would correctly evict every row,
        // which is the right behaviour for an admin who genuinely
        // wants to purge everything.
        validate_namespace(namespace)?;
        let max_age = match retention.max_age_ms {
            None => return Ok(0),
            Some(a) => a,
        };
        // Anything strictly older than this cut-off is dropped.
        let cutoff = now_ms.saturating_sub(max_age);
        let mut g = lock_recover(&self.inner);
        let mut total_evicted = 0usize;
        for ((ns, _), history) in g.histories.iter_mut() {
            if ns != namespace {
                continue;
            }
            let before = history.revisions.len();
            history
                .revisions
                .retain(|r| r.archived_at_ms >= cutoff);
            total_evicted += before - history.revisions.len();
        }
        Ok(total_evicted)
    }

    fn policy_for(&self, namespace: &str) -> RetentionPolicy {
        // Infallible: an unknown (or invalid-shaped) namespace
        // simply falls through to the default policy. This is
        // safe because `set_policy` validates on the way in, so
        // `g.policies` never contains a malformed key.
        let g = lock_recover(&self.inner);
        g.policies.get(namespace).copied().unwrap_or_default()
    }

    fn set_policy(
        &self,
        namespace: &str,
        policy: RetentionPolicy,
    ) -> Result<(), RevisionError> {
        validate_namespace(namespace)?;
        validate_retention(&policy)?;
        let mut g = lock_recover(&self.inner);
        g.policies.insert(namespace.to_string(), policy);
        Ok(())
    }
}

// === Section 6. Tests =====================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> InMemoryRevisionStore {
        InMemoryRevisionStore::new()
    }

    // --- archive + list ---

    #[test]
    fn archive_returns_revision_id_starting_at_one() {
        let s = store();
        let r = s.archive("kv", "k", b"v1".to_vec(), 100).unwrap();
        assert_eq!(r.id, 1);
        assert_eq!(r.ciphertext, b"v1");
    }

    #[test]
    fn second_archive_increments_id() {
        let s = store();
        s.archive("kv", "k", b"v1".to_vec(), 100).unwrap();
        let r2 = s.archive("kv", "k", b"v2".to_vec(), 200).unwrap();
        assert_eq!(r2.id, 2);
    }

    #[test]
    fn list_returns_metadata_only() {
        let s = store();
        s.archive("kv", "k", b"v1".to_vec(), 100).unwrap();
        s.archive("kv", "k", b"v2-longer".to_vec(), 200).unwrap();
        let metas = s.list("kv", "k").unwrap();
        assert_eq!(metas.len(), 2);
        assert_eq!(metas[0].id, 1);
        assert_eq!(metas[0].archived_at_ms, 100);
        assert_eq!(metas[0].ciphertext_len, 2);
        assert_eq!(metas[1].id, 2);
        assert_eq!(metas[1].ciphertext_len, 9);
    }

    #[test]
    fn list_unknown_path_returns_empty() {
        let s = store();
        assert!(s.list("kv", "missing").unwrap().is_empty());
    }

    // --- get_revision ---

    #[test]
    fn get_revision_returns_ciphertext() {
        let s = store();
        let r = s.archive("kv", "k", b"v1".to_vec(), 100).unwrap();
        let g = s.get_revision("kv", "k", r.id).unwrap();
        assert_eq!(g.ciphertext, b"v1");
    }

    #[test]
    fn get_revision_unknown_id_returns_unknown_revision() {
        let s = store();
        s.archive("kv", "k", b"v1".to_vec(), 100).unwrap();
        let r = s.get_revision("kv", "k", 999);
        assert!(matches!(r, Err(RevisionError::UnknownRevision)));
    }

    #[test]
    fn get_revision_unknown_path_returns_not_found() {
        let s = store();
        let r = s.get_revision("kv", "missing", 1);
        assert!(matches!(r, Err(RevisionError::NotFound)));
    }

    // --- restore ---

    #[test]
    fn restore_appends_new_revision() {
        let s = store();
        s.archive("kv", "k", b"old".to_vec(), 100).unwrap();
        s.archive("kv", "k", b"new".to_vec(), 200).unwrap();
        let restored = s.restore("kv", "k", 1, 300).unwrap();
        assert_eq!(restored.id, 3);
        assert_eq!(restored.ciphertext, b"old");
        // Original is still there.
        assert_eq!(s.get_revision("kv", "k", 1).unwrap().ciphertext, b"old");
        // Latest history now has three rows.
        assert_eq!(s.list("kv", "k").unwrap().len(), 3);
    }

    #[test]
    fn restore_unknown_revision_errors() {
        let s = store();
        s.archive("kv", "k", b"v1".to_vec(), 100).unwrap();
        let r = s.restore("kv", "k", 99, 200);
        assert!(matches!(r, Err(RevisionError::UnknownRevision)));
    }

    // --- retention policy ---

    #[test]
    fn max_revisions_evicts_oldest_first() {
        let s = store();
        s.set_policy(
            "kv",
            RetentionPolicy {
                max_revisions_per_key: Some(3),
                max_age_ms: None,
            },
        ).unwrap();
        for i in 0..5u32 {
            s.archive("kv", "k", format!("v{}", i).into_bytes(), 100 + i as u64)
                .unwrap();
        }
        let metas = s.list("kv", "k").unwrap();
        // Only the last 3 should remain.
        assert_eq!(metas.len(), 3);
        assert_eq!(metas[0].id, 3);
        assert_eq!(metas[1].id, 4);
        assert_eq!(metas[2].id, 5);
        // Oldest are gone.
        assert!(matches!(
            s.get_revision("kv", "k", 1),
            Err(RevisionError::UnknownRevision)
        ));
    }

    #[test]
    fn purge_due_evicts_old_rows() {
        let s = store();
        s.archive("kv", "k", b"v1".to_vec(), 100).unwrap();
        s.archive("kv", "k", b"v2".to_vec(), 1_000).unwrap();
        s.archive("kv", "k", b"v3".to_vec(), 10_000).unwrap();
        let policy = RetentionPolicy {
            max_revisions_per_key: None,
            max_age_ms: Some(5_000),
        };
        // now=10_000, cutoff=5_000, so revisions at 100 and 1_000 go.
        let evicted = s.purge_due("kv", &policy, 10_000).unwrap();
        assert_eq!(evicted, 2);
        let metas = s.list("kv", "k").unwrap();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].id, 3);
    }

    #[test]
    fn purge_due_with_no_age_cap_is_noop() {
        let s = store();
        s.archive("kv", "k", b"v1".to_vec(), 100).unwrap();
        let policy = RetentionPolicy {
            max_revisions_per_key: None,
            max_age_ms: None,
        };
        let n = s.purge_due("kv", &policy, 999_999).unwrap();
        assert_eq!(n, 0);
        assert_eq!(s.list("kv", "k").unwrap().len(), 1);
    }

    #[test]
    fn purge_due_only_affects_target_namespace() {
        let s = store();
        s.archive("kv1", "k", b"a".to_vec(), 100).unwrap();
        s.archive("kv2", "k", b"b".to_vec(), 100).unwrap();
        let policy = RetentionPolicy {
            max_revisions_per_key: None,
            max_age_ms: Some(50),
        };
        let n = s.purge_due("kv1", &policy, 10_000).unwrap();
        assert_eq!(n, 1);
        // kv2 untouched.
        assert_eq!(s.list("kv2", "k").unwrap().len(), 1);
    }

    #[test]
    fn policy_default_caps_after_archive() {
        // The archive path uses the per-namespace policy; if
        // none is set, it uses RetentionPolicy::default() which
        // is the password-manager default (32 revisions).
        let s = store();
        for i in 0..40u32 {
            s.archive("kv", "k", format!("v{}", i).into_bytes(), 100 + i as u64)
                .unwrap();
        }
        // Capped at 32.
        assert_eq!(s.list("kv", "k").unwrap().len(), 32);
    }

    #[test]
    fn unbounded_policy_keeps_everything() {
        let s = store();
        s.set_policy("kv", RetentionPolicy::unbounded()).unwrap();
        for i in 0..40u32 {
            s.archive("kv", "k", format!("v{}", i).into_bytes(), 100 + i as u64)
                .unwrap();
        }
        assert_eq!(s.list("kv", "k").unwrap().len(), 40);
    }

    // --- validation ---

    #[test]
    fn rejects_empty_namespace() {
        let s = store();
        let r = s.archive("", "k", b"v".to_vec(), 100);
        assert!(matches!(r, Err(RevisionError::InvalidParameter(_))));
    }

    #[test]
    fn rejects_oversize_namespace() {
        let s = store();
        let r = s.archive(&"x".repeat(MAX_NAMESPACE_LEN + 1), "k", b"v".to_vec(), 100);
        assert!(matches!(r, Err(RevisionError::InvalidParameter(_))));
    }

    #[test]
    fn rejects_namespace_with_control_chars() {
        let s = store();
        let r = s.archive("kv\nadmin", "k", b"v".to_vec(), 100);
        assert!(matches!(r, Err(RevisionError::InvalidParameter(_))));
    }

    #[test]
    fn rejects_key_with_bidi_override() {
        // U+202E — Trojan Source attack.
        let s = store();
        let r = s.archive("kv", "admin\u{202e}txt", b"v".to_vec(), 100);
        assert!(matches!(r, Err(RevisionError::InvalidParameter(_))));
    }

    #[test]
    fn rejects_empty_key() {
        let s = store();
        let r = s.archive("kv", "", b"v".to_vec(), 100);
        assert!(matches!(r, Err(RevisionError::InvalidParameter(_))));
    }

    #[test]
    fn rejects_oversize_ciphertext() {
        let s = store();
        let r = s.archive("kv", "k", vec![0u8; MAX_CIPHERTEXT_LEN + 1], 100);
        assert!(matches!(r, Err(RevisionError::InvalidParameter(_))));
    }

    #[test]
    fn rejects_empty_ciphertext() {
        let s = store();
        let r = s.archive("kv", "k", vec![], 100);
        assert!(matches!(r, Err(RevisionError::InvalidParameter(_))));
    }

    // --- redaction ---

    #[test]
    fn revision_debug_redacts_ciphertext() {
        let r = Revision {
            id: 5,
            archived_at_ms: 1_000,
            ciphertext: b"super-secret".to_vec(),
        };
        let s = format!("{:?}", r);
        assert!(!s.contains("super-secret"));
        assert!(s.contains("12 bytes redacted"));
    }

    // --- threading ---

    #[test]
    fn store_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InMemoryRevisionStore>();
        assert_send_sync::<Box<dyn RevisionStore>>();
    }

    #[test]
    fn concurrent_archives_all_get_unique_ids() {
        // 16 threads each archive 4 revisions to the same key.
        // After the dust settles we expect 64 revisions with
        // ids 1..=64.
        use std::sync::Arc;
        use std::thread;
        let s = Arc::new({
            let s = InMemoryRevisionStore::new();
            s.set_policy("kv", RetentionPolicy::unbounded()).unwrap();
            s
        });
        let mut handles = Vec::new();
        for tid in 0..16u32 {
            let s = s.clone();
            handles.push(thread::spawn(move || {
                for i in 0..4u32 {
                    s.archive(
                        "kv",
                        "k",
                        format!("v-{}-{}", tid, i).into_bytes(),
                        1_000 + (tid as u64) * 100 + (i as u64),
                    )
                    .unwrap();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let metas = s.list("kv", "k").unwrap();
        assert_eq!(metas.len(), 64);
        // ids are dense 1..=64
        for (i, m) in metas.iter().enumerate() {
            assert_eq!(m.id, (i as u64) + 1);
        }
    }

    // --- policy round-trip ---

    #[test]
    fn set_policy_round_trips() {
        let s = store();
        let p = RetentionPolicy {
            max_revisions_per_key: Some(7),
            max_age_ms: Some(42),
        };
        s.set_policy("kv", p).unwrap();
        assert_eq!(s.policy_for("kv"), p);
    }

    #[test]
    fn set_policy_rejects_zero_max_revisions() {
        let s = store();
        let p = RetentionPolicy {
            max_revisions_per_key: Some(0),
            max_age_ms: None,
        };
        let r = s.set_policy("kv", p);
        assert!(matches!(r, Err(RevisionError::InvalidParameter(_))));
    }

    #[test]
    fn set_policy_rejects_invalid_namespace() {
        let s = store();
        let p = RetentionPolicy::default();
        // Empty namespace.
        assert!(matches!(s.set_policy("", p), Err(RevisionError::InvalidParameter(_))));
        // Control characters.
        assert!(matches!(s.set_policy("kv\nadmin", p), Err(RevisionError::InvalidParameter(_))));
        // Bidi-override.
        assert!(matches!(s.set_policy("kv\u{202e}", p), Err(RevisionError::InvalidParameter(_))));
        // Oversize.
        let big = "x".repeat(MAX_NAMESPACE_LEN + 1);
        assert!(matches!(s.set_policy(&big, p), Err(RevisionError::InvalidParameter(_))));
    }

    #[test]
    fn purge_due_rejects_oversize_namespace() {
        let s = store();
        let p = RetentionPolicy {
            max_revisions_per_key: None,
            max_age_ms: Some(1_000),
        };
        let big = "x".repeat(MAX_NAMESPACE_LEN + 1);
        let r = s.purge_due(&big, &p, 100_000);
        assert!(matches!(r, Err(RevisionError::InvalidParameter(_))));
    }

    #[test]
    fn policy_for_unknown_namespace_returns_default() {
        let s = store();
        assert_eq!(s.policy_for("never-set"), RetentionPolicy::default());
    }
}
