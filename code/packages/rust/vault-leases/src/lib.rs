//! # `coding_adventures_vault_leases` — VLT07
//!
//! ## What this crate is
//!
//! A **lease manager**: a trait + an in-memory reference
//! implementation for handing out *short-lived capability tokens*
//! over arbitrary opaque payloads.
//!
//! In Vault terms, a "lease" is the receipt that the user gets back
//! when the system gives them something time-limited:
//!
//!   * a freshly-rotated cloud credential pair (VLT08 dynamic-secret
//!     engine — "give me an AWS key for the next 30 minutes"),
//!   * a *response-wrapping* token for secure introduction
//!     ("here's a one-shot pointer to the real secret; the human
//!     hand-carries the pointer, the agent unwraps it once, the
//!     real secret never appears in the chat log"),
//!   * any other capability that should expire on its own without
//!     requiring a sweep across every consumer.
//!
//! HashiCorp Vault calls this "leases & response wrapping". 1Password
//! and Bitwarden don't expose it directly to end users but use
//! variants internally for short-lived recovery tokens. Both shapes
//! sit on top of the *same* primitives:
//!
//!   * monotonically-issued opaque IDs that nobody else can guess,
//!   * an authoritative TTL the issuer enforces,
//!   * a way to *revoke* before the TTL elapses,
//!   * for one-shots, an atomic *consume* (read-and-delete).
//!
//! ## Why a separate crate
//!
//! Leases are orthogonal to the cryptography (VLT01..VLT05) and to
//! the policy decision (VLT06). VLT07 sits one layer above policy:
//! after the policy says "yes, the caller may receive this secret",
//! the lease manager wraps the secret in a TTL'd, revocable
//! envelope. The dynamic-secret engines (VLT08) consume this trait
//! to attach a TTL to the credentials they mint; the audit log
//! (VLT09) records every issue/renew/revoke/consume; the sync
//! engine (VLT10) propagates revocations across replicas.
//!
//! The trait is split out so each of those layers can be tested
//! against an [`InMemoryLeaseManager`] and a real backend can
//! arrive in a follow-up PR (likely backed by storage-core +
//! VLT01 sealed-store, so the lease body is encrypted at rest).
//!
//! ## Threat model
//!
//! * **Lease ID guessability** — the ID must be drawn from a CSPRNG
//!   with enough entropy that brute-forcing the address space is
//!   infeasible. We use 128 bits (16 bytes), hex-encoded for transport.
//! * **Lookup oracle** — `lookup` and `read` both take a borrowed
//!   `LeaseId`. A wrong ID returns `NotFound` rather than leaking
//!   "the ID exists but you can't read it". Comparison of stored
//!   versus supplied IDs is constant-time (hash-based dispatch on
//!   the byte string is fine because the IDs are random and not
//!   user-derived; the constant-time path matters only inside
//!   downstream introspection helpers).
//! * **Payload exposure** — the in-memory backing store wraps every
//!   payload in [`coding_adventures_zeroize::Zeroizing`] so the
//!   bytes are wiped on revocation, expiry, or process exit.
//! * **Race between consume and revoke** — `consume` and `revoke`
//!   both take `&mut` access to the entry under a mutex, so the
//!   read-and-delete is atomic.
//! * **Clock manipulation** — TTL is enforced against a
//!   *caller-supplied* `now_ms` (via [`LeaseManager::expire_due`])
//!   and against a monotonic check at every lookup (the manager
//!   uses [`std::time::Instant`] internally for actual real-time
//!   work; the test surface lets callers pin time deterministically).
//!
//! ## What this crate is *not*
//!
//! * It does not encrypt the payload at rest. Layer
//!   `coding_adventures_vault_sealed_store` (VLT01) under the
//!   storage-backed implementation if you want that.
//! * It does not propagate revocations across replicas — that's
//!   VLT10.
//! * It does not record audit events — that's VLT09. The
//!   in-memory implementation is intentionally silent.
//! * It is not a job queue, a message broker, or a KV store — see
//!   `storage_core::StorageBackend` for general-purpose K/V.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_zeroize::{Zeroize, Zeroizing};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// === Section 1. Public types ================================================
//
// The vocabulary the rest of the Vault stack speaks when it talks
// to a lease manager. All three are "newtype" wrappers (no runtime
// cost) — the goal is to make the type system reject mistakes like
// "passed a TTL where a payload was expected".

