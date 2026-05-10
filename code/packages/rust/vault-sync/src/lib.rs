//! # `coding_adventures_vault_sync` — VLT10 sync engine
//!
//! ## What this crate is
//!
//! The **multi-device sync layer** of the Vault stack. It
//! provides:
//!
//!   * a [`VersionVector`] primitive that captures *what each
//!     device has seen*, so the system can tell "A happened
//!     before B", "B happened after A", or "A and B happened
//!     concurrently",
//!   * a [`SyncRecord`] type that pairs an opaque ciphertext
//!     payload with the metadata the server needs to resolve
//!     order (namespace, key, version vector, last-writer
//!     timestamp + device, optional VLT04 wrap-set),
//!   * a [`SyncServer`] trait — `push` / `pull` / `list` — that
//!     a real deployment implements over Postgres / S3 / DynamoDB
//!     and that this crate provides as an [`InMemorySyncServer`]
//!     reference,
//!   * a [`LwwResolver`] that merges concurrent writes per the
//!     vault's documented policy: **last-writer-wins per record**
//!     with conflicts *surfaced* (not silently discarded) so the
//!     application layer can prompt the user when needed,
//!   * a small [`OrSet`] CRDT for the documented opt-in case
//!     ("tags" — adds + removes commute, no LWW loss).
//!
//! ## Storage-agnostic by construction
//!
//! The whole point of putting an E2EE sync layer between
//! `vault-sealed-store` (VLT01) and an untrusted server is that
//! the server learns *nothing* about the contents. This crate
//! enforces that property in the type system: a `SyncRecord`'s
//! payload is `Vec<u8>` with no per-payload typing on the
//! server side. Servers see:
//!
//! ```text
//!   (namespace_str, key_str, version_vector, lww_metadata,
//!    ciphertext_bytes, optional_wrap_set_bytes)
//! ```
//!
//! …and that is *all* they see. No record type, no field names,
//! no plaintext. The wrap-set (VLT04) is also opaque from the
//! server's perspective — it's the recipient-list-keyed-by-pubkey
//! that lets the client decrypt; the server forwards it
//! untouched.
//!
//! ## Conflict policy
//!
//! Two writers race on the same `(namespace, key)`:
//!
//!   * If one's version vector dominates the other (happens-
//!     after), the dominant write replaces the older one. No
//!     conflict.
//!   * If the version vectors are concurrent (incomparable),
//!     LWW kicks in: the write with the larger
//!     `last_writer_timestamp_ms` wins (with a deterministic
//!     `last_writer_device` tie-break for equal timestamps).
//!     The losing record is *retained as a sibling* in
//!     [`PushOutcome::ConflictResolved`] so the application
//!     can prompt the user.
//!
//! For fields where LWW is *wrong* (a tag list — losing one
//! tag because two devices added different tags is a bug, not a
//! merge), the application layer wraps the field in [`OrSet`]
//! and uses [`OrSet::merge`] to compute the union with proper
//! tombstone tracking.
//!
//! ## Where it fits
//!
//! ```text
//!   ┌──────────────────────────────────────────────┐
//!   │  application (Bitwarden / 1Password / Vault) │
//!   └──────────────────────┬───────────────────────┘
//!                          │ typed records (VLT02)
//!   ┌──────────────────────▼───────────────────────┐
//!   │  vault-sealed-store (VLT01)                  │
//!   │   produces opaque ciphertext bytes           │
//!   └──────────────────────┬───────────────────────┘
//!                          │ ciphertext + wrap-set
//!   ┌──────────────────────▼───────────────────────┐
//!   │  vault-sync  (THIS CRATE)                    │
//!   │   - per-device version vectors               │
//!   │   - last-writer-wins resolver                │
//!   │   - OR-set helper for CRDT-friendly fields   │
//!   │   - SyncServer trait                         │
//!   └──────────────────────┬───────────────────────┘
//!                          │ over the wire (TLS, gRPC, …)
//!   ┌──────────────────────▼───────────────────────┐
//!   │  server-side companion                       │
//!   │     ├─ InMemorySyncServer (this crate)       │
//!   │     ├─ vault-sync-postgres (future)          │
//!   │     └─ vault-sync-s3       (future)          │
//!   └──────────────────────────────────────────────┘
//! ```
//!
//! ## Threat model (sync tier)
//!
//! * **Untrusted server**: server sees only ciphertext bytes
//!   and per-record metadata it cannot interpret beyond version
//!   vectors. It cannot read, infer record type, or selectively
//!   leak fields. The chain is `client → ciphertext via VLT01 →
//!   server`, so this layer never sees plaintext.
//! * **Replay attempt**: a stale record submitted again with
//!   the *old* version vector is rejected with
//!   [`PushOutcome::Stale`] because its vector does not
//!   dominate the current server state.
//! * **Forgery via injection**: prevented at the layer below
//!   (VLT01 sealed-store + the wrap-set's recipient signatures);
//!   this crate's only forgery surface is "the server lies
//!   about the version vector". Detected by the client because
//!   each pulled record's vector must be `>=` the local view of
//!   that record, never strictly less.
//! * **Concurrent legitimate edits**: surfaced as a conflict
//!   rather than silently overwritten — see [`LwwResolver`].

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Reject identifiers containing control characters or
/// whitespace — these would enable log-injection / format-
/// confusion in downstream layers (logs, JSON-encoded transports
/// in VLT11). Permits printable ASCII + non-control Unicode.
fn is_safe_id_string(s: &str) -> bool {
    s.chars()
        .all(|c| !c.is_control() && !c.is_whitespace())
}

/// Best-effort lock that recovers from a poisoned mutex rather
/// than panicking. The data inside any of this crate's mutexes
/// is a coherent `HashMap<(String, String), SyncRecord>` snapshot;
/// no panic site mutates it mid-write, so recovering the inner
/// guard preserves invariants. Same rationale as `vault-audit`'s
/// `lock_recover`.
fn lock_recover<'a, T>(m: &'a Mutex<T>) -> MutexGuard<'a, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

// === Section 1. Device identity ============================================
//
// A `DeviceId` is the unit of *who participated in this version
// vector*. It needs to be:
//   - cheap to clone (used as map keys),
//   - canonically orderable (so vectors hash deterministically
//     for equality testing),
//   - opaque from a security standpoint — the server learns the
//     device's identifier as it propagates the version vector
//     but learns nothing about *who* that device belongs to
//     (that's an upstream identity concern).

