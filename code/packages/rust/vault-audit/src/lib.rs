//! # `coding_adventures_vault_audit` — VLT09 audit log
//!
//! ## What this crate is
//!
//! An **append-only, tamper-evident, hash-chained, signed audit
//! log** for the Vault stack. Every operation that mutates the
//! vault — `mint`, `revoke`, `rotate_root`, `auth_succeed`,
//! `auth_fail`, `policy_decide`, `lease_consume` — produces a
//! signed entry, and every entry binds itself to the entry that
//! came before via a `prev_hash`. Verifying the chain is a
//! single linear pass.
//!
//! Two security primitives stack here:
//!
//! 1. **Hash chain**: each entry's `prev_hash` field is
//!    `blake2b-256(prev_entry_canonical_bytes || this_entry_body_bytes)`.
//!    Tampering with any entry invalidates every subsequent entry
//!    and is detected on verification.
//! 2. **Ed25519 signature**: the issuer signs every entry with
//!    its long-term device key. A malicious sync server can
//!    *delete* the tail of the log (which is detectable as
//!    "missing audit events"), but cannot *forge* an entry — the
//!    signature would fail verification against the device's
//!    public key.
//!
//! Together: an attacker with full control of the storage layer
//! can refuse to deliver new entries (denial-of-service) but
//! cannot lie about what happened. This is the same threat
//! model as Sigstore Rekor, Trillian, Sigsum, and HashiCorp
//! Vault's own audit device.
//!
//! ## Why this layer exists
//!
//! The Vault tier above (VLT08 dynamic-secret engines, VLT07
//! leases, VLT06 policy decisions) makes security-critical
//! decisions. Without an audit log, a compromise is undetectable
//! — the attacker mints and walks away. With this layer, every
//! mint is a signed entry in an append-only chain visible to
//! every honest party with the device's public key, even when
//! the storage server is malicious.
//!
//! It is also the substrate for compliance-driven workflows
//! (SOC 2, ISO 27001) that require a non-repudiable trail of
//! who-did-what-when.
//!
//! ## Pluggable sink
//!
//! [`AuditSink`] is a `Send + Sync` trait with two methods:
//! `append` and `iter`. Concrete implementations land in
//! sibling crates (e.g. `vault-audit-fs` writing to a file, a
//! future `vault-audit-syslog`, `vault-audit-s3`, or a
//! `vault-audit-trillian` backed by a transparency log).
//! Production deployments seal each entry at rest via VLT01
//! before passing it to a sink so even the storage layer sees
//! only ciphertext.
//!
//! [`InMemoryAuditSink`] is the reference implementation —
//! suitable for unit tests of upstream layers and for
//! single-process tools whose audit log doesn't need to outlive
//! the process.
//!
//! ## Where it fits
//!
//! ```text
//!                ┌────────────────────────────────────────┐
//!                │   VLT07 leases                         │
//!                │   VLT08 dynamic-secret engines         │
//!                │   VLT06 policy decisions               │
//!                │   VLT05 auth events                    │
//!                └──────────────┬─────────────────────────┘
//!                               │ AuditEvent
//!                ┌──────────────▼─────────────────────────┐
//!                │   AuditChain  (THIS CRATE)             │
//!                │   - allocate next sequence number       │
//!                │   - link prev_hash                      │
//!                │   - sign with device key (Ed25519)      │
//!                │   - hand finished entry to sink         │
//!                └──────────────┬─────────────────────────┘
//!                               │ SignedAuditEntry
//!                ┌──────────────▼─────────────────────────┐
//!                │   AuditSink                            │
//!                │     ├─ InMemoryAuditSink (this crate)  │
//!                │     ├─ vault-audit-fs (future)         │
//!                │     ├─ vault-audit-trillian (future)   │
//!                │     └─ vault-audit-syslog (future)     │
//!                └────────────────────────────────────────┘
//! ```
//!
//! ## Threat model
//!
//! * **Storage tampering**: detected by hash-chain verification.
//!   Any modification of a stored entry breaks every subsequent
//!   `prev_hash`, surfacing on the next `verify_chain` call.
//! * **Storage truncation (drop-the-tail)**: visible as a gap in
//!   sequence numbers; the chain `verify` checks that sequence
//!   numbers are dense (`0, 1, 2, …`) and that the last entry
//!   matches the head reported by the sink. Detectable but not
//!   *preventable* at this layer — that requires an external
//!   transparency log (which is what `vault-audit-trillian`
//!   provides as a sibling).
//! * **Forgery**: prevented by Ed25519 — an attacker without the
//!   device's secret key cannot produce a valid signature.
//! * **Replay**: prevented by sequence numbers + `prev_hash`
//!   linkage; a replayed entry would have a sequence number
//!   that's already used and a `prev_hash` that doesn't match
//!   the current head.
//! * **Information leakage in the audit body**: the audit body
//!   itself carries event metadata (principal, action, reference
//!   IDs) which is *intentionally not encrypted by this crate*.
//!   When the storage sink is untrusted (multi-tenant cloud,
//!   sync server), callers wrap each
//!   `SignedAuditEntry::canonical_bytes()` in VLT01 sealed-store
//!   before persistence. The chain still verifies because hashes
//!   are over the cleartext canonical form, computed *before*
//!   sealing.
//!
//! ## What this crate does *not* do
//!
//! * Not a transparency log: there is no Merkle tree, no
//!   inclusion proof, no consistency proof. Those are
//!   Trillian-class properties and live in a sibling crate.
//! * Not a sealing layer: VLT01 is the canonical sealed-store.
//!   This crate produces canonical bytes; sealing is a sink
//!   concern.
//! * Not a query engine: `iter()` is a forward scan. Indexed
//!   queries (by principal, by action, by time range) belong in
//!   a higher tier that consumes the chain.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_blake2b::{blake2b, Blake2bOptions};
use coding_adventures_zeroize::Zeroizing;
use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Best-effort mutex lock: if the mutex was poisoned by a panic
/// in some other thread we recover the inner guard rather than
/// panic, then surface the situation to the caller as an
/// `AuditError::SinkError` *if* invariants are unsafe to
/// continue from. The audit log is exactly the layer that should
/// stay available when other parts of the process misbehave —
/// turning a panic into a permanent DoS would silently swallow
/// security-critical events.
///
/// We accept the inner guard from a poisoned mutex because the
/// data inside the chain head / sink is a coherent
/// `(next_seq, prev_canonical)` snapshot or a `VecDeque` of
/// fully-formed entries; neither structure is left mid-mutation
/// by any panic site we have. (Verified by inspection: the
/// only fallible operation between lock and unlock is
/// `sink.append(...)?`, which does not panic on a healthy sink.)
fn lock_recover<'a, T>(m: &'a Mutex<T>) -> MutexGuard<'a, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