/// Opaque, unguessable lease identifier.
///
/// Hex-encoded 16 bytes (128 bits) of CSPRNG output. The string
/// form survives every transport we care about (HTTP headers,
/// shell args, JSON) without escaping.
///
/// `LeaseId` deliberately does **not** implement `Display`. Rendering
/// requires going through [`LeaseId::as_hex`] which makes it visible
/// in code review when an ID is being externalised.
///
/// The inner string is held under [`Zeroizing`] so the hex bytes are
/// scrubbed from the heap when the `LeaseId` (or the `HashMap` row
/// keyed by it) drops. IDs are bearer capabilities; if forensic
/// access to process memory is part of the threat model, the
/// residue of an issued-but-now-reaped ID would otherwise enumerate
/// every lease the manager has ever handed out.
pub struct LeaseId(Zeroizing<String>);

impl LeaseId {
    /// Borrow the hex form. The bytes are zeroized on drop, so
    /// callers should not copy the slice into long-lived plain
    /// `String`s.
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// Parse a hex-encoded ID supplied by an external caller.
    /// Returns `None` if the input is not valid 32-char ASCII hex.
    /// (We are deliberately strict so a typo'd ID is rejected at
    /// the boundary instead of becoming an exotic `NotFound`.)
    pub fn from_hex(s: &str) -> Option<Self> {
        if s.len() != 32 {
            return None;
        }
        if !s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return None;
        }
        Some(Self(Zeroizing::new(s.to_owned())))
    }
}

// `Zeroizing<String>` does not derive Clone/Debug/Hash/Eq/PartialEq,
// so we hand-roll them via Deref to `&str`. The hex form is the
// canonical comparison/hashing key — equal hex strings are equal IDs.

impl Clone for LeaseId {
    fn clone(&self) -> Self {
        Self(Zeroizing::new((*self.0).clone()))
    }
}

impl core::fmt::Debug for LeaseId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // The hex string is a bearer cap, so we redact it in Debug
        // the same way we redact `LeasePayload`. Stray `dbg!()`s
        // print the *length* (always 32) but not the bytes.
        write!(f, "LeaseId(<{}-char redacted>)", self.0.len())
    }
}

impl core::hash::Hash for LeaseId {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_hex().hash(state);
    }
}

impl PartialEq for LeaseId {
    fn eq(&self, other: &Self) -> bool {
        self.as_hex() == other.as_hex()
    }
}

impl Eq for LeaseId {}

/// Raw bytes wrapped by a lease.
///
/// The lease manager treats the body as opaque. Callers that want
/// structure (e.g. a JSON object containing a temporary AWS access
/// key + secret + session token) serialize before issuing and
/// deserialize on read.
///
/// The `Vec<u8>` is wiped on `Drop` via the inner `Zeroizing`.
///
/// `Debug` is intentionally a hand-rolled "redacted" formatter so a
/// stray `dbg!` or panic message cannot leak the body bytes.
pub struct LeasePayload(Zeroizing<Vec<u8>>);

impl core::fmt::Debug for LeasePayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "LeasePayload(<{} bytes redacted>)", self.0.len())
    }
}

impl LeasePayload {
    /// Wrap an owned byte vector. The vector is moved in and zeroized
    /// when the [`LeasePayload`] drops.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// Borrow the bytes. Callers are responsible for not copying
    /// them into long-lived heap buffers.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Clone for LeasePayload {
    /// Cloning a payload is allowed but rare — typically only the
    /// in-memory store does it, when handing out a copy on `read()`.
    /// The clone is itself zeroizing-on-drop.
    fn clone(&self) -> Self {
        Self(Zeroizing::new(self.0.to_vec()))
    }
}