/// Stable identifier of a device participating in the sync
/// graph. Backed by a `String` so callers can pick whatever
/// representation makes sense (UUID, public-key hex, hash of a
/// device certificate). The wire format is the literal string.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceId(String);

impl DeviceId {
    /// Wrap a string. Empty IDs are rejected because the empty
    /// string is reserved for "missing entry".
    pub fn new(id: impl Into<String>) -> Result<Self, SyncError> {
        let s: String = id.into();
        if s.is_empty() {
            return Err(SyncError::InvalidParameter("device id must not be empty"));
        }
        if s.len() > MAX_DEVICE_ID_LEN {
            return Err(SyncError::InvalidParameter(
                "device id exceeds MAX_DEVICE_ID_LEN",
            ));
        }
        if !is_safe_id_string(&s) {
            return Err(SyncError::InvalidParameter(
                "device id contains forbidden characters (control / whitespace)",
            ));
        }
        Ok(Self(s))
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Maximum bytes for a `DeviceId`.
pub const MAX_DEVICE_ID_LEN: usize = 256;
/// Maximum bytes for a namespace.
pub const MAX_NAMESPACE_LEN: usize = 128;
/// Maximum bytes for a key.
pub const MAX_KEY_LEN: usize = 512;
/// Maximum size of a single ciphertext payload (1 MiB).
/// This bounds memory growth; large attachments are an
/// out-of-scope concern (VLT14 attachments handles them via
/// chunked storage).
pub const MAX_CIPHERTEXT_LEN: usize = 1024 * 1024;
/// Maximum size of a single wrap-set payload (64 KiB).
pub const MAX_WRAP_SET_LEN: usize = 64 * 1024;

// === Section 2. Version vector =============================================
//
// A version vector V is a map from DeviceId to monotonic u64
// counter. A record's V[device] = "the highest write at this
// `(namespace, key)` that `device` has ever produced". Comparing
// two vectors:
//
//   - `a >= b` iff for every device d: a[d] >= b[d].
//   - `a > b`  iff `a >= b` and `a != b`. (Strict happens-after.)
//   - `a == b` iff for every device d: a[d] == b[d].
//   - otherwise `a` and `b` are *concurrent* (neither dominates).
//
// On a write, the writing device increments its own counter; on
// a merge, V_merged[d] = max(V1[d], V2[d]).

/// Per-device counter map. Sorted (`BTreeMap`) for deterministic
/// equality + canonical ordering when serialised.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VersionVector {
    counters: BTreeMap<DeviceId, u64>,
}

/// The four possible relationships between two version vectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorOrdering {
    /// Vectors are equal — same set of writes seen by both.
    Equal,
    /// Left strictly dominates right (left has seen everything
    /// right has, plus at least one more).
    Dominates,
    /// Right strictly dominates left.
    DominatedBy,
    /// Neither dominates — concurrent edits.
    Concurrent,
}

impl VersionVector {
    /// Empty vector — what a fresh `(namespace, key)` starts at.
    pub fn new() -> Self {
        Self {
            counters: BTreeMap::new(),
        }
    }

    /// Fluent builder: increment `device`'s counter by 1, or
    /// initialise it to 1 if not present.
    ///
    /// **Panics** on `u64` overflow. 2^64 distinct writes from
    /// one device is unreachable in any realistic deployment
    /// (a billion writes per second for ~585 years), so we
    /// treat the saturation case as a programming-error
    /// signal rather than a silent no-op (which would route
    /// every subsequent write through the equal-vector branch
    /// and confuse downstream conflict resolution).
    pub fn bump(mut self, device: &DeviceId) -> Self {
        let c = self.counters.entry(device.clone()).or_insert(0);
        *c = c
            .checked_add(1)
            .expect("VersionVector counter overflow (2^64 writes from one device — programming error)");
        self
    }

    /// Read a device's counter (0 if not present).
    pub fn get(&self, device: &DeviceId) -> u64 {
        self.counters.get(device).copied().unwrap_or(0)
    }

    /// Pointwise maximum with `other`. Used when merging two
    /// known states (e.g. resolving concurrent writes — both
    /// sides "have seen" everything that's in either vector).
    pub fn merge(&self, other: &VersionVector) -> VersionVector {
        let mut out: BTreeMap<DeviceId, u64> = self.counters.clone();
        for (d, &v) in &other.counters {
            let entry = out.entry(d.clone()).or_insert(0);
            if v > *entry {
                *entry = v;
            }
        }
        VersionVector { counters: out }
    }

    /// Compare two vectors. The four-valued result tells the
    /// caller what kind of merge is required.
    pub fn compare(&self, other: &VersionVector) -> VectorOrdering {
        let mut left_ge = true;
        let mut right_ge = true;
        // Walk the union of devices.
        let mut all_devices: BTreeSet<&DeviceId> = self.counters.keys().collect();
        for d in other.counters.keys() {
            all_devices.insert(d);
        }
        for d in all_devices {
            let lv = self.get(d);
            let rv = other.get(d);
            if lv < rv {
                left_ge = false;
            }
            if rv < lv {
                right_ge = false;
            }
        }
        match (left_ge, right_ge) {
            (true, true) => VectorOrdering::Equal,
            (true, false) => VectorOrdering::Dominates,
            (false, true) => VectorOrdering::DominatedBy,
            (false, false) => VectorOrdering::Concurrent,
        }
    }

    /// `true` iff `self` strictly dominates `other` (happens-
    /// after).
    pub fn dominates(&self, other: &VersionVector) -> bool {
        matches!(self.compare(other), VectorOrdering::Dominates)
    }

    /// `true` iff `self` and `other` are concurrent.
    pub fn concurrent_with(&self, other: &VersionVector) -> bool {
        matches!(self.compare(other), VectorOrdering::Concurrent)
    }
}

// === Section 3. SyncRecord ================================================
//
// What flies over the wire. The server treats the `payload` as
// completely opaque — it receives bytes, stores bytes, ships
// bytes back, never looks inside. The metadata fields are *only*
// what the server needs for ordering.