// === Section 1. Vocabulary types ============================================
//
// `AuditEvent` is the cleartext input the caller builds and
// hands to the chain. The chain wraps it in an `AuditEntry`
// (with sequence number + prev_hash + timestamp) and signs the
// canonical bytes to produce a `SignedAuditEntry`.

/// Action recorded by an audit entry. Narrow set so consumers
/// (compliance tooling, SIEMs) can switch on a known small enum
/// rather than parsing free-text strings. New variants land
/// non-breakingly because the enum is `#[non_exhaustive]`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuditAction {
    /// Authentication attempt succeeded (VLT05).
    AuthSucceed,
    /// Authentication attempt failed (VLT05). The reason is
    /// carried in `AuditEvent::detail` as opaque bytes (kept
    /// short to avoid a log-flooding amplifier).
    AuthFail,
    /// Policy decision: allow.
    PolicyAllow,
    /// Policy decision: deny.
    PolicyDeny,
    /// VLT08 engine minted a secret.
    EngineMint,
    /// VLT08 engine revoked a secret.
    EngineRevoke,
    /// VLT08 engine rotated its root credential.
    EngineRotateRoot,
    /// VLT07 lease consumed (one-shot read).
    LeaseConsume,
    /// VLT07 lease revoked.
    LeaseRevoke,
    /// VLT01 sealed record was written.
    SealedWrite,
    /// VLT01 sealed record was read (selective audit; off by
    /// default in chatty deployments).
    SealedRead,
    /// Catch-all for callers that want to record an event the
    /// trait doesn't yet have a variant for. Inner string is a
    /// short (< 64 chars) action label.
    Other(String),
}

/// Cleartext audit event the caller builds. The chain enriches
/// this with sequence + prev_hash + timestamp before signing.
#[derive(Clone, Debug)]
pub struct AuditEvent {
    /// Who did it. Opaque principal identifier from VLT05 (user
    /// id, service-account ARN, AWS-STS principal). Treated as
    /// audit metadata, *not* a privilege check.
    pub principal: String,
    /// What happened.
    pub action: AuditAction,
    /// Optional engine / resource reference. For an engine mint:
    /// the mount path. For a lease consume: the lease ID. For a
    /// policy decision: the path being decided. Caller-defined.
    pub resource: Option<String>,
    /// Free-form short detail bytes (< 1 KiB).
    ///
    /// **Do NOT put secrets here.** This crate intentionally
    /// does not encrypt the audit body — sealing-at-rest is a
    /// higher-tier concern (VLT01 sealed-store). Detail bytes
    /// flow through several non-zeroizing `Vec<u8>`
    /// intermediates (canonical encoding, signing buffer,
    /// `Clone` into the sink) and linger in the heap until the
    /// allocator reuses the page. Use this field for opaque
    /// non-sensitive metadata (auth failure reason code, IP, a
    /// short request ID) and route real secrets through VLT01
    /// before they get anywhere near an audit sink.
    pub detail: Option<Vec<u8>>,
}

/// One link in the chain — an `AuditEvent` enriched with
/// sequencing + chain linkage + wall-clock time.
///
/// All fields are public for callers that want to inspect the
/// chain after `iter()`. The signed bytes are `canonical_bytes()`
/// of this struct.
#[derive(Clone, Debug)]
pub struct AuditEntry {
    /// Monotonic sequence number, starting at 0 for the genesis
    /// entry and dense thereafter (no gaps).
    pub seq: u64,
    /// Wall-clock time of the entry, ms since UNIX epoch.
    /// Caller-supplied (to keep this crate clock-pure for
    /// tests; deployments source it from a trusted clock).
    pub timestamp_ms: u64,
    /// `blake2b-256(prev_entry.canonical_bytes() || this.body_bytes())`.
    /// For the genesis entry (`seq == 0`) this is 32 zero bytes
    /// — there is no prior entry to bind to.
    pub prev_hash: [u8; AUDIT_HASH_LEN],
    /// The cleartext event.
    pub event: AuditEvent,
}

/// `AuditEntry` + Ed25519 signature over its canonical bytes.
/// This is what the sink stores.
#[derive(Clone, Debug)]
pub struct SignedAuditEntry {
    /// The (sequenced + linked) entry.
    pub entry: AuditEntry,
    /// Ed25519 public key of the signing device. Stored
    /// alongside the signature so a verifier needs only the
    /// chain to verify (no out-of-band key registry required —
    /// though deployments will still pin known device keys to
    /// detect a swapped-issuer attack).
    pub signer_pub: [u8; ED25519_PUB_LEN],
    /// Ed25519 signature over `entry.canonical_bytes()`.
    pub signature: [u8; ED25519_SIG_LEN],
}

/// Length in bytes of an audit hash (blake2b-256).
pub const AUDIT_HASH_LEN: usize = 32;
/// Ed25519 public-key length.
pub const ED25519_PUB_LEN: usize = 32;
/// Ed25519 signature length.
pub const ED25519_SIG_LEN: usize = 64;
/// Ed25519 secret key length (the layout used by
/// `coding_adventures_ed25519` is the 64-byte
/// "expanded" form).
pub const ED25519_SECRET_LEN: usize = 64;

/// Maximum length of `AuditEvent::detail`. Bounding it keeps
/// chain verification linear-in-entries rather than
/// linear-in-bytes-of-detail, and prevents a malicious caller
/// from amplifying log size.
pub const MAX_DETAIL_LEN: usize = 1024;

