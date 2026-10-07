//! # Sealed secret records for the Chief of Staff vault (D18U)
//!
//! `ChiefVaultRuntime` keeps secrets in memory and decides, per secret, who may
//! lease them and how they may leave. `SealedStore` keeps opaque encrypted
//! bytes on disk. Until this crate, **nothing serialized between the two**: the
//! only code that could ever put a secret into the vault was a test. This crate
//! is that missing middle.
//!
//! ```text
//!   owner ──stdin──▶ ChiefSecretStore::put(name, policy, payload)
//!                          │  encode_record  (the D18U envelope)
//!                          ▼
//!                    SealedStore::put("chief-secrets", name, bytes)
//!                          │  XChaCha20-Poly1305, AAD = namespace ∥ 0 ∥ name
//!                          ▼
//!                        disk
//!
//!   daemon start ─▶ ChiefSecretStore::register_all(&runtime)
//!                          │  list → get → decode_record, ALL of them first
//!                          ▼
//!                    ChiefVaultRuntime::register_secret(name, payload, policy)
//! ```
//!
//! ## What protects a record
//!
//! Only the sealed store's AEAD (spec decision U-D1). Its additional data binds
//! every ciphertext to `namespace ∥ 0x00 ∥ key`, so a record for `bank` cannot
//! be renamed to `weather`, and any flipped bit fails the tag. Anyone who can
//! produce a record that verifies already holds the KEK — and so can already
//! read every secret in the vault. A second signature would guard a policy
//! against a party who has no reason to widen it.
//!
//! ## The envelope, version 1
//!
//! All integers big-endian. Every byte is accounted for; nothing may follow
//! the payload.
//!
//! ```text
//!  offset  field            encoding                         bound
//!  ──────  ───────────────  ───────────────────────────────  ────────────────
//!   0      magic            "CHIEFSEC"                       exact
//!   8      version          u8                               == 1
//!   9      privilege_tier   u8                               0..=3
//!  10      allowed_mode     u8  0 Direct │ 1 Leased │ 2 Both  exact
//!  11      rotated_at_ms    u64                              any
//!  19      agents tag       u8  0 Any │ 1 Only               exact
//!  20      (Only) count     u16                              1..=64
//!   …      (Only) each id   u32 len ∥ UTF-8                  1..=256, ascending
//!   …      payload          u32 len ∥ bytes                  1..=65 536
//! ```
//!
//! The policy comes **before** the payload (U-E2) so that a reader rejects a
//! bad policy before a single secret byte is copied out of the decrypted
//! plaintext — VLT06 P4, "refuse before materializing", applied to storage.
//!
//! ## Why decode is so strict
//!
//! A lenient decoder is a policy-widening machine. Read an unknown mode byte as
//! `Both` and a direct-only bank password becomes leasable. Accept agent ids in
//! any order and two encodings of one policy look identical to a person
//! comparing records but different to code. So every out-of-range value is an
//! error and never a "nearest" value (U-E1), ids must be strictly ascending
//! (U-E4), and an `Only` set that admits nobody is refused at both ends (U-E3),
//! because that is a provisioning mistake, not a way to disable a secret —
//! disabling is deleting.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::BTreeSet;
use std::fmt;

use chief_of_staff_vault_runtime::{
    AllowedAgents, ChiefVaultRuntime, SecretPolicy, VaultDeliveryMode,
};
use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_vault_sealed_store::{SealedStore, SealedStoreError};
use coding_adventures_zeroize::Zeroizing;
use storage_core::StorageListOptions;

// ── Constants ────────────────────────────────────────────────────────────────

/// The sealed-store namespace every Chief secret lives in.
pub const NAMESPACE: &str = "chief-secrets";

/// The eight bytes every version of the envelope starts with.
pub const MAGIC: &[u8; 8] = b"CHIEFSEC";

/// The only envelope version this crate reads or writes.
pub const VERSION: u8 = 1;

/// Highest `privilege_tier` VLT06 defines.
pub const MAX_PRIVILEGE_TIER: u8 = 3;