/// One synchronised record. The payload is opaque ciphertext —
/// the server never interprets it. The wrap-set (recipient list
/// keyed by public key) is similarly opaque from the server's
/// perspective; it accompanies the record so that any device
/// authorised to read can pull both halves and decrypt.
///
/// `Debug` is hand-rolled to redact `ciphertext` and `wrap_set`:
/// even though they are encrypted, logging them is a fingerprint
/// / oracle surface (size persistence, replay aids) and the
/// derived `Vec<u8>` Debug would dump every byte. The redacted
/// form prints lengths only, which is enough for triage.
#[derive(Clone, PartialEq, Eq)]
pub struct SyncRecord {
    /// Namespace (logical container). Server-visible string.
    pub namespace: String,
    /// Key within the namespace. Server-visible string.
    pub key: String,
    /// Causality vector: who's seen what.
    pub version_vector: VersionVector,
    /// Device that produced this version (for LWW tie-break).
    pub last_writer: DeviceId,
    /// Wall-clock (ms since epoch) at the writing device when
    /// this version was produced. Caller-supplied so the crate
    /// stays clock-pure for tests.
    pub last_writer_ms: u64,
    /// Encrypted record body (opaque to the server). Bounded by
    /// [`MAX_CIPHERTEXT_LEN`].
    pub ciphertext: Vec<u8>,
    /// Optional wrap-set (recipient-list keyed by pubkey, from
    /// VLT04). Bounded by [`MAX_WRAP_SET_LEN`]. `None` when the
    /// record is single-recipient.
    pub wrap_set: Option<Vec<u8>>,
}

impl core::fmt::Debug for SyncRecord {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SyncRecord")
            .field("namespace", &self.namespace)
            .field("key", &self.key)
            .field("version_vector", &self.version_vector)
            .field("last_writer", &self.last_writer)
            .field("last_writer_ms", &self.last_writer_ms)
            .field(
                "ciphertext",
                &format_args!("<{} bytes redacted>", self.ciphertext.len()),
            )
            .field(
                "wrap_set",
                &format_args!(
                    "{}",
                    match &self.wrap_set {
                        Some(w) => format!("Some(<{} bytes redacted>)", w.len()),
                        None => "None".to_string(),
                    }
                ),
            )
            .finish()
    }
}

impl SyncRecord {
    /// Validate the record's bounds. Called by the server on
    /// `push` and by clients before sending — no `unwrap` on
    /// the wire.
    pub fn validate(&self) -> Result<(), SyncError> {
        if self.namespace.is_empty() {
            return Err(SyncError::InvalidParameter("namespace must not be empty"));
        }
        if self.namespace.len() > MAX_NAMESPACE_LEN {
            return Err(SyncError::InvalidParameter(
                "namespace exceeds MAX_NAMESPACE_LEN",
            ));
        }
        if !is_safe_id_string(&self.namespace) {
            return Err(SyncError::InvalidParameter(
                "namespace contains forbidden characters",
            ));
        }
        if self.key.is_empty() {
            return Err(SyncError::InvalidParameter("key must not be empty"));
        }
        if self.key.len() > MAX_KEY_LEN {
            return Err(SyncError::InvalidParameter("key exceeds MAX_KEY_LEN"));
        }
        if !is_safe_id_string(&self.key) {
            return Err(SyncError::InvalidParameter(
                "key contains forbidden characters",
            ));
        }
        if self.ciphertext.is_empty() {
            return Err(SyncError::InvalidParameter("ciphertext must not be empty"));
        }
        if self.ciphertext.len() > MAX_CIPHERTEXT_LEN {
            return Err(SyncError::InvalidParameter(
                "ciphertext exceeds MAX_CIPHERTEXT_LEN",
            ));
        }
        if let Some(w) = &self.wrap_set {
            if w.is_empty() {
                return Err(SyncError::InvalidParameter("wrap_set must not be empty if Some"));
            }
            if w.len() > MAX_WRAP_SET_LEN {
                return Err(SyncError::InvalidParameter(
                    "wrap_set exceeds MAX_WRAP_SET_LEN",
                ));
            }
        }
        if self.version_vector.counters.is_empty() {
            return Err(SyncError::InvalidParameter(
                "version_vector must contain at least the last_writer",
            ));
        }
        if self.version_vector.get(&self.last_writer) == 0 {
            return Err(SyncError::InvalidParameter(
                "version_vector must contain a non-zero entry for last_writer",
            ));
        }
        Ok(())
    }
}

// === Section 4. Errors =====================================================

/// All errors produced by the sync layer.
#[derive(Debug)]
pub enum SyncError {
    /// Caller supplied a malformed record / argument.
    InvalidParameter(&'static str),
    /// Server-side problem (full disk, transport break, etc).
    Server(String),
    /// `(namespace, key)` does not exist.
    NotFound,
}

impl core::fmt::Display for SyncError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidParameter(why) => write!(f, "invalid parameter: {}", why),
            Self::Server(why) => write!(f, "server error: {}", why),
            Self::NotFound => write!(f, "record not found"),
        }
    }
}

impl std::error::Error for SyncError {}

// === Section 5. Push / pull surface ========================================
//
// `push` returns one of four outcomes:
//
//   1. `Applied`        — record is now the canonical version.
//   2. `Stale`          — incoming version is dominated by the
//                          server's record; client should pull
//                          and rebase.
//   3. `ConflictResolved` — incoming version is concurrent with
//                          the server's; the resolver picked one
//                          (LWW). Both sides are returned so the
//                          client/UI can present a conflict
//                          dialog.
//   4. `Unchanged`      — incoming version equals server's;
//                          no-op (same record, idempotent push).

/// Outcome of a [`SyncServer::push`] call.
#[derive(Clone, Debug)]
pub enum PushOutcome {
    /// The pushed record is now the canonical record on the
    /// server. The new version vector is the same as the one
    /// the client supplied.
    Applied {
        /// The record now stored on the server.
        stored: SyncRecord,
    },
    /// The pushed version is strictly dominated by the
    /// server's. The client should pull and try again with a
    /// post-merge vector.
    Stale {
        /// The newer record currently held by the server.
        server: SyncRecord,
    },
    /// Concurrent edit. The resolver picked a winner under LWW;
    /// both records are returned so the application can surface
    /// the conflict to the user. The "winner"'s version_vector
    /// is the merge of the two inputs.
    ConflictResolved {
        /// The record that prevailed under LWW (now stored).
        winner: SyncRecord,
        /// The record that lost — preserved so the application
        /// can present a conflict-resolution UI.
        loser: SyncRecord,
    },
    /// The pushed record is identical to the server's (same
    /// vector, equal payload). Idempotent retry.
    Unchanged,
}