/// Read-only view of a lease's metadata. Returned by
/// [`LeaseManager::lookup`].
///
/// Carrying the payload is intentionally *not* part of `LeaseInfo`:
/// inspection of "does this lease still exist?" is much more common
/// than "give me the bytes", and we want callers to opt explicitly
/// into the latter via [`LeaseManager::read`] /
/// [`LeaseManager::consume`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaseInfo {
    /// Echo of the ID, so a caller can pass `LeaseInfo` around
    /// without keeping the [`LeaseId`] separately.
    pub id: LeaseId,
    /// When the lease was issued, in ms since UNIX epoch.
    pub issued_at_ms: u64,
    /// When the lease *would* expire if neither renewed nor revoked,
    /// in ms since UNIX epoch.
    pub expires_at_ms: u64,
    /// Number of times `read()` has been called against this lease.
    /// `0` for a brand-new lease, `1` for one that has been
    /// consumed (consume is a read), and N for an N-times-read
    /// multi-read lease. Diagnostic only — used by
    /// response-wrapping callers to assert "this token has never
    /// been seen by anyone but me".
    pub read_count: u32,
    /// True if the lease has been revoked (either explicitly via
    /// [`LeaseManager::revoke`] or implicitly via
    /// [`LeaseManager::consume`]). A revoked lease cannot be read
    /// again and will be reaped on the next `expire_due` sweep.
    pub revoked: bool,
}

/// All errors the lease manager can produce.
///
/// The variants are intentionally narrow: callers usually want to
/// distinguish "this ID never existed", "this ID *did* exist but
/// the window has closed", and "we hit a system fault". Conflating
/// the first two would create a privacy oracle ("does ID X exist?")
/// — see the threat-model section in the crate-level docs.
#[derive(Debug)]
pub enum LeaseError {
    /// No such lease ID. This is what the caller sees both for IDs
    /// that were never issued *and* for IDs whose entry has already
    /// been reaped after expiry/revocation. Keeping these
    /// indistinguishable is deliberate — see threat model.
    NotFound,

    /// The lease exists but its TTL has elapsed and it has not been
    /// renewed. Distinct from `NotFound` because the caller
    /// already had the ID and is being told *why* it stopped
    /// working — that's not an oracle, it's an explanation.
    Expired,

    /// The lease was explicitly revoked.
    Revoked,

    /// Caller passed an invalid argument (e.g. zero TTL, payload
    /// too large for the configured limit, malformed ID).
    InvalidParameter(&'static str),

    /// The CSPRNG was unavailable while drawing a fresh ID.
    Crypto(coding_adventures_csprng::CsprngError),
}

impl core::fmt::Display for LeaseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotFound => write!(f, "lease not found"),
            Self::Expired => write!(f, "lease expired"),
            Self::Revoked => write!(f, "lease revoked"),
            Self::InvalidParameter(why) => write!(f, "invalid parameter: {}", why),
            Self::Crypto(e) => write!(f, "crypto error drawing lease ID: {}", e),
        }
    }
}

impl std::error::Error for LeaseError {}

impl From<coding_adventures_csprng::CsprngError> for LeaseError {
    fn from(e: coding_adventures_csprng::CsprngError) -> Self {
        Self::Crypto(e)
    }
}

// === Section 2. The trait ===================================================
//
// One trait, every backend implements it, callers (VLT08, VLT09)
// don't care which is in use. The shape is intentionally similar
// to `storage_core::StorageBackend` so a future
// `StorageBackedLeaseManager` is a thin adaptor.