/// Most agent identities one `Only` policy may name.
pub const MAX_ALLOWED_AGENTS: usize = 64;

/// Longest agent identity, in bytes.
pub const MAX_AGENT_ID_BYTES: usize = 256;

/// Largest secret payload, in bytes.
pub const MAX_PAYLOAD_BYTES: usize = 64 * 1024;

/// Longest secret name, in bytes.
pub const MAX_SECRET_NAME_BYTES: usize = 128;

/// Most records the startup loader will read (U-L2).
pub const MAX_RECORDS: usize = 1024;

/// How many records the loader asks the backend for per page.
const LIST_PAGE_SIZE: usize = 128;

/// Bytes before the agents section: magic, version, tier, mode, rotated_at.
const FIXED_HEADER_BYTES: usize = MAGIC.len() + 1 + 1 + 1 + 8;

/// The largest envelope any valid record can produce.
///
/// The encoder reserves this much up front (U-E6). A `Vec` that grows by
/// reallocating copies its contents into a new allocation and frees the old
/// one *without* zeroing it — and the final zeroizing drop only reaches the
/// allocation the vector ends up in. Reserving the worst case once means
/// there is only ever one allocation holding the secret.
pub const MAX_RECORD_BYTES: usize = FIXED_HEADER_BYTES
    + 1
    + 2
    + MAX_ALLOWED_AGENTS * (4 + MAX_AGENT_ID_BYTES)
    + 4
    + MAX_PAYLOAD_BYTES;

// Wire values for the two tagged fields. Named so that the encoder and the
// decoder cannot drift apart on what `2` means.
const MODE_DIRECT: u8 = 0;
const MODE_LEASED: u8 = 1;
const MODE_BOTH: u8 = 2;
const AGENTS_ANY: u8 = 0;
const AGENTS_ONLY: u8 = 1;

// ── Secret names ─────────────────────────────────────────────────────────────

/// A validated secret name (spec rule U-N1).
///
/// The name is the sealed-store record key **verbatim**, and the filesystem
/// backend turns keys into paths. Rather than add an encoding layer, the
/// alphabet is narrowed until nothing path-shaped can get through:
///
/// | rule | rules out |
/// |---|---|
/// | 1–128 bytes | empty keys, names past common filename limits |
/// | `[a-z0-9]` first | leading `.` (hidden files), leading `-` (flag confusion) |
/// | then `[a-z0-9._-]` | `/`, `\`, NUL, spaces, upper case (case-folding collisions on macOS and Windows), all non-ASCII (confusables) |
/// | no `..` | parent-directory segments |
///
/// This is narrower than the D18D vault tools accept. That is the safe
/// direction: a name that cannot be stored cannot be leased, and the tool
/// returns its ordinary not-found denial.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecretName(String);

impl SecretName {
    /// Validate and wrap a secret name.
    pub fn parse(name: &str) -> Result<Self, NameError> {
        let bytes = name.as_bytes();
        let Some(&first) = bytes.first() else {
            return Err(NameError::Empty);
        };
        if bytes.len() > MAX_SECRET_NAME_BYTES {
            return Err(NameError::TooLong);
        }
        if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
            return Err(NameError::BadStart);
        }
        let allowed = |b: &u8| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        };
        if !bytes.iter().all(allowed) {
            return Err(NameError::InvalidCharacter);
        }
        if name.contains("..") {
            return Err(NameError::DotDot);
        }
        Ok(Self(name.to_string()))
    }

    /// The name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SecretName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a string is not a valid [`SecretName`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameError {
    /// The name is empty.
    Empty,
    /// The name is longer than [`MAX_SECRET_NAME_BYTES`].
    TooLong,
    /// The first byte is not a lower-case ASCII letter or digit.
    BadStart,
    /// A byte outside `[a-z0-9._-]`.
    InvalidCharacter,
    /// The name contains `..`.
    DotDot,
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "secret name is empty",
            Self::TooLong => "secret name is longer than 128 bytes",
            Self::BadStart => "secret name must start with a lower-case letter or digit",
            Self::InvalidCharacter => "secret name may contain only a-z, 0-9, '.', '_' and '-'",
            Self::DotDot => "secret name must not contain '..'",
        })
    }
}