/// Sync server contract. Concrete implementations land in
/// sibling crates (Postgres, S3, etc).
///
/// **Authorization is OUT OF SCOPE for this trait.** Every
/// method assumes the caller is already authenticated and
/// authorised. A networked transport (VLT11) MUST gate
/// `push`/`pull`/`get` per-namespace via VLT05 + VLT06 before
/// dispatching to a `SyncServer` implementation.
///
/// **Wire-tier validation is OUT OF SCOPE for `validate()`.**
/// `SyncRecord::validate` runs against an *already-allocated*
/// record. A networked transport MUST enforce the
/// `MAX_*_LEN` caps at the deserialiser before constructing
/// `SyncRecord`, otherwise an attacker pushing a multi-GiB
/// body fails validation only after the bytes are in memory.
pub trait SyncServer: Send + Sync {
    /// Submit a record. The server validates, compares against
    /// any existing record at `(namespace, key)`, and returns
    /// the outcome.
    fn push(&self, record: SyncRecord) -> Result<PushOutcome, SyncError>;

    /// Fetch a single record by `(namespace, key)`.
    fn get(&self, namespace: &str, key: &str) -> Result<SyncRecord, SyncError>;

    /// List all records in `namespace` whose version vector is
    /// *not* dominated by `since`. Empty `since` returns
    /// everything. Used by clients on reconnect to catch up.
    fn pull(
        &self,
        namespace: &str,
        since: &VersionVector,
    ) -> Result<Vec<SyncRecord>, SyncError>;
}

// === Section 6. LwwResolver ================================================

/// Resolver for concurrent writes. The default policy is
/// last-writer-wins with a deterministic device-id tie-break:
///
/// * Higher `last_writer_ms` wins.
/// * If tied, the lexicographically *smaller* `last_writer`
///   `DeviceId` wins (so the choice is deterministic across
///   replicas — no flapping).
///
/// The merged vector is `a.version_vector.merge(&b.version_vector)`.
pub struct LwwResolver;

impl LwwResolver {
    /// Pick the winner between two concurrent records and
    /// return `(winner, loser)` with the winner's
    /// `version_vector` upgraded to the merged vector. The
    /// winner's payload, ciphertext, and last_writer fields are
    /// preserved from the original winner.
    pub fn resolve(a: SyncRecord, b: SyncRecord) -> (SyncRecord, SyncRecord) {
        let merged = a.version_vector.merge(&b.version_vector);
        let a_wins = match a.last_writer_ms.cmp(&b.last_writer_ms) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => a.last_writer < b.last_writer,
        };
        let (mut winner, loser) = if a_wins { (a, b) } else { (b, a) };
        winner.version_vector = merged;
        (winner, loser)
    }
}

// === Section 7. OR-set CRDT ================================================
//
// For fields where LWW is wrong (most notoriously a tag list:
// device A adds "work", device B adds "personal", LWW would
// drop one). An OR-set tracks per-element add timestamps and
// per-element remove timestamps; an element is "in" the set iff
// its latest add is more recent than its latest remove.
//
// We use `(value, unique_tag)` pairs so the same value can be
// added, removed, and re-added without confusing the tombstone
// machinery — the standard observed-removal semantics.

/// Per-element observation in the OR-set.
#[derive(Clone, Debug, PartialEq, Eq)]
struct OrEntry {
    /// Per-(device, value) unique tag (just the device id +
    /// counter — same shape as a version-vector entry).
    tag: (DeviceId, u64),
    /// Wall-clock observation time, used as tiebreaker when
    /// merging entries that share the same tag (which they
    /// shouldn't, but we encode defensively).
    observed_ms: u64,
}

/// Observed-removal Set CRDT keyed by `String`. The merge of two
/// `OrSet`s is the union of their `adds` minus the union of
/// their `removes`. A given value is "in the set" iff some add
/// tag for it exists that no removal has shadowed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OrSet {
    /// `value → list of add-observations`. Multiple adds of the
    /// same value across devices each get their own tag.
    adds: BTreeMap<String, Vec<OrEntry>>,
    /// `value → set of removal tags`. A removal references a
    /// specific add tag, so re-adding after removal works.
    removes: BTreeMap<String, BTreeSet<(DeviceId, u64)>>,
}

impl OrSet {
    /// Empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add `value` on behalf of `device` at `now_ms` using a
    /// counter derived from this `OrSet`'s local state. Each
    /// call produces a fresh tag, so add-then-remove-then-add
    /// works correctly across devices.
    ///
    /// **Constraint** — for CRDT correctness, only **one**
    /// `OrSet` instance per `(device, logical-field)` may exist
    /// at a time. Two independently-constructed `OrSet`s on the
    /// same device that each call `add("x")` will both produce
    /// tag `(device, 1)`; merging will collapse them, and a
    /// subsequent remove on either side will retroactively
    /// tombstone both. If your application creates `OrSet`s
    /// from independent sources (e.g. restoring from snapshot
    /// + a parallel live edit), use [`OrSet::add_with_tag`] and
    /// supply a globally-unique tag value (e.g. derived from
    /// the field's `VersionVector[device]` counter).
    pub fn add(&mut self, value: impl Into<String>, device: &DeviceId, now_ms: u64) {
        let v = value.into();
        let entries = self.adds.entry(v).or_default();
        let next_counter = entries
            .iter()
            .filter(|e| &e.tag.0 == device)
            .map(|e| e.tag.1)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        entries.push(OrEntry {
            tag: (device.clone(), next_counter),
            observed_ms: now_ms,
        });
    }