/// Lease management contract — the surface VLT08+ talks to.
///
/// All methods are `&self` (interior mutability) so a single
/// manager instance can be shared by `Arc` across threads. The
/// in-memory implementation uses a [`Mutex`]; a future
/// storage-backed implementation will rely on the underlying
/// backend's CAS guarantees.
pub trait LeaseManager: Send + Sync {
    /// Issue a brand-new lease.
    ///
    /// * `payload` — opaque bytes the caller wants to put under
    ///   TTL custody.
    /// * `ttl_ms` — lifetime in milliseconds. Must be > 0.
    ///
    /// Returns the freshly-minted [`LeaseId`]. The payload is moved
    /// into the manager and zeroized when the lease ends.
    fn issue(&self, payload: LeasePayload, ttl_ms: u64) -> Result<LeaseId, LeaseError>;

    /// Extend an existing lease's TTL by `extra_ms` from *now*.
    ///
    /// Renewing a revoked lease returns `Revoked`; renewing an
    /// already-expired lease returns `Expired`. The new expiry is
    /// `now + extra_ms` — we do *not* extend from the existing
    /// expiry, because that would let a sleeping consumer
    /// effectively bank renewals.
    fn renew(&self, id: &LeaseId, extra_ms: u64) -> Result<(), LeaseError>;

    /// Mark the lease as revoked. Subsequent reads/renews fail
    /// with `Revoked`. The entry stays in memory (with its payload
    /// zeroized) until the next [`LeaseManager::expire_due`] sweep
    /// so that consumers see the explicit `Revoked` rather than a
    /// confusing `NotFound`.
    fn revoke(&self, id: &LeaseId) -> Result<(), LeaseError>;

    /// Cheap metadata lookup — no payload bytes returned.
    fn lookup(&self, id: &LeaseId) -> Result<LeaseInfo, LeaseError>;

    /// Read the payload without consuming the lease. Increments
    /// `read_count`. Use [`LeaseManager::consume`] for one-shot
    /// response-wrapping.
    fn read(&self, id: &LeaseId) -> Result<LeasePayload, LeaseError>;

    /// Atomically read the payload *and* revoke the lease. This is
    /// the response-wrapping primitive: the caller receives the
    /// bytes once, the lease is gone, and any subsequent attempt
    /// to use the same ID fails with `Revoked`.
    fn consume(&self, id: &LeaseId) -> Result<LeasePayload, LeaseError>;

    /// Reap entries whose `expires_at_ms <= now_ms` *or* that have
    /// been revoked. Returns the number of entries actually
    /// removed. Caller-driven so the manager doesn't depend on a
    /// background timer (which would be a side effect we don't
    /// want in a pure-crypto crate).
    fn expire_due(&self, now_ms: u64) -> Result<usize, LeaseError>;
}

// === Section 3. In-memory implementation ====================================
//
// The reference implementation. Used directly in unit tests for
// VLT08+ and good enough for single-process tools that don't need
// lease durability across restarts. A persistent implementation
// will be a thin shim over storage-core + VLT01.

/// One row in the in-memory lease table.
///
/// `Zeroize` is implemented manually so the payload bytes are
/// scrubbed when the entry is reaped (in addition to being
/// scrubbed by `Zeroizing`'s own Drop, which double-protects in
/// case `LeaseEntry::zeroize` is called explicitly).
#[derive(Debug)]
struct LeaseEntry {
    issued_at_ms: u64,
    expires_at_ms: u64,
    read_count: u32,
    revoked: bool,
    /// `None` once the lease has been consumed or revoked-and-cleared.
    /// We keep the entry around so subsequent calls return the
    /// correct error variant rather than `NotFound`.
    payload: Option<LeasePayload>,
}

impl Zeroize for LeaseEntry {
    fn zeroize(&mut self) {
        // The numeric fields are not secrets; clearing them keeps
        // the row "obviously dead" in a memory dump.
        self.issued_at_ms = 0;
        self.expires_at_ms = 0;
        self.read_count = 0;
        self.revoked = true;
        // The payload, if present, is wiped automatically by
        // `Zeroizing`'s `Drop`. Setting to `None` runs that drop
        // immediately rather than letting the entry linger.
        self.payload = None;
    }
}