impl std::error::Error for NameError {}

// ── Record errors ────────────────────────────────────────────────────────────

/// Why a record could not be encoded or decoded.
///
/// Every variant carries a static description of *which rule* failed and
/// nothing read from the record (U-E7). An agent id from a corrupt record is
/// attacker-influenced text, and a payload is the secret itself; neither has
/// any business in a log line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordError {
    /// The bytes are not a well-formed version-1 envelope.
    Malformed(&'static str),
    /// The policy cannot be stored as given.
    InvalidPolicy(&'static str),
    /// The payload is empty or larger than [`MAX_PAYLOAD_BYTES`].
    InvalidPayload(&'static str),
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(why) => write!(f, "malformed secret record: {why}"),
            Self::InvalidPolicy(why) => write!(f, "invalid secret policy: {why}"),
            Self::InvalidPayload(why) => write!(f, "invalid secret payload: {why}"),
        }
    }
}

impl std::error::Error for RecordError {}

// ── Encoding ─────────────────────────────────────────────────────────────────

/// Check a policy against the envelope's bounds without encoding anything.
///
/// The encoder calls this first, and so can a CLI that wants to reject bad
/// flags before it reads a secret from stdin.
pub fn validate_policy(policy: &SecretPolicy) -> Result<(), RecordError> {
    if policy.privilege_tier > MAX_PRIVILEGE_TIER {
        return Err(RecordError::InvalidPolicy("privilege_tier is above 3"));
    }
    if let AllowedAgents::Only(agents) = &policy.allowed_agents {
        if agents.is_empty() {
            return Err(RecordError::InvalidPolicy(
                "an allow-list that names no agent admits nobody",
            ));
        }
        if agents.len() > MAX_ALLOWED_AGENTS {
            return Err(RecordError::InvalidPolicy("more than 64 allowed agents"));
        }
        for agent in agents {
            validate_agent_id(agent).map_err(RecordError::InvalidPolicy)?;
        }
    }
    Ok(())
}

/// Shared by encode and decode, so a record this crate writes is always one
/// it will read back.
fn validate_agent_id(agent: &str) -> Result<(), &'static str> {
    if agent.is_empty() {
        return Err("an agent id is empty");
    }
    if agent.len() > MAX_AGENT_ID_BYTES {
        return Err("an agent id is longer than 256 bytes");
    }
    if agent.chars().any(|c| c.is_ascii_control()) {
        return Err("an agent id contains a control character");
    }
    Ok(())
}

fn validate_payload_len(len: usize) -> Result<(), RecordError> {
    if len == 0 {
        // U-E5: a failed or empty stdin read must not provision a secret that
        // leases successfully and authenticates nothing.
        return Err(RecordError::InvalidPayload("payload is empty"));
    }
    if len > MAX_PAYLOAD_BYTES {
        return Err(RecordError::InvalidPayload("payload is larger than 64 KiB"));
    }
    Ok(())
}

/// Encode one secret and its policy as a version-1 envelope.
///
/// The output is zeroizing and was allocated once, at [`MAX_RECORD_BYTES`].
pub fn encode_record(
    policy: &SecretPolicy,
    payload: &[u8],
) -> Result<Zeroizing<Vec<u8>>, RecordError> {
    validate_policy(policy)?;
    validate_payload_len(payload.len())?;

    let mut out = Zeroizing::new(Vec::with_capacity(MAX_RECORD_BYTES));
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(policy.privilege_tier);
    out.push(match policy.allowed_mode {
        VaultDeliveryMode::Direct => MODE_DIRECT,
        VaultDeliveryMode::Leased => MODE_LEASED,
        VaultDeliveryMode::Both => MODE_BOTH,
    });
    out.extend_from_slice(&policy.rotated_at_ms.to_be_bytes());
    match &policy.allowed_agents {
        AllowedAgents::Any => out.push(AGENTS_ANY),
        AllowedAgents::Only(agents) => {
            out.push(AGENTS_ONLY);
            // validate_policy bounded the count at 64, so this cannot truncate.
            let count = u16::try_from(agents.len())
                .map_err(|_| RecordError::InvalidPolicy("more than 64 allowed agents"))?;
            out.extend_from_slice(&count.to_be_bytes());
            // BTreeSet iterates in ascending byte order with no duplicates,
            // which is exactly the canonical order U-E4 requires.
            for agent in agents {
                write_bytes(&mut out, agent.as_bytes());
            }
        }
    }
    write_bytes(&mut out, payload);
    debug_assert!(out.len() <= MAX_RECORD_BYTES);
    Ok(out)
}

fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    // Every caller has bounded `bytes` far below u32::MAX.
    let len = u32::try_from(bytes.len()).expect("field length is bounded below u32::MAX");
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
}

// ── Decoding ─────────────────────────────────────────────────────────────────

/// A decoded record: the policy, and the secret in zeroizing storage.
pub struct DecodedRecord {
    /// The admission policy the owner provisioned.
    pub policy: SecretPolicy,
    /// The secret bytes.
    pub payload: LeasePayload,
}

impl fmt::Debug for DecodedRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // LeasePayload's own Debug already redacts; spelled out here too so
        // the struct is safe to log even if that ever changes.
        f.debug_struct("DecodedRecord")
            .field("policy", &self.policy)
            .field("payload", &"<redacted>")
            .finish()
    }
}

/// Decode a version-1 envelope. Total and closed: see rule U-E1.
pub fn decode_record(input: &[u8]) -> Result<DecodedRecord, RecordError> {
    if input.len() > MAX_RECORD_BYTES {
        return Err(RecordError::Malformed(
            "record is larger than any valid record",
        ));
    }
    let mut r = Reader { rest: input };
    if r.take(MAGIC.len())? != MAGIC {
        return Err(RecordError::Malformed("wrong magic"));
    }
    if r.byte()? != VERSION {
        return Err(RecordError::Malformed("unknown version"));
    }
    let privilege_tier = r.byte()?;
    if privilege_tier > MAX_PRIVILEGE_TIER {
        return Err(RecordError::Malformed("privilege_tier is above 3"));
    }
    let allowed_mode = match r.byte()? {
        MODE_DIRECT => VaultDeliveryMode::Direct,
        MODE_LEASED => VaultDeliveryMode::Leased,
        MODE_BOTH => VaultDeliveryMode::Both,
        _ => return Err(RecordError::Malformed("unknown delivery mode")),
    };
    let rotated_at_ms = r.u64()?;
    let allowed_agents = match r.byte()? {
        AGENTS_ANY => AllowedAgents::Any,
        AGENTS_ONLY => AllowedAgents::Only(read_agents(&mut r)?),
        _ => return Err(RecordError::Malformed("unknown allowed-agents tag")),
    };

    // The whole policy is now validated; only after that does a secret byte
    // move (U-E2).
    let len = r.length()?;
    if len == 0 || len > MAX_PAYLOAD_BYTES {
        return Err(RecordError::Malformed("payload length out of range"));
    }
    let bytes = r.take(len)?;
    if !r.rest.is_empty() {
        return Err(RecordError::Malformed("trailing bytes after payload"));
    }
    // One exact-capacity allocation, handed straight to the zeroizing payload.
    let mut owned = Vec::with_capacity(bytes.len());
    owned.extend_from_slice(bytes);
    Ok(DecodedRecord {
        policy: SecretPolicy {
            privilege_tier,
            allowed_agents,
            allowed_mode,
            rotated_at_ms,
        },
        payload: LeasePayload::new(owned),
    })
}