    /// Add `value` on behalf of `device` with a caller-supplied
    /// `tag_id`. The caller is responsible for ensuring the
    /// `(device, value, tag_id)` triple is globally unique
    /// across every `OrSet` that may merge into this one — a
    /// natural source is the field's per-device write counter
    /// from the surrounding `VersionVector`.
    ///
    /// Idempotent: two `add_with_tag` calls with the same tag
    /// are equivalent to one (the second is dropped on merge).
    /// This is what makes `OrSet` truly mergeable across
    /// independently-constructed instances.
    pub fn add_with_tag(
        &mut self,
        value: impl Into<String>,
        device: &DeviceId,
        tag_id: u64,
        now_ms: u64,
    ) {
        let v = value.into();
        let entries = self.adds.entry(v).or_default();
        let tag = (device.clone(), tag_id);
        if entries.iter().any(|e| e.tag == tag) {
            // Already observed — idempotent.
            return;
        }
        entries.push(OrEntry {
            tag,
            observed_ms: now_ms,
        });
    }

    /// Remove every currently-observed add-tag for `value`. New
    /// adds *after* this remove will reappear as expected.
    pub fn remove(&mut self, value: &str) {
        if let Some(adds) = self.adds.get(value) {
            let tombstones: BTreeSet<(DeviceId, u64)> =
                adds.iter().map(|e| e.tag.clone()).collect();
            let entry = self.removes.entry(value.to_string()).or_default();
            for t in tombstones {
                entry.insert(t);
            }
        }
    }

    /// `true` iff `value` is currently in the set.
    pub fn contains(&self, value: &str) -> bool {
        let adds = match self.adds.get(value) {
            Some(a) => a,
            None => return false,
        };
        let removes = self.removes.get(value);
        for entry in adds {
            let removed = match removes {
                Some(rs) => rs.contains(&entry.tag),
                None => false,
            };
            if !removed {
                return true;
            }
        }
        false
    }

    /// Snapshot of all currently-present values, sorted.
    pub fn values(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .adds
            .keys()
            .filter(|v| self.contains(v))
            .cloned()
            .collect();
        out.sort();
        out
    }

    /// Merge another OR-set into this one. The merge is
    /// idempotent, commutative, and associative — the CRDT
    /// invariant.
    pub fn merge(&self, other: &OrSet) -> OrSet {
        let mut out = self.clone();
        for (val, entries) in &other.adds {
            let target = out.adds.entry(val.clone()).or_default();
            for e in entries {
                if !target.iter().any(|x| x.tag == e.tag) {
                    target.push(e.clone());
                }
            }
        }
        for (val, tags) in &other.removes {
            let target = out.removes.entry(val.clone()).or_default();
            for t in tags {
                target.insert(t.clone());
            }
        }
        out
    }
}

// === Section 8. InMemorySyncServer =========================================
//
// Reference implementation. Suitable for unit tests of clients
// and for single-process tools (a local-only password manager
// that still wants the ordered conflict semantics).

/// Threadsafe in-memory `SyncServer`. Production deployments
/// implement the trait against Postgres / SQLite / S3 with the
/// same protocol.
pub struct InMemorySyncServer {
    inner: Mutex<HashMap<(String, String), SyncRecord>>,
}

impl Default for InMemorySyncServer {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemorySyncServer {
    /// Construct a fresh, empty server.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl SyncServer for InMemorySyncServer {
    fn push(&self, record: SyncRecord) -> Result<PushOutcome, SyncError> {
        record.validate()?;
        let mut g = lock_recover(&self.inner);
        let coord = (record.namespace.clone(), record.key.clone());
        let existing = g.remove(&coord);
        let outcome = match existing {
            None => {
                // First-write — accept unconditionally.
                let stored = record.clone();
                g.insert(coord, stored.clone());
                PushOutcome::Applied { stored }
            }
            Some(server_rec) => {
                match record.version_vector.compare(&server_rec.version_vector) {
                    VectorOrdering::Dominates => {
                        // Incoming dominates — accept.
                        let stored = record.clone();
                        g.insert(coord, stored.clone());
                        PushOutcome::Applied { stored }
                    }
                    VectorOrdering::Equal => {
                        // Vectors equal: same revision. The
                        // record is byte-for-byte identical to
                        // what we already have only when ALL
                        // fields match — including `wrap_set`
                        // and `last_writer_ms`. Comparing only
                        // ciphertext+writer would silently drop
                        // a recipient-set rotation push (e.g.
                        // VLT04 wrap-set updated to add a new
                        // device or revoke an old one without
                        // changing the underlying ciphertext).
                        // Any other equal-vector mismatch is a
                        // protocol violation by the clients
                        // (they didn't bump their counter) and
                        // is defensively routed through LWW.
                        let truly_unchanged = record.ciphertext == server_rec.ciphertext
                            && record.last_writer == server_rec.last_writer
                            && record.last_writer_ms == server_rec.last_writer_ms
                            && record.wrap_set == server_rec.wrap_set;
                        if truly_unchanged {
                            g.insert(coord, server_rec);
                            PushOutcome::Unchanged
                        } else {
                            let (winner, loser) =
                                LwwResolver::resolve(record, server_rec);
                            g.insert(coord, winner.clone());
                            PushOutcome::ConflictResolved { winner, loser }
                        }
                    }
                    VectorOrdering::DominatedBy => {
                        // Server has strictly newer record —
                        // reject and return server view.
                        let server_clone = server_rec.clone();
                        g.insert(coord, server_rec);
                        PushOutcome::Stale {
                            server: server_clone,
                        }
                    }
                    VectorOrdering::Concurrent => {
                        // True conflict: pick a winner via LWW
                        // and surface both halves.
                        let (winner, loser) = LwwResolver::resolve(record, server_rec);
                        g.insert(coord, winner.clone());
                        PushOutcome::ConflictResolved { winner, loser }
                    }
                }
            }
        };
        Ok(outcome)
    }

    fn get(&self, namespace: &str, key: &str) -> Result<SyncRecord, SyncError> {
        let g = lock_recover(&self.inner);
        g.get(&(namespace.to_string(), key.to_string()))
            .cloned()
            .ok_or(SyncError::NotFound)
    }