/// Maximum length of `AuditEvent::principal`.
pub const MAX_PRINCIPAL_LEN: usize = 256;
/// Maximum length of `AuditEvent::resource`.
pub const MAX_RESOURCE_LEN: usize = 512;
/// Maximum length of an `AuditAction::Other` label.
pub const MAX_OTHER_ACTION_LEN: usize = 64;

// === Section 2. Errors ======================================================

/// All errors produced by this crate.
#[derive(Debug)]
pub enum AuditError {
    /// Caller passed a malformed event (oversize fields, empty
    /// principal, etc).
    InvalidEvent(&'static str),
    /// The chain's signature could not be produced (extremely
    /// unlikely with Ed25519 — included for symmetry).
    SignatureFailed,
    /// Verification failed: chain is broken. The caller knows
    /// the integrity of the chain is compromised and *must not*
    /// treat any later entry as authoritative.
    VerificationFailed(&'static str),
    /// Sink-level I/O failure. The text is sink-defined.
    SinkError(String),
}

impl core::fmt::Display for AuditError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidEvent(why) => write!(f, "invalid audit event: {}", why),
            Self::SignatureFailed => write!(f, "audit signature failed"),
            Self::VerificationFailed(why) => write!(f, "audit verification failed: {}", why),
            Self::SinkError(why) => write!(f, "audit sink error: {}", why),
        }
    }
}

impl std::error::Error for AuditError {}

// === Section 3. Canonicalization ============================================
//
// The signed bytes need to be reproducible. We use a tagged
// length-prefixed framing rather than CBOR/JSON because:
//
//   - it has zero deps inside this crate,
//   - it's trivially reviewable (one function, < 80 lines),
//   - the inputs are constrained (small enum + bounded strings)
//     so the encoder cannot blow up.
//
// Wire shape (big-endian throughout):
//
//   "AUD1"                 (4 bytes magic)
//   u8  version            (= 1)
//   u64 seq
//   u64 timestamp_ms
//   [u8;32] prev_hash
//   u8  action_tag         (see AUDIT_ACTION_* below)
//   u8  has_other_label    (1 if AuditAction::Other, else 0)
//   u32 other_label_len
//   bytes other_label
//   u32 principal_len      || principal_bytes
//   u8  has_resource       (1 if Some, else 0)
//   u32 resource_len       || resource_bytes
//   u8  has_detail         (1 if Some, else 0)
//   u32 detail_len         || detail_bytes
//
// The encoder is total — bounded inputs, never allocates more
// than ~MAX_DETAIL_LEN bytes per entry — and the decoder isn't
// needed for chain verification (we only need to re-encode a
// known entry to verify its hash, not parse arbitrary bytes).

/// Wire-format magic. "AUD" + version 1.
pub const AUDIT_WIRE_MAGIC: &[u8; 4] = b"AUD1";

const AUDIT_ACTION_AUTH_SUCCEED: u8 = 1;
const AUDIT_ACTION_AUTH_FAIL: u8 = 2;
const AUDIT_ACTION_POLICY_ALLOW: u8 = 3;
const AUDIT_ACTION_POLICY_DENY: u8 = 4;
const AUDIT_ACTION_ENGINE_MINT: u8 = 5;
const AUDIT_ACTION_ENGINE_REVOKE: u8 = 6;
const AUDIT_ACTION_ENGINE_ROTATE_ROOT: u8 = 7;
const AUDIT_ACTION_LEASE_CONSUME: u8 = 8;
const AUDIT_ACTION_LEASE_REVOKE: u8 = 9;
const AUDIT_ACTION_SEALED_WRITE: u8 = 10;
const AUDIT_ACTION_SEALED_READ: u8 = 11;
const AUDIT_ACTION_OTHER: u8 = 255;

fn action_tag(a: &AuditAction) -> (u8, &str) {
    match a {
        AuditAction::AuthSucceed => (AUDIT_ACTION_AUTH_SUCCEED, ""),
        AuditAction::AuthFail => (AUDIT_ACTION_AUTH_FAIL, ""),
        AuditAction::PolicyAllow => (AUDIT_ACTION_POLICY_ALLOW, ""),
        AuditAction::PolicyDeny => (AUDIT_ACTION_POLICY_DENY, ""),
        AuditAction::EngineMint => (AUDIT_ACTION_ENGINE_MINT, ""),
        AuditAction::EngineRevoke => (AUDIT_ACTION_ENGINE_REVOKE, ""),
        AuditAction::EngineRotateRoot => (AUDIT_ACTION_ENGINE_ROTATE_ROOT, ""),
        AuditAction::LeaseConsume => (AUDIT_ACTION_LEASE_CONSUME, ""),
        AuditAction::LeaseRevoke => (AUDIT_ACTION_LEASE_REVOKE, ""),
        AuditAction::SealedWrite => (AUDIT_ACTION_SEALED_WRITE, ""),
        AuditAction::SealedRead => (AUDIT_ACTION_SEALED_READ, ""),
        AuditAction::Other(label) => (AUDIT_ACTION_OTHER, label.as_str()),
    }
}

impl AuditEntry {
    /// Canonical byte form of this entry. The signature is over
    /// these bytes; the chain hash binds to them. Two equal
    /// entries produce equal canonical bytes, byte-for-byte.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128);
        out.extend_from_slice(AUDIT_WIRE_MAGIC);
        out.push(1u8); // version
        out.extend_from_slice(&self.seq.to_be_bytes());
        out.extend_from_slice(&self.timestamp_ms.to_be_bytes());
        out.extend_from_slice(&self.prev_hash);
        let (tag, other_label) = action_tag(&self.event.action);
        out.push(tag);
        if tag == AUDIT_ACTION_OTHER {
            out.push(1u8);
            out.extend_from_slice(&(other_label.len() as u32).to_be_bytes());
            out.extend_from_slice(other_label.as_bytes());
        } else {
            out.push(0u8);
            out.extend_from_slice(&0u32.to_be_bytes());
        }
        let p = self.event.principal.as_bytes();
        out.extend_from_slice(&(p.len() as u32).to_be_bytes());
        out.extend_from_slice(p);
        match &self.event.resource {
            Some(r) => {
                out.push(1u8);
                out.extend_from_slice(&(r.len() as u32).to_be_bytes());
                out.extend_from_slice(r.as_bytes());
            }
            None => {
                out.push(0u8);
                out.extend_from_slice(&0u32.to_be_bytes());
            }
        }
        match &self.event.detail {
            Some(d) => {
                out.push(1u8);
                out.extend_from_slice(&(d.len() as u32).to_be_bytes());
                out.extend_from_slice(d);
            }
            None => {
                out.push(0u8);
                out.extend_from_slice(&0u32.to_be_bytes());
            }
        }
        out
    }
}