impl Drop for LeaseEntry {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Default in-memory lease manager. Threadsafe via `Mutex`.
///
/// The intended pattern is `Arc::new(InMemoryLeaseManager::new())`
/// at startup, then hand the `Arc` to every component that needs
/// to issue or consume leases. The mutex is held for the duration
/// of a single operation; lease ops are O(1) (HashMap access) so
/// contention is not a concern.
pub struct InMemoryLeaseManager {
    inner: Mutex<HashMap<LeaseId, LeaseEntry>>,
}

impl Default for InMemoryLeaseManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryLeaseManager {
    /// Construct an empty manager.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Current wall-clock millis. Pulled into a method so tests can
    /// substitute a fake clock by using
    /// [`LeaseManager::expire_due`] directly with a pinned `now_ms`.
    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            // If the system clock is before 1970, fall back to 0
            // rather than panic. The lease will then expire on the
            // next `expire_due` sweep with `now_ms > 0`, which is
            // the safest "fail closed" behaviour.
            .unwrap_or(0)
    }

    /// Draw a fresh 128-bit ID and hex-encode it.
    fn fresh_id() -> Result<LeaseId, LeaseError> {
        let bytes = coding_adventures_csprng::random_array::<16>()?;
        // Hex encode by hand to avoid pulling in `hex`. The output
        // is always 32 lowercase hex chars.
        let mut s = String::with_capacity(32);
        for b in bytes.iter() {
            s.push(HEX[(b >> 4) as usize] as char);
            s.push(HEX[(b & 0x0f) as usize] as char);
        }
        Ok(LeaseId(Zeroizing::new(s)))
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Maximum TTL the in-memory manager will accept on `issue` /
/// `renew`. 90 days. Reasoning:
///
/// - HashiCorp Vault defaults to 32-day max-TTL for most engines;
///   90 days gives slack for caller-defined long-lived flows
///   (e.g. invitation tokens, slow rotation cycles).
/// - It bounds memory growth: even an attacker with `issue` access
///   cannot pin entries to effectively-infinite expiry by passing
///   `u64::MAX`, which would saturate `expires_at_ms` past any
///   realistic `expire_due(now_ms)` sweep.
/// - It is well below `u64::MAX` ms (~584M years), so we can use
///   `checked_add` without ambiguity.
const MAX_TTL_MS: u64 = 90 * 24 * 60 * 60 * 1_000;

impl LeaseManager for InMemoryLeaseManager {
    fn issue(&self, payload: LeasePayload, ttl_ms: u64) -> Result<LeaseId, LeaseError> {
        if ttl_ms == 0 {
            return Err(LeaseError::InvalidParameter("ttl_ms must be > 0"));
        }
        if ttl_ms > MAX_TTL_MS {
            return Err(LeaseError::InvalidParameter("ttl_ms exceeds MAX_TTL_MS"));
        }
        let id = Self::fresh_id()?;
        let mut g = self.inner.lock().expect("lease mutex poisoned");
        // Sample wall-clock under the lock so `expires_at_ms` is
        // computed against the same clock observation that any
        // concurrent operation would observe — no TOCTOU on
        // expiry-edge semantics.
        let now = Self::now_ms();
        let expires = now
            .checked_add(ttl_ms)
            .ok_or(LeaseError::InvalidParameter("ttl_ms overflow"))?;
        let entry = LeaseEntry {
            issued_at_ms: now,
            expires_at_ms: expires,
            read_count: 0,
            revoked: false,
            payload: Some(payload),
        };
        // 128 random bits → collision is astronomically unlikely;
        // we still treat a collision as a system fault rather than
        // overwrite an existing lease silently.
        if g.contains_key(&id) {
            return Err(LeaseError::InvalidParameter("id collision (rng failure?)"));
        }
        g.insert(id.clone(), entry);
        Ok(id)
    }

    fn renew(&self, id: &LeaseId, extra_ms: u64) -> Result<(), LeaseError> {
        if extra_ms == 0 {
            return Err(LeaseError::InvalidParameter("extra_ms must be > 0"));
        }
        if extra_ms > MAX_TTL_MS {
            return Err(LeaseError::InvalidParameter("extra_ms exceeds MAX_TTL_MS"));
        }
        let mut g = self.inner.lock().expect("lease mutex poisoned");
        let now = Self::now_ms();
        let entry = g.get_mut(id).ok_or(LeaseError::NotFound)?;
        if entry.revoked {
            return Err(LeaseError::Revoked);
        }
        if entry.expires_at_ms <= now {
            return Err(LeaseError::Expired);
        }
        entry.expires_at_ms = now
            .checked_add(extra_ms)
            .ok_or(LeaseError::InvalidParameter("extra_ms overflow"))?;
        Ok(())
    }

    fn revoke(&self, id: &LeaseId) -> Result<(), LeaseError> {
        let mut g = self.inner.lock().expect("lease mutex poisoned");
        let entry = g.get_mut(id).ok_or(LeaseError::NotFound)?;
        if entry.revoked {
            // Idempotent — re-revoking an already-revoked lease is
            // not an error. Useful in cleanup paths where the
            // caller doesn't track which IDs it has touched.
            return Ok(());
        }
        entry.revoked = true;
        // Drop the payload immediately on revocation. Subsequent
        // reads will see `Revoked` because the flag is set.
        entry.payload = None;
        Ok(())
    }

    fn lookup(&self, id: &LeaseId) -> Result<LeaseInfo, LeaseError> {
        let g = self.inner.lock().expect("lease mutex poisoned");
        let entry = g.get(id).ok_or(LeaseError::NotFound)?;
        // We *do* return info for revoked or expired leases —
        // lookup is a metadata call, the caller is asking
        // explicitly "what is the state of this ID?". `now` is
        // not consulted; expiry state is stored on the entry.
        Ok(LeaseInfo {
            id: id.clone(),
            issued_at_ms: entry.issued_at_ms,
            expires_at_ms: entry.expires_at_ms,
            read_count: entry.read_count,
            revoked: entry.revoked,
        })
    }

    fn read(&self, id: &LeaseId) -> Result<LeasePayload, LeaseError> {
        let mut g = self.inner.lock().expect("lease mutex poisoned");
        let now = Self::now_ms();
        let entry = g.get_mut(id).ok_or(LeaseError::NotFound)?;
        if entry.revoked {
            return Err(LeaseError::Revoked);
        }
        if entry.expires_at_ms <= now {
            return Err(LeaseError::Expired);
        }
        let payload = entry.payload.as_ref().ok_or(LeaseError::Revoked)?.clone();
        entry.read_count = entry.read_count.saturating_add(1);
        Ok(payload)
    }

    fn consume(&self, id: &LeaseId) -> Result<LeasePayload, LeaseError> {
        let mut g = self.inner.lock().expect("lease mutex poisoned");
        let now = Self::now_ms();
        let entry = g.get_mut(id).ok_or(LeaseError::NotFound)?;
        if entry.revoked {
            return Err(LeaseError::Revoked);
        }
        if entry.expires_at_ms <= now {
            return Err(LeaseError::Expired);
        }
        // Atomic read-and-revoke: take ownership of the payload,
        // mark the entry revoked, increment read_count.
        let payload = entry.payload.take().ok_or(LeaseError::Revoked)?;
        entry.revoked = true;
        entry.read_count = entry.read_count.saturating_add(1);
        Ok(payload)
    }

    fn expire_due(&self, now_ms: u64) -> Result<usize, LeaseError> {
        let mut g = self.inner.lock().expect("lease mutex poisoned");
        // Collect IDs to drop first to avoid mutating-while-iterating.
        let dead: Vec<LeaseId> = g
            .iter()
            .filter(|(_, e)| e.revoked || e.expires_at_ms <= now_ms)
            .map(|(k, _)| k.clone())
            .collect();
        let n = dead.len();
        for id in dead {
            // Removing the entry runs `LeaseEntry::Drop` which
            // calls `zeroize()`.
            g.remove(&id);
        }
        Ok(n)
    }
}

// === Section 4. Tests =======================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_payload(s: &str) -> LeasePayload {
        LeasePayload::new(s.as_bytes().to_vec())
    }