    fn pull(
        &self,
        namespace: &str,
        since: &VersionVector,
    ) -> Result<Vec<SyncRecord>, SyncError> {
        let g = lock_recover(&self.inner);
        let mut out: Vec<SyncRecord> = g
            .iter()
            .filter(|((ns, _), _)| ns == namespace)
            .filter(|(_, rec)| {
                // Skip records the caller has already seen.
                // "Already seen" = the record's version vector
                // is `<=` `since`, i.e. `since.dominates(rec) ||
                // since == rec`.
                match since.compare(&rec.version_vector) {
                    VectorOrdering::Equal | VectorOrdering::Dominates => false,
                    _ => true,
                }
            })
            .map(|(_, rec)| rec.clone())
            .collect();
        // Stable order — sorted by key — so two clients see the
        // same ordering on identical state.
        out.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(out)
    }
}

// === Section 9. Tests ======================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(s: &str) -> DeviceId {
        DeviceId::new(s).unwrap()
    }

    fn rec(
        ns: &str,
        key: &str,
        vv: VersionVector,
        last_writer: DeviceId,
        ts: u64,
        body: &[u8],
    ) -> SyncRecord {
        SyncRecord {
            namespace: ns.into(),
            key: key.into(),
            version_vector: vv,
            last_writer,
            last_writer_ms: ts,
            ciphertext: body.to_vec(),
            wrap_set: None,
        }
    }

    // --- VersionVector ---

    #[test]
    fn empty_vector_equals_itself() {
        let a = VersionVector::new();
        let b = VersionVector::new();
        assert_eq!(a.compare(&b), VectorOrdering::Equal);
    }

    #[test]
    fn bumping_one_device_dominates_empty() {
        let a = VersionVector::new().bump(&dev("A"));
        let b = VersionVector::new();
        assert_eq!(a.compare(&b), VectorOrdering::Dominates);
        assert_eq!(b.compare(&a), VectorOrdering::DominatedBy);
        assert!(a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn divergent_devices_are_concurrent() {
        let a = VersionVector::new().bump(&dev("A"));
        let b = VersionVector::new().bump(&dev("B"));
        assert_eq!(a.compare(&b), VectorOrdering::Concurrent);
        assert!(a.concurrent_with(&b));
    }

    #[test]
    fn merge_takes_pointwise_max() {
        let a = VersionVector::new().bump(&dev("A")).bump(&dev("A"));
        let b = VersionVector::new().bump(&dev("A")).bump(&dev("B"));
        let m = a.merge(&b);
        assert_eq!(m.get(&dev("A")), 2);
        assert_eq!(m.get(&dev("B")), 1);
    }

    #[test]
    fn merge_dominates_both_inputs() {
        let a = VersionVector::new().bump(&dev("A")).bump(&dev("A"));
        let b = VersionVector::new().bump(&dev("B"));
        let m = a.merge(&b);
        assert!(m.dominates(&a) || m.compare(&a) == VectorOrdering::Equal);
        assert!(m.dominates(&b) || m.compare(&b) == VectorOrdering::Equal);
    }

    // --- SyncRecord validation ---

    #[test]
    fn validate_rejects_empty_namespace() {
        let r = rec(
            "",
            "k",
            VersionVector::new().bump(&dev("A")),
            dev("A"),
            1,
            b"x",
        );
        assert!(matches!(
            r.validate(),
            Err(SyncError::InvalidParameter(_))
        ));
    }

    #[test]
    fn validate_rejects_empty_key() {
        let r = rec(
            "ns",
            "",
            VersionVector::new().bump(&dev("A")),
            dev("A"),
            1,
            b"x",
        );
        assert!(matches!(
            r.validate(),
            Err(SyncError::InvalidParameter(_))
        ));
    }

    #[test]
    fn validate_rejects_empty_ciphertext() {
        let r = rec(
            "ns",
            "k",
            VersionVector::new().bump(&dev("A")),
            dev("A"),
            1,
            b"",
        );
        assert!(matches!(
            r.validate(),
            Err(SyncError::InvalidParameter(_))
        ));
    }

    #[test]
    fn validate_rejects_oversize_ciphertext() {
        let r = rec(
            "ns",
            "k",
            VersionVector::new().bump(&dev("A")),
            dev("A"),
            1,
            &vec![0u8; MAX_CIPHERTEXT_LEN + 1],
        );
        assert!(matches!(
            r.validate(),
            Err(SyncError::InvalidParameter(_))
        ));
    }

    #[test]
    fn validate_rejects_vector_without_last_writer() {
        let r = rec(
            "ns",
            "k",
            VersionVector::new().bump(&dev("A")),
            dev("B"), // Last writer not in vector.
            1,
            b"x",
        );
        assert!(matches!(
            r.validate(),
            Err(SyncError::InvalidParameter(_))
        ));
    }

    // --- DeviceId ---

    #[test]
    fn device_id_rejects_empty() {
        assert!(matches!(
            DeviceId::new(""),
            Err(SyncError::InvalidParameter(_))
        ));
    }

    #[test]
    fn device_id_rejects_control_chars() {
        // Newline / NUL / etc. would let downstream logs and
        // wire transports be tricked by a malicious caller.
        assert!(matches!(
            DeviceId::new("alice\nrole=admin"),
            Err(SyncError::InvalidParameter(_))
        ));
        assert!(matches!(
            DeviceId::new("alice\0"),
            Err(SyncError::InvalidParameter(_))
        ));
        assert!(matches!(
            DeviceId::new("alice bob"), // whitespace
            Err(SyncError::InvalidParameter(_))
        ));
    }

    #[test]
    fn validate_rejects_namespace_with_control_chars() {
        let r = rec(
            "ns\nrole=admin",
            "k",
            VersionVector::new().bump(&dev("A")),
            dev("A"),
            1,
            b"x",
        );
        assert!(matches!(
            r.validate(),
            Err(SyncError::InvalidParameter(_))
        ));
    }

    #[test]
    fn validate_rejects_key_with_control_chars() {
        let r = rec(
            "ns",
            "k\0",
            VersionVector::new().bump(&dev("A")),
            dev("A"),
            1,
            b"x",
        );
        assert!(matches!(
            r.validate(),
            Err(SyncError::InvalidParameter(_))
        ));
    }

    #[test]
    fn debug_redacts_ciphertext_and_wrap_set() {
        let r = SyncRecord {
            namespace: "n".into(),
            key: "k".into(),
            version_vector: VersionVector::new().bump(&dev("A")),
            last_writer: dev("A"),
            last_writer_ms: 1,
            ciphertext: b"super-secret-bytes".to_vec(),
            wrap_set: Some(b"recipient-list".to_vec()),
        };
        let s = format!("{:?}", r);
        assert!(!s.contains("super-secret-bytes"));
        assert!(!s.contains("recipient-list"));
        assert!(s.contains("18 bytes redacted"));
        assert!(s.contains("14 bytes redacted"));
    }

    #[test]
    fn or_set_add_with_tag_is_idempotent() {
        let mut a = OrSet::new();
        a.add_with_tag("work", &dev("A"), 1, 100);
        a.add_with_tag("work", &dev("A"), 1, 200); // same tag → no-op
        assert!(a.contains("work"));
        // Only one tag is recorded.
        assert_eq!(a.adds.get("work").map(|v| v.len()).unwrap_or(0), 1);
    }

    #[test]
    fn or_set_add_with_tag_supports_independent_instances() {
        // Two OrSets constructed independently on the same
        // device, each adding "work". Without explicit tags,
        // both would issue tag (A, 1) and merging would collapse.
        // With caller-supplied tags rooted in a shared counter
        // source (here, simulated via two distinct tag_ids),
        // both observations survive merge.
        let mut a = OrSet::new();
        a.add_with_tag("work", &dev("A"), 5, 100);
        let mut b = OrSet::new();
        b.add_with_tag("work", &dev("A"), 6, 200);
        let m = a.merge(&b);
        assert!(m.contains("work"));
        assert_eq!(m.adds.get("work").map(|v| v.len()).unwrap_or(0), 2);
    }

    #[test]
    fn device_id_rejects_oversize() {
        assert!(matches!(
            DeviceId::new("x".repeat(MAX_DEVICE_ID_LEN + 1)),
            Err(SyncError::InvalidParameter(_))
        ));
    }

    // --- LWW resolver ---

    #[test]
    fn lww_picks_higher_timestamp() {
        let a = rec(
            "n",
            "k",
            VersionVector::new().bump(&dev("A")),
            dev("A"),
            10,
            b"a",
        );
        let b = rec(
            "n",
            "k",
            VersionVector::new().bump(&dev("B")),
            dev("B"),
            20,
            b"b",
        );
        let (winner, loser) = LwwResolver::resolve(a.clone(), b.clone());
        assert_eq!(winner.ciphertext, b"b");
        assert_eq!(loser.ciphertext, b"a");
        // Winner's vector is the merge of both.
        assert!(winner.version_vector.dominates(&a.version_vector));
        assert!(winner.version_vector.dominates(&b.version_vector)
            || winner.version_vector == b.version_vector);
    }

    #[test]
    fn lww_breaks_ties_by_smaller_device_id() {
        let a = rec(
            "n",
            "k",
            VersionVector::new().bump(&dev("alice")),
            dev("alice"),
            42,
            b"alice-bytes",
        );
        let b = rec(
            "n",
            "k",
            VersionVector::new().bump(&dev("bob")),
            dev("bob"),
            42,
            b"bob-bytes",
        );
        let (winner, _) = LwwResolver::resolve(a, b);
        assert_eq!(winner.ciphertext, b"alice-bytes");
        assert_eq!(winner.last_writer, dev("alice"));
    }

    // --- Server flows ---

    #[test]
    fn first_push_is_applied() {
        let s = InMemorySyncServer::new();
        let r = rec(
            "n",
            "k",
            VersionVector::new().bump(&dev("A")),
            dev("A"),
            1,
            b"v1",
        );
        match s.push(r).unwrap() {
            PushOutcome::Applied { stored } => assert_eq!(stored.ciphertext, b"v1"),
            other => panic!("wrong outcome: {:?}", other),
        }
    }

    #[test]
    fn dominating_push_replaces_existing() {
        let s = InMemorySyncServer::new();
        let v1 = VersionVector::new().bump(&dev("A"));
        let v2 = v1.clone().bump(&dev("A"));
        s.push(rec("n", "k", v1, dev("A"), 1, b"v1")).unwrap();
        let outcome = s
            .push(rec("n", "k", v2, dev("A"), 2, b"v2"))
            .unwrap();
        assert!(matches!(outcome, PushOutcome::Applied { .. }));
        let now = s.get("n", "k").unwrap();
        assert_eq!(now.ciphertext, b"v2");
    }

    #[test]
    fn dominated_push_returns_stale() {
        let s = InMemorySyncServer::new();
        let v1 = VersionVector::new().bump(&dev("A"));
        let v2 = v1.clone().bump(&dev("A"));
        s.push(rec("n", "k", v2, dev("A"), 2, b"v2")).unwrap();
        let outcome = s
            .push(rec("n", "k", v1, dev("A"), 1, b"v1"))
            .unwrap();
        assert!(matches!(outcome, PushOutcome::Stale { .. }));
        // Server unchanged.
        assert_eq!(s.get("n", "k").unwrap().ciphertext, b"v2");
    }

    #[test]
    fn idempotent_push_returns_unchanged() {
        let s = InMemorySyncServer::new();
        let v1 = VersionVector::new().bump(&dev("A"));
        let r = rec("n", "k", v1, dev("A"), 1, b"v1");
        s.push(r.clone()).unwrap();
        let outcome = s.push(r).unwrap();
        assert!(matches!(outcome, PushOutcome::Unchanged));
    }

    #[test]
    fn concurrent_push_resolves_via_lww() {
        let s = InMemorySyncServer::new();
        let va = VersionVector::new().bump(&dev("A"));
        let vb = VersionVector::new().bump(&dev("B"));
        s.push(rec("n", "k", va, dev("A"), 10, b"alice")).unwrap();
        let outcome = s
            .push(rec("n", "k", vb, dev("B"), 20, b"bob"))
            .unwrap();
        match outcome {
            PushOutcome::ConflictResolved { winner, loser } => {
                assert_eq!(winner.ciphertext, b"bob"); // higher ts wins
                assert_eq!(loser.ciphertext, b"alice");
                // Winner's vector dominates both originals.
                assert!(winner
                    .version_vector
                    .dominates(&VersionVector::new().bump(&dev("A"))));
                assert!(winner
                    .version_vector
                    .dominates(&VersionVector::new().bump(&dev("B"))));
            }
            other => panic!("expected ConflictResolved, got {:?}", other),
        }
    }

    #[test]
    fn equal_vector_same_bytes_different_wrap_set_is_conflict() {
        // Re-pushing a record under the same vector but with a
        // different wrap_set (recipient-set rotation, e.g. a
        // device added via VLT04) must NOT be silently dropped.
        // The server treats it as a conflict so the upper layer
        // can prompt the user / pick which wrap-set wins.
        let s = InMemorySyncServer::new();
        let v = VersionVector::new().bump(&dev("A"));
        let r1 = SyncRecord {
            namespace: "n".into(),
            key: "k".into(),
            version_vector: v.clone(),
            last_writer: dev("A"),
            last_writer_ms: 10,
            ciphertext: b"same".to_vec(),
            wrap_set: Some(b"old-recipients".to_vec()),
        };
        let r2 = SyncRecord {
            wrap_set: Some(b"new-recipients".to_vec()),
            ..r1.clone()
        };
        s.push(r1).unwrap();
        let outcome = s.push(r2).unwrap();
        assert!(matches!(outcome, PushOutcome::ConflictResolved { .. }));
    }

    #[test]
    fn equal_vector_with_different_bytes_is_treated_as_conflict() {
        // Two clients writing under the same vector with
        // different bytes is a protocol violation by the
        // clients (they should have bumped their own counter
        // first), but the server defends by treating it as a
        // conflict instead of silently overwriting.
        let s = InMemorySyncServer::new();
        let v = VersionVector::new().bump(&dev("A"));
        s.push(rec("n", "k", v.clone(), dev("A"), 10, b"first"))
            .unwrap();
        let outcome = s
            .push(rec("n", "k", v, dev("A"), 20, b"second"))
            .unwrap();
        assert!(matches!(outcome, PushOutcome::ConflictResolved { .. }));
    }

    #[test]
    fn pull_returns_unseen_records_only() {
        let s = InMemorySyncServer::new();
        for k in &["a", "b", "c"] {
            s.push(rec(
                "n",
                k,
                VersionVector::new().bump(&dev("A")),
                dev("A"),
                1,
                b"x",
            ))
            .unwrap();
        }
        // since=A=1 → server has A=1 for all three — none are new.
        let since = VersionVector::new().bump(&dev("A"));
        let unseen = s.pull("n", &since).unwrap();
        assert!(unseen.is_empty());
        // since=A=0 (empty) → all three are new.
        let unseen2 = s.pull("n", &VersionVector::new()).unwrap();
        assert_eq!(unseen2.len(), 3);
    }

    #[test]
    fn pull_skips_other_namespaces() {
        let s = InMemorySyncServer::new();
        s.push(rec(
            "ns1",
            "k",
            VersionVector::new().bump(&dev("A")),
            dev("A"),
            1,
            b"x",
        ))
        .unwrap();
        s.push(rec(
            "ns2",
            "k",
            VersionVector::new().bump(&dev("A")),
            dev("A"),
            1,
            b"y",
        ))
        .unwrap();
        let r1 = s.pull("ns1", &VersionVector::new()).unwrap();
        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].ciphertext, b"x");
    }

    #[test]
    fn get_unknown_returns_not_found() {
        let s = InMemorySyncServer::new();
        assert!(matches!(s.get("ns", "k"), Err(SyncError::NotFound)));
    }

    #[test]
    fn server_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InMemorySyncServer>();
        assert_send_sync::<Box<dyn SyncServer>>();
    }

    // --- OR-set ---

    #[test]
    fn orset_add_then_contains() {
        let mut s = OrSet::new();
        s.add("work", &dev("A"), 1);
        assert!(s.contains("work"));
        assert_eq!(s.values(), vec!["work"]);
    }

    #[test]
    fn orset_remove_clears_observed_adds() {
        let mut s = OrSet::new();
        s.add("work", &dev("A"), 1);
        s.remove("work");
        assert!(!s.contains("work"));
    }

    #[test]
    fn orset_readd_after_remove_works() {
        let mut s = OrSet::new();
        s.add("work", &dev("A"), 1);
        s.remove("work");
        s.add("work", &dev("A"), 2);
        assert!(s.contains("work"));
    }

    #[test]
    fn orset_merge_unions_concurrent_adds() {
        // Device A adds "work", device B adds "personal" — the
        // canonical case where LWW would have lost a tag. OR-set
        // merge keeps both.
        let mut a = OrSet::new();
        a.add("work", &dev("A"), 1);
        let mut b = OrSet::new();
        b.add("personal", &dev("B"), 1);
        let m = a.merge(&b);
        assert!(m.contains("work"));
        assert!(m.contains("personal"));
        assert_eq!(m.values(), vec!["personal", "work"]);
    }

    #[test]
    fn orset_merge_idempotent_commutative() {
        let mut a = OrSet::new();
        a.add("work", &dev("A"), 1);
        let mut b = OrSet::new();
        b.add("personal", &dev("B"), 1);
        let m1 = a.merge(&b);
        let m2 = b.merge(&a);
        assert_eq!(m1, m2);
        let m3 = m1.merge(&m2);
        assert_eq!(m1, m3); // idempotent
    }

    #[test]
    fn orset_remove_propagates_via_merge() {
        // Device A adds "work", then device A removes "work".
        // Merging this with B's view (which never saw "work")
        // must end up not containing "work".
        let mut a = OrSet::new();
        a.add("work", &dev("A"), 1);
        a.remove("work");
        let b = OrSet::new();
        let m = a.merge(&b);
        assert!(!m.contains("work"));
    }

    // --- Concurrency ---

    #[test]
    fn concurrent_pushes_all_resolve() {
        use std::sync::Arc;
        use std::thread;
        let s = Arc::new(InMemorySyncServer::new());
        let mut handles = Vec::new();
        for i in 0..16 {
            let s = s.clone();
            handles.push(thread::spawn(move || {
                let dev = DeviceId::new(format!("d{}", i)).unwrap();
                let r = SyncRecord {
                    namespace: "n".into(),
                    key: format!("k{}", i),
                    version_vector: VersionVector::new().bump(&dev),
                    last_writer: dev,
                    last_writer_ms: 100 + i as u64,
                    ciphertext: format!("v{}", i).into_bytes(),
                    wrap_set: None,
                };
                s.push(r).unwrap();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        // All 16 should be present.
        let all = s.pull("n", &VersionVector::new()).unwrap();
        assert_eq!(all.len(), 16);
    }
}