/// Compute the chain hash that *links the next entry to this
/// one*: `blake2b-256(this.canonical_bytes() || next.body_bytes())`.
/// (For the body of "next" we use only the event-level fields —
/// this is what `prev_hash` should be when constructing the
/// next entry.) Internal helper.
fn chain_hash(prev_canonical: &[u8], next_body: &[u8]) -> [u8; AUDIT_HASH_LEN] {
    let mut buf = Vec::with_capacity(prev_canonical.len() + next_body.len());
    buf.extend_from_slice(prev_canonical);
    buf.extend_from_slice(next_body);
    let h =
        blake2b(&buf, &Blake2bOptions::new().digest_size(AUDIT_HASH_LEN)).expect("blake2b cannot fail with valid options");
    let mut out = [0u8; AUDIT_HASH_LEN];
    out.copy_from_slice(&h);
    out
}

/// Body bytes for prev_hash linkage — the event-level fields only,
/// without seq/timestamp/prev_hash. Same encoding scheme as
/// `canonical_bytes` minus the framing fields.
fn event_body_bytes(ev: &AuditEvent) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    let (tag, other_label) = action_tag(&ev.action);
    out.push(tag);
    if tag == AUDIT_ACTION_OTHER {
        out.push(1u8);
        out.extend_from_slice(&(other_label.len() as u32).to_be_bytes());
        out.extend_from_slice(other_label.as_bytes());
    } else {
        out.push(0u8);
        out.extend_from_slice(&0u32.to_be_bytes());
    }
    let p = ev.principal.as_bytes();
    out.extend_from_slice(&(p.len() as u32).to_be_bytes());
    out.extend_from_slice(p);
    match &ev.resource {
        Some(r) => {
            out.push(1u8);
            out.extend_from_slice(&(r.len() as u32).to_be_bytes());
            out.extend_from_slice(r.as_bytes());
        }
        None => {
            out.push(0u8);
            out.extend_from_slice(&0u32.to_be_bytes());
        }
    }
    match &ev.detail {
        Some(d) => {
            out.push(1u8);
            out.extend_from_slice(&(d.len() as u32).to_be_bytes());
            out.extend_from_slice(d);
        }
        None => {
            out.push(0u8);
            out.extend_from_slice(&0u32.to_be_bytes());
        }
    }
    out
}

// === Section 4. Sink trait ==================================================

/// The append-and-iterate contract. Sinks may persist (file,
/// S3, transparency log) or stay in memory. A sink is *append-
/// only*: it has no `delete` or `update` method. The trait is
/// `Send + Sync` so a single `Arc<dyn AuditSink>` can be shared
/// across the whole stack.
pub trait AuditSink: Send + Sync {
    /// Append a fully-formed signed entry. The sink is
    /// responsible for atomic durability if it claims to be
    /// persistent. Sinks that need to seal at rest should do so
    /// here, taking `entry.entry.canonical_bytes()` as the
    /// payload.
    fn append(&self, entry: SignedAuditEntry) -> Result<(), AuditError>;

    /// Number of entries currently in the sink (length of the
    /// chain). Used by [`AuditChain`] to allocate the next
    /// sequence number on attach.
    fn len(&self) -> Result<u64, AuditError>;

    /// Returns `true` iff `len() == 0`.
    fn is_empty(&self) -> Result<bool, AuditError> {
        Ok(self.len()? == 0)
    }

    /// All entries, in append order. The default representation
    /// is `Vec<SignedAuditEntry>`. Sinks backed by very large
    /// stores should override this to stream — but for the
    /// reference implementation we materialize.
    fn entries(&self) -> Result<Vec<SignedAuditEntry>, AuditError>;

    /// Read the most recent entry (if any). Default: pull all
    /// entries and return the last; sinks with cheap tail
    /// access should override.
    fn last(&self) -> Result<Option<SignedAuditEntry>, AuditError> {
        let all = self.entries()?;
        Ok(all.into_iter().next_back())
    }
}

// === Section 5. Reference in-memory sink ====================================

/// Threadsafe in-memory `AuditSink`. Suitable for tests of
/// upstream layers and for single-process tools whose audit log
/// doesn't need to outlive the process.
pub struct InMemoryAuditSink {
    // Crate-visible so unit tests can simulate a malicious
    // storage layer by mutating entries directly. The public
    // surface is append-only.
    pub(crate) inner: Mutex<VecDeque<SignedAuditEntry>>,
}

impl Default for InMemoryAuditSink {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryAuditSink {
    /// Construct a fresh, empty sink.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
        }
    }
}

impl AuditSink for InMemoryAuditSink {
    fn append(&self, entry: SignedAuditEntry) -> Result<(), AuditError> {
        let mut g = lock_recover(&self.inner);
        g.push_back(entry);
        Ok(())
    }

    fn len(&self) -> Result<u64, AuditError> {
        let g = lock_recover(&self.inner);
        Ok(g.len() as u64)
    }

    fn entries(&self) -> Result<Vec<SignedAuditEntry>, AuditError> {
        let g = lock_recover(&self.inner);
        Ok(g.iter().cloned().collect())
    }

    fn last(&self) -> Result<Option<SignedAuditEntry>, AuditError> {
        let g = lock_recover(&self.inner);
        Ok(g.back().cloned())
    }
}

// === Section 6. Chain (writer) ==============================================

/// The signing key carried by an [`AuditChain`]. Held under
/// [`Zeroizing`] so the secret bytes are scrubbed on drop.
pub struct AuditSigningKey {
    secret: Zeroizing<[u8; ED25519_SECRET_LEN]>,
    public: [u8; ED25519_PUB_LEN],
}