fn read_agents(r: &mut Reader<'_>) -> Result<BTreeSet<String>, RecordError> {
    let count = usize::from(r.u16()?);
    if count == 0 {
        return Err(RecordError::Malformed("allow-list names no agent"));
    }
    if count > MAX_ALLOWED_AGENTS {
        return Err(RecordError::Malformed("more than 64 allowed agents"));
    }
    let mut agents = BTreeSet::new();
    let mut previous: Option<&str> = None;
    for _ in 0..count {
        let len = r.length()?;
        if len > MAX_AGENT_ID_BYTES {
            return Err(RecordError::Malformed(
                "an agent id is longer than 256 bytes",
            ));
        }
        let agent = std::str::from_utf8(r.take(len)?)
            .map_err(|_| RecordError::Malformed("an agent id is not UTF-8"))?;
        validate_agent_id(agent).map_err(RecordError::Malformed)?;
        // Strictly ascending catches both reordering and duplicates (U-E4).
        if previous.is_some_and(|p| p >= agent) {
            return Err(RecordError::Malformed(
                "agent ids are not strictly ascending",
            ));
        }
        previous = Some(agent);
        agents.insert(agent.to_string());
    }
    Ok(agents)
}

/// A cursor over the envelope. Every read either succeeds in full or reports
/// truncation; nothing reads past the end.
struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], RecordError> {
        if n > self.rest.len() {
            return Err(RecordError::Malformed("record is truncated"));
        }
        let (head, tail) = self.rest.split_at(n);
        self.rest = tail;
        Ok(head)
    }

    fn byte(&mut self) -> Result<u8, RecordError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, RecordError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn u64(&mut self) -> Result<u64, RecordError> {
        let mut b = [0u8; 8];
        b.copy_from_slice(self.take(8)?);
        Ok(u64::from_be_bytes(b))
    }

    /// A `u32` length prefix, as a `usize`. On every target this repo builds
    /// for, `usize` is at least 32 bits, so the conversion cannot fail; the
    /// callers' bounds then cap it far lower.
    fn length(&mut self) -> Result<usize, RecordError> {
        let b = self.take(4)?;
        let len = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        usize::try_from(len).map_err(|_| RecordError::Malformed("length does not fit"))
    }
}

// ── The store ────────────────────────────────────────────────────────────────

/// Why a store operation failed.
#[derive(Debug)]
pub enum StoreError {
    /// The record could not be encoded (on `put`).
    Record(RecordError),
    /// A stored record failed to decode. Names the secret, never its bytes.
    CorruptRecord {
        /// The secret whose record is bad.
        name: SecretName,
        /// Which rule it broke.
        error: RecordError,
    },
    /// A record key in the namespace is not a valid secret name. The key
    /// itself is deliberately not reported: it was read from disk and passed
    /// no validation, so it is not safe to echo.
    UnnamedRecord,
    /// The namespace holds more than [`MAX_RECORDS`] records.
    TooManyRecords,
    /// The sealed store refused: sealed, tampered, or a backend failure.
    Sealed(SealedStoreError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Record(e) => write!(f, "chief secret store: {e}"),
            Self::CorruptRecord { name, error } => {
                write!(f, "chief secret store: record {name}: {error}")
            }
            Self::UnnamedRecord => {
                f.write_str("chief secret store: a record key is not a valid secret name")
            }
            Self::TooManyRecords => {
                f.write_str("chief secret store: more than 1024 records; refusing to load")
            }
            Self::Sealed(e) => write!(f, "chief secret store: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<SealedStoreError> for StoreError {
    fn from(e: SealedStoreError) -> Self {
        Self::Sealed(e)
    }
}

/// One secret read back from the store, ready to register.
#[derive(Debug)]
pub struct LoadedSecret {
    /// The secret's name.
    pub name: SecretName,
    /// Its policy and payload.
    pub record: DecodedRecord,
}

/// Chief secrets over an already-unsealed [`SealedStore`].
///
/// The store does not open or unseal anything itself: the daemon and the CLI
/// each already know how to read the owner-only KEK file and unseal, and
/// keeping that out of here leaves this crate with no filesystem authority.
pub struct ChiefSecretStore {
    sealed: SealedStore,
}

impl fmt::Debug for ChiefSecretStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChiefSecretStore(<redacted>)")
    }
}