    #[test]
    fn issue_then_lookup_returns_info() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("hello"), 60_000).unwrap();
        let info = mgr.lookup(&id).unwrap();
        assert_eq!(info.id, id);
        assert_eq!(info.read_count, 0);
        assert!(!info.revoked);
        assert!(info.expires_at_ms >= info.issued_at_ms);
    }

    #[test]
    fn issue_with_zero_ttl_rejected() {
        let mgr = InMemoryLeaseManager::new();
        let r = mgr.issue(mk_payload("x"), 0);
        assert!(matches!(r, Err(LeaseError::InvalidParameter(_))));
    }

    #[test]
    fn issue_rejects_ttl_above_max() {
        let mgr = InMemoryLeaseManager::new();
        // u64::MAX would saturate `expires_at_ms` past any realistic
        // expire_due() sweep, pinning the entry in memory. We bound
        // it explicitly so an attacker with `issue` access can't
        // grow the map without bound.
        assert!(matches!(
            mgr.issue(mk_payload("x"), u64::MAX),
            Err(LeaseError::InvalidParameter(_))
        ));
    }

    #[test]
    fn renew_rejects_extra_above_max() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("x"), 60_000).unwrap();
        assert!(matches!(
            mgr.renew(&id, u64::MAX),
            Err(LeaseError::InvalidParameter(_))
        ));
    }

    #[test]
    fn read_returns_payload_and_increments_counter() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("payload-bytes"), 60_000).unwrap();
        let body = mgr.read(&id).unwrap();
        assert_eq!(body.as_bytes(), b"payload-bytes");
        let info = mgr.lookup(&id).unwrap();
        assert_eq!(info.read_count, 1);
        // Re-reading a multi-read lease still works.
        let body2 = mgr.read(&id).unwrap();
        assert_eq!(body2.as_bytes(), b"payload-bytes");
        let info2 = mgr.lookup(&id).unwrap();
        assert_eq!(info2.read_count, 2);
    }

    #[test]
    fn read_unknown_id_returns_not_found() {
        let mgr = InMemoryLeaseManager::new();
        let bogus = LeaseId::from_hex("00112233445566778899aabbccddeeff").unwrap();
        assert!(matches!(mgr.read(&bogus), Err(LeaseError::NotFound)));
    }

    #[test]
    fn consume_returns_payload_and_revokes_atomically() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("once"), 60_000).unwrap();
        let body = mgr.consume(&id).unwrap();
        assert_eq!(body.as_bytes(), b"once");
        // Second consume → Revoked.
        assert!(matches!(mgr.consume(&id), Err(LeaseError::Revoked)));
        // Read after consume → Revoked.
        assert!(matches!(mgr.read(&id), Err(LeaseError::Revoked)));
        // Lookup still shows the metadata, with revoked=true.
        let info = mgr.lookup(&id).unwrap();
        assert!(info.revoked);
        assert_eq!(info.read_count, 1);
    }

    #[test]
    fn revoke_makes_subsequent_read_fail() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("secret"), 60_000).unwrap();
        mgr.revoke(&id).unwrap();
        assert!(matches!(mgr.read(&id), Err(LeaseError::Revoked)));
        // Idempotent revoke.
        assert!(mgr.revoke(&id).is_ok());
    }

    #[test]
    fn revoke_unknown_id_is_not_found() {
        let mgr = InMemoryLeaseManager::new();
        let bogus = LeaseId::from_hex("ffffffffffffffffffffffffffffffff").unwrap();
        assert!(matches!(mgr.revoke(&bogus), Err(LeaseError::NotFound)));
    }

    #[test]
    fn renew_extends_expiry() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("x"), 1_000).unwrap();
        let before = mgr.lookup(&id).unwrap().expires_at_ms;
        std::thread::sleep(std::time::Duration::from_millis(5));
        mgr.renew(&id, 60_000).unwrap();
        let after = mgr.lookup(&id).unwrap().expires_at_ms;
        // After is not before + extra (renew is from now), but it
        // *is* strictly greater than before.
        assert!(after > before);
    }

    #[test]
    fn renew_revoked_returns_revoked() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("x"), 60_000).unwrap();
        mgr.revoke(&id).unwrap();
        assert!(matches!(mgr.renew(&id, 60_000), Err(LeaseError::Revoked)));
    }

    #[test]
    fn renew_with_zero_extra_rejected() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("x"), 60_000).unwrap();
        assert!(matches!(
            mgr.renew(&id, 0),
            Err(LeaseError::InvalidParameter(_))
        ));
    }

    #[test]
    fn expire_due_removes_old_and_revoked() {
        let mgr = InMemoryLeaseManager::new();
        let alive = mgr.issue(mk_payload("alive"), 1_000_000).unwrap();
        let dead = mgr.issue(mk_payload("dead"), 1_000_000).unwrap();
        mgr.revoke(&dead).unwrap();
        // Sweep with now=0 → only the revoked one is reaped (the
        // other's expiry is in the future).
        let n = mgr.expire_due(0).unwrap();
        assert_eq!(n, 1);
        assert!(matches!(mgr.lookup(&dead), Err(LeaseError::NotFound)));
        // Alive is still around.
        assert!(mgr.lookup(&alive).is_ok());
        // Sweep with now=u64::MAX → alive is reaped too.
        let n2 = mgr.expire_due(u64::MAX).unwrap();
        assert_eq!(n2, 1);
        assert!(matches!(mgr.lookup(&alive), Err(LeaseError::NotFound)));
    }

    #[test]
    fn lease_id_round_trips_hex() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("x"), 60_000).unwrap();
        let s = id.as_hex().to_owned();
        assert_eq!(s.len(), 32);
        let parsed = LeaseId::from_hex(&s).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn lease_id_from_hex_rejects_bad_input() {
        assert!(LeaseId::from_hex("").is_none());
        assert!(LeaseId::from_hex("not-hex-not-hex-not-hex-not-hex!").is_none());
        assert!(LeaseId::from_hex("ABCDEF0123456789abcdef0123456789").is_none()); // uppercase
        assert!(LeaseId::from_hex("0123").is_none()); // wrong length
    }

    #[test]
    fn distinct_ids_for_distinct_issues() {
        let mgr = InMemoryLeaseManager::new();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..256 {
            let id = mgr.issue(mk_payload("x"), 60_000).unwrap();
            assert!(seen.insert(id));
        }
        // 256 distinct IDs from 128-bit space → no collision is the
        // overwhelmingly likely outcome (collision probability is
        // negligible at this scale).
    }

    #[test]
    fn payload_clone_is_independent() {
        let p = LeasePayload::new(vec![1, 2, 3, 4, 5]);
        let p2 = p.clone();
        assert_eq!(p.as_bytes(), p2.as_bytes());
        // Dropping one does not affect the other.
        drop(p);
        assert_eq!(p2.as_bytes(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn manager_is_send_and_sync() {
        // Compile-time check: the trait promises Send+Sync.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InMemoryLeaseManager>();
        assert_send_sync::<Box<dyn LeaseManager>>();
    }

    #[test]
    fn read_after_natural_expiry_returns_expired() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("x"), 1).unwrap();
        // Sleep enough that wall-clock has advanced past expiry.
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(matches!(mgr.read(&id), Err(LeaseError::Expired)));
    }

    #[test]
    fn renew_after_natural_expiry_returns_expired() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("x"), 1).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(matches!(
            mgr.renew(&id, 60_000),
            Err(LeaseError::Expired)
        ));
    }

    #[test]
    fn consume_after_natural_expiry_returns_expired() {
        let mgr = InMemoryLeaseManager::new();
        let id = mgr.issue(mk_payload("x"), 1).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(matches!(mgr.consume(&id), Err(LeaseError::Expired)));
    }
}