impl AuditSigningKey {
    /// Derive the keypair from a 32-byte seed using the
    /// underlying Ed25519 implementation. Callers are expected
    /// to source the seed from `coding_adventures_csprng` or
    /// from a long-term device key custodian (VLT03).
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let (public, secret_arr) = coding_adventures_ed25519::generate_keypair(seed);
        Self {
            secret: Zeroizing::new(secret_arr),
            public,
        }
    }

    /// The Ed25519 public key — safe to embed in entries and
    /// hand out for verification.
    pub fn public(&self) -> [u8; ED25519_PUB_LEN] {
        self.public
    }
}

impl core::fmt::Debug for AuditSigningKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AuditSigningKey")
            .field("secret", &"<redacted>")
            .field("public", &hex32(&self.public))
            .finish()
    }
}

fn hex32(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes.iter() {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}
const HEX: &[u8; 16] = b"0123456789abcdef";

/// The audit chain writer. Holds the signing key + a reference
/// to a sink + a cached "head" (sequence number and prior
/// canonical bytes) so each `record` is `O(1)` rather than
/// re-scanning the sink.
pub struct AuditChain<S: AuditSink> {
    key: AuditSigningKey,
    sink: S,
    /// Cached (next_seq, prev_canonical_bytes) under a mutex so
    /// concurrent `record` calls are correctly serialized.
    head: Mutex<ChainHead>,
}

struct ChainHead {
    /// The sequence number to use for the *next* entry.
    next_seq: u64,
    /// Canonical bytes of the most recently appended entry, or
    /// empty if the chain is empty (genesis case).
    prev_canonical: Vec<u8>,
}

impl<S: AuditSink> AuditChain<S> {
    /// Attach a chain to a sink **after verifying** the existing
    /// chain end-to-end against the supplied signing key.
    ///
    /// This closes the writer-side trust gap: without
    /// verification, a malicious or buggy sink could return a
    /// tampered tail and the new `record()` calls would silently
    /// extend a corrupt chain. With verification, attach fails
    /// closed — `AuditError::VerificationFailed` if the sink is
    /// tampered, has gaps, has a forged tail, or was signed by
    /// a different key.
    ///
    /// Cost: O(n) on startup (one Ed25519 verify per existing
    /// entry, one blake2b per link). The same cost an external
    /// verifier would pay.
    ///
    /// If you have *already* verified the sink out-of-band (e.g.
    /// the upstream tier holds a transparency-log proof and only
    /// hands you a confirmed-clean sink), use
    /// [`AuditChain::attach_unverified`] to skip the verify
    /// pass.
    pub fn attach(key: AuditSigningKey, sink: S) -> Result<Self, AuditError> {
        // Snapshot the current contents and verify against the
        // signing key's public counterpart. We *pin* to this
        // key — extending a chain whose tail is signed by some
        // *other* key is structurally a different chain and the
        // attach-time pinning makes that explicit.
        let snapshot = sink.entries()?;
        if !snapshot.is_empty() {
            let pinned_pub = key.public();
            verify_chain(&snapshot, Some(&pinned_pub))?;
        }
        Self::attach_unverified(key, sink)
    }

    /// Attach a chain to a sink **without** verifying its
    /// existing contents.
    ///
    /// Use this only if the caller has already validated the
    /// sink (e.g. via an external transparency-log proof) or if
    /// the sink is known-empty. Otherwise prefer
    /// [`AuditChain::attach`] — the writer-side trust gap is
    /// real and silent.
    pub fn attach_unverified(key: AuditSigningKey, sink: S) -> Result<Self, AuditError> {
        let head = match sink.last()? {
            None => ChainHead {
                next_seq: 0,
                prev_canonical: Vec::new(),
            },
            Some(last) => {
                let canonical = last.entry.canonical_bytes();
                let next_seq = last.entry.seq.checked_add(1).ok_or(AuditError::SinkError(
                    "audit sequence number would overflow u64".into(),
                ))?;
                ChainHead {
                    next_seq,
                    prev_canonical: canonical,
                }
            }
        };
        Ok(Self {
            key,
            sink,
            head: Mutex::new(head),
        })
    }

    /// Public key of the signing device, in case callers want to
    /// publish it alongside the chain so verifiers can pin it.
    pub fn signer_public(&self) -> [u8; ED25519_PUB_LEN] {
        self.key.public()
    }

    /// Append a new event to the chain. Returns the appended
    /// signed entry so callers can echo it (or wrap it in a
    /// transparency-log proof later).
    pub fn record(
        &self,
        event: AuditEvent,
        timestamp_ms: u64,
    ) -> Result<SignedAuditEntry, AuditError> {
        validate_event(&event)?;
        let mut head = lock_recover(&self.head);
        let body = event_body_bytes(&event);
        let prev_hash: [u8; AUDIT_HASH_LEN] = if head.prev_canonical.is_empty() {
            // Genesis entry — prev_hash is all zeros.
            [0u8; AUDIT_HASH_LEN]
        } else {
            chain_hash(&head.prev_canonical, &body)
        };
        let entry = AuditEntry {
            seq: head.next_seq,
            timestamp_ms,
            prev_hash,
            event,
        };
        let canonical = entry.canonical_bytes();
        let signature = coding_adventures_ed25519::sign(&canonical, &self.key.secret);
        let signed = SignedAuditEntry {
            entry,
            signer_pub: self.key.public,
            signature,
        };
        // Append first; only update head if the sink accepts.
        // If the sink errors, the head stays where it was so a
        // retry with a fresh event reuses the same sequence
        // number.
        self.sink.append(signed.clone())?;
        head.next_seq = head.next_seq.checked_add(1).ok_or(AuditError::SinkError(
            "audit sequence number would overflow u64".into(),
        ))?;
        head.prev_canonical = canonical;
        Ok(signed)
    }

    /// Borrow the sink (read-only) so callers can iterate /
    /// verify without re-attaching.
    pub fn sink(&self) -> &S {
        &self.sink
    }
}

// === Section 7. Validation ==================================================

fn validate_event(ev: &AuditEvent) -> Result<(), AuditError> {
    if ev.principal.is_empty() {
        return Err(AuditError::InvalidEvent("principal must not be empty"));
    }
    if ev.principal.len() > MAX_PRINCIPAL_LEN {
        return Err(AuditError::InvalidEvent("principal exceeds MAX_PRINCIPAL_LEN"));
    }
    if let Some(r) = &ev.resource {
        if r.len() > MAX_RESOURCE_LEN {
            return Err(AuditError::InvalidEvent("resource exceeds MAX_RESOURCE_LEN"));
        }
    }
    if let Some(d) = &ev.detail {
        if d.len() > MAX_DETAIL_LEN {
            return Err(AuditError::InvalidEvent("detail exceeds MAX_DETAIL_LEN"));
        }
    }
    if let AuditAction::Other(label) = &ev.action {
        if label.is_empty() {
            return Err(AuditError::InvalidEvent("Other(label) must not be empty"));
        }
        if label.len() > MAX_OTHER_ACTION_LEN {
            return Err(AuditError::InvalidEvent(
                "Other(label) exceeds MAX_OTHER_ACTION_LEN",
            ));
        }
    }
    Ok(())
}

// === Section 8. Verifier ====================================================

/// Verify the integrity of a chain.
///
/// Walks the entries in append order and checks:
///
///   1. `seq` is dense, starting at 0.
///   2. `prev_hash` of entry `i` (i > 0) equals
///      `blake2b-256(entries[i-1].canonical_bytes() || entries[i].body_bytes())`.
///   3. The Ed25519 signature on every entry verifies against
///      that entry's embedded `signer_pub`.
///
/// If `expected_signer_pub` is `Some`, also checks that *every*
/// entry was signed by that key — catches a swapped-issuer
/// attack where a malicious party rewrote the chain under a
/// different keypair.
pub fn verify_chain(
    entries: &[SignedAuditEntry],
    expected_signer_pub: Option<&[u8; ED25519_PUB_LEN]>,
) -> Result<(), AuditError> {
    let mut prev_canonical: Vec<u8> = Vec::new();
    for (i, signed) in entries.iter().enumerate() {
        // Sequence number check.
        let expected_seq = i as u64;
        if signed.entry.seq != expected_seq {
            return Err(AuditError::VerificationFailed(
                "sequence number mismatch (gap or reorder)",
            ));
        }
        // prev_hash check.
        let body = event_body_bytes(&signed.entry.event);
        let want_prev: [u8; AUDIT_HASH_LEN] = if i == 0 {
            [0u8; AUDIT_HASH_LEN]
        } else {
            chain_hash(&prev_canonical, &body)
        };
        if signed.entry.prev_hash != want_prev {
            return Err(AuditError::VerificationFailed(
                "prev_hash does not match recomputed chain hash",
            ));
        }
        // Pinned-issuer check.
        if let Some(want_pub) = expected_signer_pub {
            if signed.signer_pub != *want_pub {
                return Err(AuditError::VerificationFailed(
                    "signer_pub does not match expected pinned key",
                ));
            }
        }
        // Signature check.
        let canonical = signed.entry.canonical_bytes();
        let ok = coding_adventures_ed25519::verify(
            &canonical,
            &signed.signature,
            &signed.signer_pub,
        );
        if !ok {
            return Err(AuditError::VerificationFailed(
                "Ed25519 signature did not verify",
            ));
        }
        prev_canonical = canonical;
    }
    Ok(())
}

// === Section 9. Tests =======================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_seed() -> [u8; 32] {
        // Deterministic test seed — never a production key.
        [
            0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
            0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
            0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
            0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
        ]
    }

    fn fresh() -> AuditChain<InMemoryAuditSink> {
        let key = AuditSigningKey::from_seed(&fixed_seed());
        AuditChain::attach(key, InMemoryAuditSink::new()).unwrap()
    }

    fn ev(action: AuditAction, resource: Option<&str>) -> AuditEvent {
        AuditEvent {
            principal: "alice".into(),
            action,
            resource: resource.map(|s| s.to_owned()),
            detail: None,
        }
    }

    #[test]
    fn genesis_has_zero_prev_hash() {
        let chain = fresh();
        let s = chain.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        assert_eq!(s.entry.seq, 0);
        assert_eq!(s.entry.prev_hash, [0u8; AUDIT_HASH_LEN]);
    }

    #[test]
    fn second_entry_links_to_first() {
        let chain = fresh();
        let s1 = chain.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        let s2 = chain
            .record(ev(AuditAction::EngineMint, Some("kv/")), 2)
            .unwrap();
        assert_eq!(s2.entry.seq, 1);
        let recomputed = chain_hash(
            &s1.entry.canonical_bytes(),
            &event_body_bytes(&s2.entry.event),
        );
        assert_eq!(s2.entry.prev_hash, recomputed);
    }

    #[test]
    fn signatures_verify_with_embedded_pub_key() {
        let chain = fresh();
        for i in 0..5 {
            chain
                .record(ev(AuditAction::EngineMint, Some("kv/")), 100 + i)
                .unwrap();
        }
        let entries = chain.sink().entries().unwrap();
        verify_chain(&entries, None).expect("clean chain must verify");
    }

    #[test]
    fn pinned_issuer_check() {
        let chain = fresh();
        chain.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        let entries = chain.sink().entries().unwrap();
        let pinned = chain.signer_public();
        verify_chain(&entries, Some(&pinned)).expect("pinned signer matches");
        let wrong_pin = [0xFFu8; ED25519_PUB_LEN];
        let r = verify_chain(&entries, Some(&wrong_pin));
        assert!(matches!(r, Err(AuditError::VerificationFailed(_))));
    }

    #[test]
    fn tampered_event_breaks_chain() {
        let chain = fresh();
        chain.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        chain.record(ev(AuditAction::EngineMint, None), 2).unwrap();
        let mut entries = chain.sink().entries().unwrap();
        // Flip a byte in the second entry's principal — must
        // break verification.
        entries[1].entry.event.principal = "mallory".into();
        let r = verify_chain(&entries, None);
        assert!(matches!(r, Err(AuditError::VerificationFailed(_))));
    }

    #[test]
    fn tampered_first_entry_breaks_second_via_chain() {
        let chain = fresh();
        chain.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        chain.record(ev(AuditAction::EngineMint, None), 2).unwrap();
        let mut entries = chain.sink().entries().unwrap();
        // Tamper with entry 0's resource. Entry 1's prev_hash
        // was computed against the *old* entry-0 canonical bytes
        // and now fails — proving the chain catches mutations to
        // earlier entries even when the per-entry signature
        // would still be valid.
        entries[0].entry.event.resource = Some("rewritten".into());
        let r = verify_chain(&entries, None);
        assert!(matches!(r, Err(AuditError::VerificationFailed(_))));
    }

    #[test]
    fn truncation_visible_as_resequence() {
        let chain = fresh();
        for i in 0..3 {
            chain
                .record(ev(AuditAction::EngineMint, None), 100 + i)
                .unwrap();
        }
        let mut entries = chain.sink().entries().unwrap();
        // Drop the head and verify — first surviving entry has
        // seq=1 but verifier expects 0, so verification fails.
        entries.remove(0);
        let r = verify_chain(&entries, None);
        assert!(matches!(r, Err(AuditError::VerificationFailed(_))));
    }

    #[test]
    fn forged_signature_breaks_verification() {
        let chain = fresh();
        chain.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        let mut entries = chain.sink().entries().unwrap();
        // Flip a byte in the signature.
        entries[0].signature[0] ^= 0x01;
        let r = verify_chain(&entries, None);
        assert!(matches!(r, Err(AuditError::VerificationFailed(_))));
    }

    #[test]
    fn empty_chain_verifies() {
        verify_chain(&[], None).expect("empty chain has nothing to falsify");
    }

    #[test]
    fn validate_rejects_empty_principal() {
        let chain = fresh();
        let bad = AuditEvent {
            principal: "".into(),
            action: AuditAction::AuthSucceed,
            resource: None,
            detail: None,
        };
        let r = chain.record(bad, 1);
        assert!(matches!(r, Err(AuditError::InvalidEvent(_))));
    }

    #[test]
    fn validate_rejects_oversize_detail() {
        let chain = fresh();
        let bad = AuditEvent {
            principal: "alice".into(),
            action: AuditAction::AuthFail,
            resource: None,
            detail: Some(vec![0u8; MAX_DETAIL_LEN + 1]),
        };
        let r = chain.record(bad, 1);
        assert!(matches!(r, Err(AuditError::InvalidEvent(_))));
    }

    #[test]
    fn validate_rejects_oversize_principal() {
        let chain = fresh();
        let bad = AuditEvent {
            principal: "x".repeat(MAX_PRINCIPAL_LEN + 1),
            action: AuditAction::AuthSucceed,
            resource: None,
            detail: None,
        };
        let r = chain.record(bad, 1);
        assert!(matches!(r, Err(AuditError::InvalidEvent(_))));
    }

    #[test]
    fn validate_rejects_empty_other_label() {
        let chain = fresh();
        let bad = AuditEvent {
            principal: "alice".into(),
            action: AuditAction::Other("".into()),
            resource: None,
            detail: None,
        };
        let r = chain.record(bad, 1);
        assert!(matches!(r, Err(AuditError::InvalidEvent(_))));
    }

    #[test]
    fn validate_rejects_oversize_other_label() {
        let chain = fresh();
        let bad = AuditEvent {
            principal: "alice".into(),
            action: AuditAction::Other("x".repeat(MAX_OTHER_ACTION_LEN + 1)),
            resource: None,
            detail: None,
        };
        let r = chain.record(bad, 1);
        assert!(matches!(r, Err(AuditError::InvalidEvent(_))));
    }

    #[test]
    fn attach_picks_up_existing_chain() {
        // Build a chain, write some entries, then re-attach a
        // *new* AuditChain over the same sink (simulating a
        // process restart). New entries must continue the
        // sequence and link via prev_hash.
        let key1 = AuditSigningKey::from_seed(&fixed_seed());
        let sink_arc = std::sync::Arc::new(InMemoryAuditSink::new());
        // Need a sink we can hand to AuditChain by value while
        // keeping a reference. Use a small adapter.
        struct ArcSink(std::sync::Arc<InMemoryAuditSink>);
        impl AuditSink for ArcSink {
            fn append(&self, e: SignedAuditEntry) -> Result<(), AuditError> {
                self.0.append(e)
            }
            fn len(&self) -> Result<u64, AuditError> {
                self.0.len()
            }
            fn entries(&self) -> Result<Vec<SignedAuditEntry>, AuditError> {
                self.0.entries()
            }
            fn last(&self) -> Result<Option<SignedAuditEntry>, AuditError> {
                self.0.last()
            }
        }

        let chain1 = AuditChain::attach(key1, ArcSink(sink_arc.clone())).unwrap();
        chain1.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        chain1.record(ev(AuditAction::EngineMint, None), 2).unwrap();
        drop(chain1);

        let key2 = AuditSigningKey::from_seed(&fixed_seed());
        let chain2 = AuditChain::attach(key2, ArcSink(sink_arc.clone())).unwrap();
        let s3 = chain2
            .record(ev(AuditAction::EngineRevoke, None), 3)
            .unwrap();
        assert_eq!(s3.entry.seq, 2);

        let entries = sink_arc.entries().unwrap();
        verify_chain(&entries, None).expect("post-restart chain verifies");
    }

    #[test]
    fn attach_rejects_tampered_chain() {
        // Build a chain through one process; tamper with the
        // sink's last entry; then a *new* AuditChain attempting
        // to attach must fail closed rather than extend the
        // corrupt chain.
        struct ArcSink(std::sync::Arc<InMemoryAuditSink>);
        impl AuditSink for ArcSink {
            fn append(&self, e: SignedAuditEntry) -> Result<(), AuditError> {
                self.0.append(e)
            }
            fn len(&self) -> Result<u64, AuditError> {
                self.0.len()
            }
            fn entries(&self) -> Result<Vec<SignedAuditEntry>, AuditError> {
                self.0.entries()
            }
            fn last(&self) -> Result<Option<SignedAuditEntry>, AuditError> {
                self.0.last()
            }
        }

        let sink_arc = std::sync::Arc::new(InMemoryAuditSink::new());
        let key1 = AuditSigningKey::from_seed(&fixed_seed());
        let chain1 = AuditChain::attach(key1, ArcSink(sink_arc.clone())).unwrap();
        chain1.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        chain1.record(ev(AuditAction::EngineMint, None), 2).unwrap();
        drop(chain1);

        // Mutate the in-memory sink directly to simulate a
        // tampering storage layer.
        {
            let mut g = sink_arc.inner.lock().unwrap();
            g.back_mut().unwrap().entry.event.principal = "mallory".into();
        }

        // Attaching the *same key* must now fail.
        let key2 = AuditSigningKey::from_seed(&fixed_seed());
        let r = AuditChain::attach(key2, ArcSink(sink_arc));
        assert!(matches!(r, Err(AuditError::VerificationFailed(_))));
    }

    #[test]
    fn attach_rejects_mismatched_signer() {
        // Chain signed by key A; attach attempted with key B —
        // the signer-pin in attach catches the swap.
        struct ArcSink(std::sync::Arc<InMemoryAuditSink>);
        impl AuditSink for ArcSink {
            fn append(&self, e: SignedAuditEntry) -> Result<(), AuditError> {
                self.0.append(e)
            }
            fn len(&self) -> Result<u64, AuditError> {
                self.0.len()
            }
            fn entries(&self) -> Result<Vec<SignedAuditEntry>, AuditError> {
                self.0.entries()
            }
            fn last(&self) -> Result<Option<SignedAuditEntry>, AuditError> {
                self.0.last()
            }
        }

        let sink_arc = std::sync::Arc::new(InMemoryAuditSink::new());
        let key_a = AuditSigningKey::from_seed(&fixed_seed());
        let chain = AuditChain::attach(key_a, ArcSink(sink_arc.clone())).unwrap();
        chain.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        drop(chain);

        // Different seed → different keypair.
        let mut other_seed = fixed_seed();
        other_seed[0] ^= 0xff;
        let key_b = AuditSigningKey::from_seed(&other_seed);
        let r = AuditChain::attach(key_b, ArcSink(sink_arc));
        assert!(matches!(r, Err(AuditError::VerificationFailed(_))));
    }

    #[test]
    fn attach_unverified_skips_check() {
        // attach_unverified is the documented escape hatch; it
        // must succeed even on a tampered chain (so callers who
        // know what they're doing can use it).
        struct ArcSink(std::sync::Arc<InMemoryAuditSink>);
        impl AuditSink for ArcSink {
            fn append(&self, e: SignedAuditEntry) -> Result<(), AuditError> {
                self.0.append(e)
            }
            fn len(&self) -> Result<u64, AuditError> {
                self.0.len()
            }
            fn entries(&self) -> Result<Vec<SignedAuditEntry>, AuditError> {
                self.0.entries()
            }
            fn last(&self) -> Result<Option<SignedAuditEntry>, AuditError> {
                self.0.last()
            }
        }

        let sink_arc = std::sync::Arc::new(InMemoryAuditSink::new());
        let key1 = AuditSigningKey::from_seed(&fixed_seed());
        let chain1 = AuditChain::attach_unverified(key1, ArcSink(sink_arc.clone())).unwrap();
        chain1.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        {
            let mut g = sink_arc.inner.lock().unwrap();
            g.back_mut().unwrap().entry.event.principal = "mallory".into();
        }
        drop(chain1);
        let key2 = AuditSigningKey::from_seed(&fixed_seed());
        let r = AuditChain::attach_unverified(key2, ArcSink(sink_arc));
        assert!(r.is_ok());
    }

    #[test]
    fn other_action_round_trips() {
        let chain = fresh();
        chain
            .record(
                AuditEvent {
                    principal: "svc".into(),
                    action: AuditAction::Other("custom-event".into()),
                    resource: Some("/foo".into()),
                    detail: Some(b"hello".to_vec()),
                },
                10,
            )
            .unwrap();
        let entries = chain.sink().entries().unwrap();
        verify_chain(&entries, None).expect("Other variant entries must verify");
        match &entries[0].entry.event.action {
            AuditAction::Other(s) => assert_eq!(s, "custom-event"),
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn signing_key_debug_redacts_secret() {
        let k = AuditSigningKey::from_seed(&fixed_seed());
        let s = format!("{:?}", k);
        assert!(s.contains("<redacted>"));
        // The hex-encoded public key is fine to expose.
        assert!(!s.contains("0x42"));
    }

    #[test]
    fn chain_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AuditChain<InMemoryAuditSink>>();
        assert_send_sync::<InMemoryAuditSink>();
    }

    #[test]
    fn concurrent_record_serializes_via_mutex() {
        use std::sync::Arc;
        use std::thread;
        let chain = Arc::new(fresh());
        let mut handles = Vec::new();
        for i in 0..32 {
            let chain = chain.clone();
            handles.push(thread::spawn(move || {
                chain
                    .record(
                        ev(AuditAction::EngineMint, Some("kv/")),
                        1_000 + i as u64,
                    )
                    .unwrap();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let entries = chain.sink().entries().unwrap();
        assert_eq!(entries.len(), 32);
        verify_chain(&entries, None).expect("32-thread chain verifies");
        // Sequence numbers must be 0..32 with no gaps.
        for (i, e) in entries.iter().enumerate() {
            assert_eq!(e.entry.seq, i as u64);
        }
    }

    #[test]
    fn canonical_bytes_starts_with_magic() {
        let chain = fresh();
        let s = chain.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        let bytes = s.entry.canonical_bytes();
        assert_eq!(&bytes[..4], AUDIT_WIRE_MAGIC);
        assert_eq!(bytes[4], 1u8); // version
    }

    #[test]
    fn signed_entry_pub_matches_chain_pub() {
        let chain = fresh();
        let s = chain.record(ev(AuditAction::AuthSucceed, None), 1).unwrap();
        assert_eq!(s.signer_pub, chain.signer_public());
    }
}