impl ChiefSecretStore {
    /// Wrap an initialized, unsealed store.
    pub fn new(sealed: SealedStore) -> Self {
        Self { sealed }
    }

    /// Write (or overwrite) one secret.
    ///
    /// Overwriting is rotation. It takes effect in a running daemon only when
    /// that daemon restarts (U-D5), and the restart is what revokes every
    /// lease minted against the old value.
    pub fn put(
        &self,
        name: &SecretName,
        policy: &SecretPolicy,
        payload: &[u8],
    ) -> Result<(), StoreError> {
        let envelope = encode_record(policy, payload).map_err(StoreError::Record)?;
        self.sealed.put(NAMESPACE, name.as_str(), &envelope, None)?;
        Ok(())
    }

    /// Remove one secret. Removing a secret that is not there is not an error.
    pub fn delete(&self, name: &SecretName) -> Result<(), StoreError> {
        self.sealed.delete(NAMESPACE, name.as_str(), None)?;
        Ok(())
    }

    /// The names of every stored secret, without decrypting any of them.
    pub fn names(&self) -> Result<Vec<SecretName>, StoreError> {
        let mut names = Vec::new();
        let mut cursor = None;
        loop {
            let page = self.sealed.list(
                NAMESPACE,
                StorageListOptions {
                    prefix: None,
                    recursive: false,
                    page_size: Some(LIST_PAGE_SIZE),
                    cursor,
                },
            )?;
            // A store this crate wrote returns at most LIST_PAGE_SIZE per
            // page, so checking after each page bounds memory at
            // MAX_RECORDS + one page.
            for stat in &page {
                let name = SecretName::parse(&stat.key).map_err(|_| StoreError::UnnamedRecord)?;
                names.push(name);
            }
            if names.len() > MAX_RECORDS {
                return Err(StoreError::TooManyRecords);
            }
            match next_cursor(&page) {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        Ok(names)
    }

    /// Decrypt and decode every stored secret — all of them, or none (U-L1).
    ///
    /// A record that fails stops the load and is named in the error. It is
    /// never skipped: a silently missing secret surfaces only as a refused
    /// legitimate caller, far from its cause, and a vault someone has tampered
    /// with should stop the daemon rather than serve whatever survived.
    pub fn load_all(&self) -> Result<Vec<LoadedSecret>, StoreError> {
        let mut loaded = Vec::new();
        for name in self.names()? {
            let Some(sealed) = self.sealed.get(NAMESPACE, name.as_str())? else {
                // Deleted between list and get: it is genuinely gone, which is
                // the state the owner asked for.
                continue;
            };
            let record =
                decode_record(&sealed.plaintext).map_err(|error| StoreError::CorruptRecord {
                    name: name.clone(),
                    error,
                })?;
            loaded.push(LoadedSecret { name, record });
        }
        Ok(loaded)
    }

    /// Load every secret and register it into `runtime`. Returns how many.
    ///
    /// Everything is decoded **before** anything is registered, so a corrupt
    /// record leaves the runtime exactly as it was rather than half-populated.
    pub fn register_all(&self, runtime: &ChiefVaultRuntime) -> Result<usize, StoreError> {
        let loaded = self.load_all()?;
        let count = loaded.len();
        for LoadedSecret { name, record } in loaded {
            runtime.register_secret(name.as_str(), record.payload, record.policy);
        }
        Ok(count)
    }
}

/// `SealedStore::list` returns a flat `Vec` and drops the backend's
/// `next_cursor`, so the loader rebuilds it. That is sound because every
/// `storage-core` backend defines the cursor the same way — "skip keys `<=`
/// cursor", over keys in ascending order — which makes the last key of a full
/// page exactly the cursor the backend would have returned. A short page means
/// the listing is complete.
fn next_cursor(page: &[coding_adventures_vault_sealed_store::SealedStat]) -> Option<String> {
    if page.len() < LIST_PAGE_SIZE {
        return None;
    }
    page.last().map(|stat| stat.key.clone())
}
