//! # vault-sealed-store — at-rest envelope encryption
//!
//! This crate implements **VLT01**: the Vault's at-rest encryption layer.
//! It wraps any `storage_core::StorageBackend` and exposes a
//! *sealed secrets store*.
//!
//! See `code/specs/VLT01-vault-sealed-store.md` for the full spec. A
//! short summary:
//!
//! ```text
//!        put(plaintext) ─┐                      ┌─> wrapped_dek (meta)
//!                        │                      │
//!                        ▼                      │
//!                   DEK ← CSPRNG(32)            │
//!                        │                      │
//!          ┌─ AEAD_xchacha20poly1305(DEK, nonce,│ aad) ─┐
//!          │                                    │      │
//!     plaintext                             wrap DEK   ▼
//!          │                                    │   ciphertext
//!          ▼                                    ▼
//!     StorageRecord.body   metadata = { nonces, tags, kek_id, wrapped_dek }
//! ```
//!
//! The **master KEK** lives in RAM only while the store is *unsealed*.
//! It is either derived from an operator password via Argon2id or injected
//! after a caller-owned custody ceremony, verified against a known-plaintext
//! verifier stored in the manifest, and wiped on `seal()` or drop.

#![forbid(unsafe_code)]

mod anchor;
mod freshness;

pub use anchor::{AnchorError, FileFreshnessAnchor, FreshnessAnchor};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use freshness::{accepts, Entry, EntryState, FreshnessIndex, MAX_ENTRIES, NO_TAG, TAG_BYTES};

use coding_adventures_argon2id::{argon2id, Options as Argon2Options, VERSION as ARGON2_VERSION};
use coding_adventures_bounded_json::{JsonNumber, JsonValue};
use coding_adventures_chacha20_poly1305::{
    xchacha20_poly1305_aead_decrypt, xchacha20_poly1305_aead_encrypt,
};
use coding_adventures_csprng::{random_array, random_bytes};
use coding_adventures_ct_compare::ct_eq;
use coding_adventures_zeroize::Zeroizing;
use storage_core::{Revision, StorageBackend, StorageError, StorageListOptions, StoragePutInput};

// ---------------------------------------------------------------------------
// Constants — on-disk format markers.
// ---------------------------------------------------------------------------

/// Reserved namespace for manifest and other vault-internal records. Writes
/// from external callers into this namespace are rejected.
pub const RESERVED_NAMESPACE: &str = "__vault__";

/// Fixed key for the singleton manifest record.
pub const MANIFEST_KEY: &str = "manifest";

/// Fixed key for the namespace-registry side record.
const NAMESPACES_KEY: &str = "namespaces";

/// Content-type tag on the manifest record.
pub const MANIFEST_CONTENT_TYPE: &str = "application/vault-manifest+json-v1";

/// Content-type tag on every sealed record.
pub const SEALED_CONTENT_TYPE: &str = "application/vault-sealed+json-v1";

/// Content-type tag on the namespace-registry record.
const NAMESPACES_CONTENT_TYPE: &str = "application/vault-namespaces+json-v1";

/// Manifest schema version. Increments on any breaking on-disk change.
const MANIFEST_VERSION: u64 = 1;

/// Sealed record schema version written today. Version 2 carries a
/// `generation` bound into the body AAD (VLT01 F2).
const SEALED_RECORD_VERSION: u64 = 2;

/// The original record format: no generation. Still read (F3), never written
/// for a record, and still used for the freshness index's own envelope, which
/// is versioned by its content type instead.
const SEALED_RECORD_VERSION_1: u64 = 1;

/// Key prefix, under the reserved namespace, of each namespace's freshness
/// index (F1).
const FRESHNESS_KEY_PREFIX: &str = "freshness/";

/// Content-type tag on a freshness index record (F1).
const FRESHNESS_CONTENT_TYPE: &str = "application/vault-freshness-v1";

/// Domain separator for the freshness index's body AAD (F1).
const FRESHNESS_AAD_DOMAIN: &[u8] = b"vault-freshness-v1";

/// How many times an index compare-and-swap is retried before giving up (F8).
const INDEX_CAS_ATTEMPTS: usize = 8;

/// The 16 zero bytes that the verifier AEADs under the KEK. Chosen over
/// hashing the KEK because a known-plaintext verifier cannot leak the KEK.
const VERIFIER_PLAINTEXT: [u8; 16] = [0u8; 16];

/// Default KDF tuning — matches the RFC 9106 §4 "uniformly safe" profile.
pub const DEFAULT_ARGON2_TIME_COST: u32 = 3;
pub const DEFAULT_ARGON2_MEMORY_KIB: u32 = 65_536;
pub const DEFAULT_ARGON2_PARALLELISM: u32 = 4;
pub const DEFAULT_ARGON2_SALT_LEN: usize = 16;

/// Hard upper bounds for Argon2id parameters as read from the persisted
/// manifest. These are defence against a tampered manifest: an attacker
/// who can rewrite the at-rest bytes can otherwise force unseal into an
/// O(hours) / O(TiB) KDF run and DoS the vault.
///
/// The ceilings chosen here are far above any legitimate operator
/// tuning (5 GiB, 10 passes, 64 lanes) but still cap the blast radius.
const ARGON2_TIME_COST_MAX: u32 = 10;
const ARGON2_MEMORY_KIB_MAX: u32 = 5 * 1024 * 1024;
const ARGON2_PARALLELISM_MAX: u32 = 64;
const ARGON2_SALT_MIN_LEN: usize = 8;
const ARGON2_SALT_MAX_LEN: usize = 1024;

/// XChaCha20-Poly1305 nonce + tag lengths. Hard-coded because we commit to
/// a single AEAD suite in v1 of the format.
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;

// ---------------------------------------------------------------------------
// Public surface.
// ---------------------------------------------------------------------------

/// Tunable parameters for `init()`.
///
/// All fields except the salt map 1-to-1 onto Argon2id parameters. Callers
/// who want interactive-login latency can lower `time_cost` / `memory_kib`;
/// callers who want maximum resistance can raise them. The chosen values are
/// persisted in the manifest so future unseals use the same profile.
#[derive(Debug, Clone)]
pub struct InitOptions {
    pub argon2id_time_cost: u32,
    pub argon2id_memory_kib: u32,
    pub argon2id_parallelism: u32,
    /// If `None`, a `DEFAULT_ARGON2_SALT_LEN`-byte salt is drawn from CSPRNG.
    /// Callers who pass their own salt must make it ≥ 8 bytes (Argon2 spec).
    pub salt_override: Option<Vec<u8>>,
}

impl Default for InitOptions {
    fn default() -> Self {
        Self {
            argon2id_time_cost: DEFAULT_ARGON2_TIME_COST,
            argon2id_memory_kib: DEFAULT_ARGON2_MEMORY_KIB,
            argon2id_parallelism: DEFAULT_ARGON2_PARALLELISM,
            salt_override: None,
        }
    }
}

/// A decrypted record. The plaintext lives inside a `Zeroizing` wrapper so
/// dropping the struct wipes it.
pub struct SealedRecord {
    pub namespace: String,
    pub key: String,
    pub revision: Revision,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub plaintext: Zeroizing<Vec<u8>>,
}

/// One page of [`SealedStore::list_page`]: the records, and where to resume.
#[derive(Debug, Clone)]
pub struct SealedPage {
    /// The records on this page.
    pub records: Vec<SealedStat>,
    /// Pass back as `StorageListOptions::cursor` to fetch the next page.
    /// `None` means the listing is complete.
    pub next_cursor: Option<String>,
}

/// A lightweight view that avoids touching the AEAD / KEK at all. Useful for
/// listings, auditing, and selective fetching.
#[derive(Debug, Clone)]
pub struct SealedStat {
    pub namespace: String,
    pub key: String,
    pub revision: Revision,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub ciphertext_len: usize,
    pub kek_id: String,
}

/// Redacted envelope metadata for one sealed record.
///
/// This carries only stable identifiers, timestamps, algorithms, and byte
/// counts. It deliberately omits ciphertext, wrapped DEKs, nonces, tags, and
/// AAD bytes so audit and host-read paths can inspect shape without copying
/// sealed material into logs.
#[derive(Debug, Clone)]
pub struct SealedEnvelopeSummary {
    pub namespace: String,
    pub key: String,
    pub revision: Revision,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub schema_version: u64,
    pub aead: String,
    pub ciphertext_len: usize,
    pub body_nonce_len: usize,
    pub body_tag_len: usize,
    pub body_aad_len: usize,
    pub wrapped_dek_len: usize,
    pub wrap_nonce_len: usize,
    pub wrap_tag_len: usize,
    pub kek_id: String,
}

/// Result of a completed `rotate_kek` call. Useful for tests and telemetry.
#[derive(Debug, Clone)]
pub struct KekRotationReport {
    pub new_kek_id: String,
    pub records_rewrapped: usize,
    pub records_already_new: usize,
}

/// Read-side vault status that does not require decrypting any record body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedStoreStatus {
    pub initialized: bool,
    pub sealed: bool,
    pub active_kek_id: Option<String>,
    pub kek_entries: usize,
    pub retired_keks: usize,
    pub registered_namespaces: usize,
}

/// Error variants surfaced by the sealed store. We deliberately collapse
/// low-level crypto failures into a single `Crypto` variant so the error
/// string can stay attacker-invisible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealedStoreError {
    /// `init()` called when a manifest already exists at the reserved address.
    AlreadyInitialized,
    /// Any data-plane operation called before `init()` / before a manifest exists.
    NotInitialized,
    /// Data-plane operation called while sealed.
    Sealed,
    /// Unseal failed because the derived KEK could not decrypt the verifier.
    BadPassword,
    /// An injected KEK could not decrypt any injected entry's verifier.
    InvalidKek,
    /// A persisted AEAD tag or AAD check failed — record was tampered with.
    Tamper { namespace: String, key: String },
    /// Backend returned an error.
    Storage(StorageError),
    /// A crypto primitive rejected its inputs. Message is intentionally vague.
    Crypto(String),
    /// Caller input violated a surface-level contract. Message strings here
    /// are always produced from static literals in this crate — never from
    /// untrusted on-disk bytes — so they cannot carry attacker payloads.
    Validation { field: String, message: String },
}

impl std::fmt::Display for SealedStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized => write!(f, "vault-sealed-store already initialized"),
            Self::NotInitialized => write!(f, "vault-sealed-store not initialized"),
            Self::Sealed => write!(f, "vault-sealed-store is sealed"),
            Self::BadPassword => write!(f, "vault-sealed-store unseal: bad password"),
            Self::InvalidKek => write!(f, "vault-sealed-store unseal: invalid injected KEK"),
            Self::Tamper { namespace, key } => {
                write!(
                    f,
                    "vault-sealed-store tamper detected for {namespace}/{key}"
                )
            }
            Self::Storage(e) => write!(f, "vault-sealed-store storage error: {e}"),
            Self::Crypto(m) => write!(f, "vault-sealed-store crypto error: {m}"),
            Self::Validation { field, message } => {
                write!(
                    f,
                    "vault-sealed-store validation failed for {field}: {message}"
                )
            }
        }
    }
}

impl std::error::Error for SealedStoreError {}

impl From<StorageError> for SealedStoreError {
    fn from(e: StorageError) -> Self {
        SealedStoreError::Storage(e)
    }
}

// ---------------------------------------------------------------------------
// Implementation.
// ---------------------------------------------------------------------------

/// The top-level facade. Cheaply cloneable via `Arc` — wraps a shared
/// backend plus a mutex-guarded in-memory KEK slot.
pub struct SealedStore {
    backend: Arc<dyn StorageBackend>,
    state: Mutex<State>,
    /// Decoded freshness indexes, by namespace, with the storage revision
    /// they were read at.
    ///
    /// Only an index that passed its AEAD is ever cached, so a cache hit can
    /// only ever return an index this process authenticated. If someone puts
    /// back an old index file but keeps the cached revision string, the hit
    /// returns the newer, authentic index, which is the safe direction. A
    /// legitimate write always changes the revision, so it always misses.
    indexes: Mutex<HashMap<String, (Revision, FreshnessIndex)>>,
    /// The external epoch floor (F11), if this store was given one.
    anchor: Option<Arc<dyn FreshnessAnchor>>,
}

/// In-memory unseal state. Held under a mutex so `seal()` from one thread
/// wipes out reads-in-flight on another deterministically.
struct State {
    /// The current active KEK, if unsealed.
    unsealed: Option<UnsealedKey>,
}

/// Zeroizing wrapper around the 32-byte KEK plus its stable id.
struct UnsealedKey {
    id: String,
    key: Zeroizing<[u8; KEY_LEN]>,
}

impl Drop for State {
    fn drop(&mut self) {
        // Zeroizing handles the actual wipe.
        self.unsealed = None;
    }
}

impl SealedStore {
    /// Wrap a backend. Does no I/O; caller is expected to have already
    /// called `StorageBackend::initialize` if required.
    pub fn new(backend: Arc<dyn StorageBackend>) -> Self {
        Self {
            backend,
            state: Mutex::new(State { unsealed: None }),
            indexes: Mutex::new(HashMap::new()),
            anchor: None,
        }
    }

    /// Wrap a backend, with a freshness anchor kept outside it (VLT01 F11).
    ///
    /// The anchor remembers, per namespace, the highest index epoch this
    /// vault wrote. An index older than that, or a missing index the anchor
    /// remembers, is `Tamper`. That closes the restored-pair cases the
    /// index alone cannot see (F10), across restarts, provided the anchor's
    /// location is outside the attacker's reach.
    pub fn with_anchor(backend: Arc<dyn StorageBackend>, anchor: Arc<dyn FreshnessAnchor>) -> Self {
        Self {
            anchor: Some(anchor),
            ..Self::new(backend)
        }
    }

    /// The anchored epoch for `namespace`, or `None` without an anchor.
    fn anchored_epoch(&self, namespace: &str) -> Result<Option<u64>, SealedStoreError> {
        match &self.anchor {
            None => Ok(None),
            Some(anchor) => anchor.load(namespace).map_err(anchor_error),
        }
    }

    /// Is the store currently sealed (no KEK in RAM)?
    pub fn is_sealed(&self) -> bool {
        self.state
            .lock()
            .expect("vault state mutex poisoned")
            .unsealed
            .is_none()
    }

    /// Return a sealed-safe status summary for host health checks.
    ///
    /// The summary reads only the manifest and namespace registry metadata:
    /// it never unwraps a DEK, derives a KEK, or decrypts a record body.
    pub fn status(&self) -> Result<SealedStoreStatus, SealedStoreError> {
        let sealed = self.is_sealed();
        let Some(manifest_record) = self.backend.get(RESERVED_NAMESPACE, MANIFEST_KEY)? else {
            return Ok(SealedStoreStatus {
                initialized: false,
                sealed,
                active_kek_id: None,
                kek_entries: 0,
                retired_keks: 0,
                registered_namespaces: 0,
            });
        };

        let manifest = Manifest::parse(&manifest_record.metadata)?;
        let active_kek_id = manifest
            .keks
            .iter()
            .find(|entry| entry.status == "active")
            .map(|entry| entry.id.clone());
        let retired_keks = manifest
            .keks
            .iter()
            .filter(|entry| entry.status == "retired")
            .count();

        Ok(SealedStoreStatus {
            initialized: true,
            sealed,
            active_kek_id,
            kek_entries: manifest.keks.len(),
            retired_keks,
            registered_namespaces: self.list_registered_namespaces()?.len(),
        })
    }

    /// Wipe the KEK from memory. Idempotent — calling on an already-sealed
    /// store is a no-op.
    pub fn seal(&self) {
        self.state
            .lock()
            .expect("vault state mutex poisoned")
            .unsealed = None;
    }

    // ---- initialize / unseal ----------------------------------------------

    /// Create a fresh vault. Writes a new manifest containing the KDF
    /// parameters, a random salt, and a verifier AEAD'd under the derived
    /// KEK. Fails if a manifest already exists.
    ///
    /// Note on TOCTOU: the absence check + write is not atomic at the
    /// backend level. Two concurrent `init()` calls racing on the same
    /// backend may both succeed at the absence check; the second write
    /// will overwrite the first. In practice `init()` runs once per
    /// machine setup; the documented invariant is that callers must not
    /// race it.
    pub fn init(&self, password: &[u8], opts: &InitOptions) -> Result<(), SealedStoreError> {
        // 1. Fail fast if a manifest already exists.
        if self
            .backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)?
            .is_some()
        {
            return Err(SealedStoreError::AlreadyInitialized);
        }

        // 2. Validate caller-supplied KDF parameters. Nothing here involves
        //    untrusted on-disk bytes yet — we just clamp to the same ceiling
        //    we use when parsing a persisted manifest, so init() and unseal()
        //    have the same concept of "legal".
        validate_argon2_params(
            opts.argon2id_time_cost,
            opts.argon2id_memory_kib,
            opts.argon2id_parallelism,
        )?;

        // 3. Collect salt (caller-supplied or CSPRNG).
        let salt = match &opts.salt_override {
            Some(s) => {
                if s.len() < ARGON2_SALT_MIN_LEN || s.len() > ARGON2_SALT_MAX_LEN {
                    return Err(SealedStoreError::Validation {
                        field: "salt_override".to_string(),
                        message: "salt length out of range".to_string(),
                    });
                }
                s.clone()
            }
            None => random_bytes(DEFAULT_ARGON2_SALT_LEN)
                .map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?,
        };

        // 4. Derive the initial KEK. `derive_kek` returns the key already
        //    wrapped in Zeroizing, so any `?` below wipes on the way out.
        let kek = derive_kek(
            password,
            &salt,
            opts.argon2id_time_cost,
            opts.argon2id_memory_kib,
            opts.argon2id_parallelism,
        )?;

        self.commit_initial_kek(
            kek,
            KekSource::PasswordDerived,
            Some(salt),
            opts.argon2id_time_cost,
            opts.argon2id_memory_kib,
            opts.argon2id_parallelism,
        )
    }

    /// Create a fresh vault around a caller-supplied random 32-byte KEK.
    ///
    /// The raw KEK is never persisted. Callers should generate it with a
    /// CSPRNG, persist only a custodian-wrapped copy, and drop their copy
    /// immediately after this call returns.
    pub fn init_with_kek(&self, kek: &[u8; KEY_LEN]) -> Result<(), SealedStoreError> {
        if self
            .backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)?
            .is_some()
        {
            return Err(SealedStoreError::AlreadyInitialized);
        }

        let mut owned = Zeroizing::new([0u8; KEY_LEN]);
        owned.copy_from_slice(kek);
        self.commit_initial_kek(
            owned,
            KekSource::Injected,
            None,
            DEFAULT_ARGON2_TIME_COST,
            DEFAULT_ARGON2_MEMORY_KIB,
            DEFAULT_ARGON2_PARALLELISM,
        )
    }

    fn commit_initial_kek(
        &self,
        kek: Zeroizing<[u8; KEY_LEN]>,
        source: KekSource,
        salt: Option<Vec<u8>>,
        time_cost: u32,
        memory_kib: u32,
        parallelism: u32,
    ) -> Result<(), SealedStoreError> {
        // Produce the verifier (known-plaintext AEAD under the KEK).
        let verifier_nonce: [u8; NONCE_LEN] =
            random_array().map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?;
        let (verifier_ct, verifier_tag) = xchacha20_poly1305_aead_encrypt(
            &VERIFIER_PLAINTEXT,
            &kek,
            &verifier_nonce,
            b"vault-verifier",
        );

        // Assemble and persist the manifest.
        let kek_id = "kek-1".to_string();
        let manifest = build_manifest_json(
            MANIFEST_VERSION,
            time_cost,
            memory_kib,
            parallelism,
            &[KekEntry {
                id: kek_id.clone(),
                status: "active",
                source,
                salt,
                verifier_nonce,
                verifier_tag,
                verifier_ct,
            }],
            now_ms_from_wallclock(),
        );

        let put = StoragePutInput::new(
            RESERVED_NAMESPACE.to_string(),
            MANIFEST_KEY.to_string(),
            MANIFEST_CONTENT_TYPE.to_string(),
            manifest,
            Vec::new(),
        )
        .map_err(SealedStoreError::Storage)?;

        self.backend.put(put)?;

        // Install the KEK in memory. We move the `Zeroizing<[u8;32]>`
        //    directly into the state so no extra stack copy is ever created.
        self.state
            .lock()
            .expect("vault state mutex poisoned")
            .unsealed = Some(UnsealedKey {
            id: kek_id,
            key: kek,
        });

        Ok(())
    }

    /// Load the manifest, derive a candidate KEK against each KEK entry's
    /// own salt, and verify against the manifest's verifier. On success
    /// holds the KEK in RAM.
    pub fn unseal(&self, password: &[u8]) -> Result<(), SealedStoreError> {
        let manifest_record = self
            .backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)?
            .ok_or(SealedStoreError::NotInitialized)?;

        let manifest = Manifest::parse(&manifest_record.metadata)?;

        // Walk active → retired KEKs; stop at the first that verifies. This
        // supports mid-rotation states where the active entry has not yet
        // been switched to the new password, and also supports recovery
        // under an old password if a rotation crashed mid-flight.
        //
        // NB: each entry has its own salt, so we re-derive per entry. That's
        // O(len(keks)) Argon2 runs per bad unseal attempt — acceptable for
        // any realistic key history (≤ a handful of entries).
        for entry in &manifest.keks {
            if entry.source != KekSource::PasswordDerived {
                continue;
            }
            let salt = entry
                .salt
                .as_ref()
                .ok_or_else(|| SealedStoreError::Validation {
                    field: "salt".to_string(),
                    message: "missing for password-derived KEK".to_string(),
                })?;
            // Derive under *this* entry's salt.
            let candidate = derive_kek(
                password,
                salt,
                manifest.time_cost,
                manifest.memory_kib,
                manifest.parallelism,
            )?;
            let decrypted = xchacha20_poly1305_aead_decrypt(
                &entry.verifier_ct,
                &candidate,
                &entry.verifier_nonce,
                b"vault-verifier",
                &entry.verifier_tag,
            );
            // Constant-time compare defensively. If the AEAD produced a
            // cleartext (Some), XChaCha20-Poly1305's tag check already
            // authenticates it, so the value equality *should* be
            // cryptographically implied — but we still route it through
            // `ct_eq` rather than `==` so the review trail is consistent
            // with the spec's "constant-time compares" guarantee.
            let bytes = decrypted.as_deref().unwrap_or(&[]);
            if ct_eq(bytes, &VERIFIER_PLAINTEXT) {
                // Move the matching KEK into state.
                self.state
                    .lock()
                    .expect("vault state mutex poisoned")
                    .unsealed = Some(UnsealedKey {
                    id: entry.id.clone(),
                    key: candidate,
                });
                return Ok(());
            }
            // `candidate` falls out of scope here and Zeroizing wipes it.
        }
        Err(SealedStoreError::BadPassword)
    }

    /// Verify and load a caller-supplied injected KEK.
    ///
    /// Password-derived entries are deliberately ignored so callers cannot
    /// accidentally bypass the custody path on a password-backed manifest.
    pub fn unseal_with_kek(&self, kek: &[u8; KEY_LEN]) -> Result<(), SealedStoreError> {
        let manifest_record = self
            .backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)?
            .ok_or(SealedStoreError::NotInitialized)?;
        let manifest = Manifest::parse(&manifest_record.metadata)?;

        let mut candidate = Zeroizing::new([0u8; KEY_LEN]);
        candidate.copy_from_slice(kek);
        for entry in &manifest.keks {
            if entry.source != KekSource::Injected {
                continue;
            }
            let decrypted = xchacha20_poly1305_aead_decrypt(
                &entry.verifier_ct,
                &candidate,
                &entry.verifier_nonce,
                b"vault-verifier",
                &entry.verifier_tag,
            );
            if ct_eq(decrypted.as_deref().unwrap_or(&[]), &VERIFIER_PLAINTEXT) {
                self.state
                    .lock()
                    .expect("vault state mutex poisoned")
                    .unsealed = Some(UnsealedKey {
                    id: entry.id.clone(),
                    key: candidate,
                });
                return Ok(());
            }
        }
        Err(SealedStoreError::InvalidKek)
    }

    // ---- data plane -------------------------------------------------------

    /// Encrypt and write one record. The reserved namespace is rejected.
    pub fn put(
        &self,
        namespace: &str,
        key: &str,
        plaintext: &[u8],
        if_revision: Option<Revision>,
    ) -> Result<Revision, SealedStoreError> {
        self.put_with_condition(namespace, key, plaintext, if_revision, false)
    }

    /// Encrypt and create one record only when the key is absent.
    pub fn put_if_absent(
        &self,
        namespace: &str,
        key: &str,
        plaintext: &[u8],
    ) -> Result<Revision, SealedStoreError> {
        self.put_with_condition(namespace, key, plaintext, None, true)
    }

    fn put_with_condition(
        &self,
        namespace: &str,
        key: &str,
        plaintext: &[u8],
        if_revision: Option<Revision>,
        if_absent: bool,
    ) -> Result<Revision, SealedStoreError> {
        check_external_namespace(namespace)?;

        // Must be unsealed before we do *any* side-effect — otherwise an
        // unauthenticated caller holding a handle to a sealed store could
        // bloat the namespace registry indefinitely by retrying puts
        // against fresh namespaces.
        {
            let guard = self.state.lock().expect("vault state mutex poisoned");
            if guard.unsealed.is_none() {
                return Err(SealedStoreError::Sealed);
            }
        }

        // Register the namespace (outside the state lock, so this does not
        // starve other ops). `register_namespace` is idempotent; if the
        // namespace is already known it returns immediately with no write.
        self.register_namespace(namespace)?;

        // The state lock is held for the whole put, index update included.
        // That is what serializes this process's index writes (F8).
        let guard = self.state.lock().expect("vault state mutex poisoned");
        let unsealed = guard.unsealed.as_ref().ok_or(SealedStoreError::Sealed)?;

        // F4: the next generation is one past whatever the index remembers,
        // live, tombstoned or legacy. An absent entry counts as zero.
        let current = self.ready_index(unsealed, namespace)?;
        let remembered = current.0.entries.get(key).map(|entry| entry.generation);
        if remembered.is_none() && current.0.entries.len() >= MAX_ENTRIES {
            // Refused before the record is written: a record the index can
            // never hold would be accepted forever as "ahead of the index".
            return Err(SealedStoreError::Validation {
                field: "freshness".to_string(),
                message: "namespace index is full".to_string(),
            });
        }
        // Bounded to i64::MAX because metadata stores it as a JSON integer.
        // A generation that cannot be written exactly must not be written
        // at all, or the metadata and the AAD would disagree.
        let generation = remembered
            .unwrap_or(0)
            .checked_add(1)
            .filter(|generation| i64::try_from(*generation).is_ok())
            .ok_or_else(|| SealedStoreError::Validation {
                field: "generation".to_string(),
                message: "exhausted".to_string(),
            })?;

        let aad = record_aad_v2(namespace, key, generation);
        let (metadata, ciphertext, tag) =
            seal_envelope(unsealed, namespace, key, &aad, Some(generation), plaintext)?;
        let mut put_in = StoragePutInput::new(
            namespace.to_string(),
            key.to_string(),
            SEALED_CONTENT_TYPE.to_string(),
            metadata,
            ciphertext,
        )
        .map_err(SealedStoreError::Storage)?;
        put_in = if if_absent {
            put_in.with_if_absent()
        } else {
            put_in.with_if_revision(if_revision)
        };

        // Record first, then index. A crash in between leaves the record
        // ahead of the index, which F3 accepts.
        let rec = self.backend.put(put_in)?;
        let key_owned = key.to_string();
        self.update_index(unsealed, namespace, current, |index| {
            let entry = index.entries.entry(key_owned.clone()).or_insert(Entry {
                generation: 0,
                state: EntryState::Live,
                tag: NO_TAG,
            });
            if entry.generation == generation && entry.tag != tag {
                // Another writer recorded a different record at this same
                // generation while we wrote ours. Pinning either one would
                // leave the other's file ambiguous, so this put reports a
                // conflict and the caller retries.
                return Err(SealedStoreError::Storage(StorageError::Conflict {
                    namespace: namespace.to_string(),
                    key: key_owned.clone(),
                    expected_revision: None,
                    actual_revision: None,
                }));
            }
            // Never lower a generation. If a racing delete already
            // tombstoned past us, the delete happened later and stands.
            if entry.generation <= generation {
                *entry = Entry {
                    generation,
                    state: EntryState::Live,
                    tag,
                };
            }
            Ok(())
        })?;
        Ok(rec.revision)
    }

    /// Read, decrypt, and return one record (or `None`).
    pub fn get(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<SealedRecord>, SealedStoreError> {
        check_external_namespace(namespace)?;

        let guard = self.state.lock().expect("vault state mutex poisoned");
        let unsealed = guard.unsealed.as_ref().ok_or(SealedStoreError::Sealed)?;

        let record = match self.backend.get(namespace, key)? {
            Some(r) => r,
            None => return Ok(None),
        };

        let sealed = SealedRecordMeta::parse(&record.metadata)?;

        // F3: refuse a record the freshness index says is stale, before
        // spending any crypto on it.
        let index = self.load_index(unsealed, namespace)?;
        let entry = index
            .as_ref()
            .and_then(|(index, _)| index.entries.get(key).copied());
        let shape = sealed
            .generation
            .map(|generation| (generation, sealed.body_tag));
        if !accepts(index.is_some(), entry, shape) {
            return Err(tamper(namespace, key));
        }

        // The generation is read from plaintext metadata, so it is trusted
        // only because it is part of the AAD the AEAD checks below.
        let expected_aad = match sealed.generation {
            None => record_aad(namespace, key),
            Some(generation) => record_aad_v2(namespace, key, generation),
        };
        let plaintext = open_envelope(
            unsealed,
            namespace,
            key,
            &sealed,
            &expected_aad,
            &record.body,
        )?;

        // `dek` drops here, wiping the cleartext DEK bytes.
        Ok(Some(SealedRecord {
            namespace: record.namespace,
            key: record.key,
            revision: record.revision,
            created_at_ms: record.created_at,
            updated_at_ms: record.updated_at,
            plaintext,
        }))
    }

    /// Delete a record. Reserved namespace is rejected. Backend is free to
    /// succeed silently on missing records.
    pub fn delete(
        &self,
        namespace: &str,
        key: &str,
        if_revision: Option<Revision>,
    ) -> Result<(), SealedStoreError> {
        check_external_namespace(namespace)?;
        // Deletion does not decrypt the record, but it does require unseal.
        // Otherwise a sealed vault could be used as a "destroy records"
        // oracle without proving knowledge of the password. Now it also
        // writes the sealed index, which needs the KEK anyway.
        let guard = self.state.lock().expect("vault state mutex poisoned");
        let unsealed = guard.unsealed.as_ref().ok_or(SealedStoreError::Sealed)?;

        let current = self.ready_index(unsealed, namespace)?;
        let entry = current.0.entries.get(key).copied();
        let record = self.backend.get(namespace, key)?;
        // Check the caller's revision before touching the index. A delete
        // that is going to fail must not leave a tombstone that hides the
        // record it failed to delete.
        if let (Some(expected), Some(record)) = (&if_revision, &record) {
            if record.revision != *expected {
                return Err(SealedStoreError::Storage(StorageError::Conflict {
                    namespace: namespace.to_string(),
                    key: key.to_string(),
                    expected_revision: Some(expected.to_string()),
                    actual_revision: Some(record.revision.to_string()),
                }));
            }
        }
        // F5: the tombstone is one past the reconciled generation `n`, and it
        // is written even when nothing is on disk or the key is unknown.
        //
        // One past, because a put whose index update was lost leaves an
        // authentic record at `n + 1` that reconcile can only absorb if it
        // can see it. Someone who hides that file during this delete and puts
        // it back afterwards would otherwise resurrect it. Every put writes
        // the reconciled `n + 1` and entries never go down, so no authentic
        // record is ever above `n + 1`, and a tombstone there covers them all.
        // It is never taken from the record's own plaintext metadata: a forged
        // generation there could otherwise push it to `i64::MAX` and destroy
        // the key.
        let tombstone = entry
            .map_or(0, |entry| entry.generation)
            .checked_add(1)
            .filter(|generation| i64::try_from(*generation).is_ok())
            .ok_or_else(|| SealedStoreError::Validation {
                field: "generation".to_string(),
                message: "exhausted".to_string(),
            })?;
        let key_owned = key.to_string();
        // Tombstone first, then delete. A crash in between leaves a record
        // at or below the tombstone, which F3 refuses.
        self.update_index(unsealed, namespace, current, |index| {
            let existing = index.entries.get(&key_owned).map_or(0, |e| e.generation);
            index.entries.insert(
                key_owned.clone(),
                Entry {
                    generation: existing.max(tombstone),
                    state: EntryState::Tombstone,
                    tag: NO_TAG,
                },
            );
            Ok(())
        })?;
        self.backend.delete(namespace, key, if_revision.as_ref())?;
        Ok(())
    }

    /// List sealed records by prefix within a namespace. Returns
    /// ciphertext-side metadata only; the AEAD is not run.
    pub fn list(
        &self,
        namespace: &str,
        options: StorageListOptions,
    ) -> Result<Vec<SealedStat>, SealedStoreError> {
        self.list_page(namespace, options).map(|page| page.records)
    }

    /// Like [`list`](Self::list), but keeps the backend's continuation cursor.
    ///
    /// A caller that pages through a namespace must use this rather than
    /// infer "done" from a short page. Backends are allowed to return fewer
    /// records than `page_size` *and still have more*: `storage-fs`, for one,
    /// drops a key deleted between its directory scan and its read, so a page
    /// can come back one short while `next_cursor` says to keep going. A
    /// caller that stopped on the short page would silently miss every record
    /// after it.
    pub fn list_page(
        &self,
        namespace: &str,
        options: StorageListOptions,
    ) -> Result<SealedPage, SealedStoreError> {
        check_external_namespace(namespace)?;
        {
            let guard = self.state.lock().expect("vault state mutex poisoned");
            if guard.unsealed.is_none() {
                return Err(SealedStoreError::Sealed);
            }
        }
        let page = self.backend.list(namespace, options)?;
        let mut out = Vec::with_capacity(page.records.len());
        for rec in page.records {
            let meta = SealedRecordMeta::parse(&rec.metadata)?;
            out.push(SealedStat {
                namespace: rec.namespace,
                key: rec.key,
                revision: rec.revision,
                created_at_ms: rec.created_at,
                updated_at_ms: rec.updated_at,
                ciphertext_len: rec.body.len(),
                kek_id: meta.kek_id,
            });
        }
        Ok(SealedPage {
            records: out,
            next_cursor: page.next_cursor,
        })
    }

    /// Return a redacted envelope summary for one sealed record.
    ///
    /// Like `list()`, this is a read-only metadata path that does not unwrap
    /// a DEK or decrypt the body. It still requires the store to be unsealed
    /// so a sealed vault cannot be used as an unauthenticated existence oracle.
    pub fn summarize(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<SealedEnvelopeSummary>, SealedStoreError> {
        check_external_namespace(namespace)?;
        {
            let guard = self.state.lock().expect("vault state mutex poisoned");
            if guard.unsealed.is_none() {
                return Err(SealedStoreError::Sealed);
            }
        }

        let record = match self.backend.get(namespace, key)? {
            Some(r) => r,
            None => return Ok(None),
        };
        let meta = SealedRecordMeta::parse(&record.metadata)?;
        Ok(Some(SealedEnvelopeSummary {
            namespace: record.namespace,
            key: record.key,
            revision: record.revision,
            created_at_ms: record.created_at,
            updated_at_ms: record.updated_at,
            schema_version: meta.version,
            aead: meta.aead,
            ciphertext_len: record.body.len(),
            body_nonce_len: meta.body_nonce.len(),
            body_tag_len: meta.body_tag.len(),
            body_aad_len: meta.body_aad.len(),
            wrapped_dek_len: meta.wrapped_dek.len(),
            wrap_nonce_len: meta.wrap_nonce.len(),
            wrap_tag_len: meta.wrap_tag.len(),
            kek_id: meta.kek_id,
        }))
    }

    // ---- rotation ----------------------------------------------------------

    /// Rotate the master KEK: unseal under `old_password`, derive a new KEK
    /// from `new_password` + fresh salt, and rewrap every record's DEK
    /// under the new KEK. Bodies are not re-encrypted.
    ///
    /// ### Crash safety
    ///
    /// This call writes the manifest **before** rewrapping any record. The
    /// persisted manifest at that point contains both the old and the new
    /// KEK entries (old marked `retired`, new marked `active`), each with
    /// its own salt and verifier. Consequences:
    ///
    /// - A crash before all records are rewrapped leaves some records with
    ///   `kek_id = old`. These are still readable — a caller can unseal
    ///   with the **old** password (the retired entry verifies), or call
    ///   `rotate_kek` again with the new password to resume the rewrap.
    /// - A crash after rewrap completes leaves all records under the new
    ///   KEK. The old retired entry remains in the manifest until a future
    ///   admin prunes it; it costs ~128 bytes and does not weaken the
    ///   security of new records.
    pub fn rotate_kek(
        &self,
        old_password: &[u8],
        new_password: &[u8],
    ) -> Result<KekRotationReport, SealedStoreError> {
        // Step 1: confirm caller knows the old password.
        self.unseal(old_password)?;

        // Step 2: pull the unsealed KEK into an owned Zeroizing<[u8;32]>.
        // We hold the state lock only long enough to copy; we do NOT hold
        // it across the (minutes-long) Argon2 derivation below.
        let old_kek: Zeroizing<[u8; KEY_LEN]>;
        let old_kek_id: String;
        {
            let guard = self.state.lock().expect("vault state mutex poisoned");
            let cur = guard.unsealed.as_ref().ok_or(SealedStoreError::Sealed)?;
            let mut copy = Zeroizing::new([0u8; KEY_LEN]);
            copy.copy_from_slice(&*cur.key);
            old_kek = copy;
            old_kek_id = cur.id.clone();
        }

        // Step 3: load and parse the manifest for its KDF parameters.
        let manifest_record = self
            .backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)?
            .ok_or(SealedStoreError::NotInitialized)?;
        let mut manifest = Manifest::parse(&manifest_record.metadata)?;
        let manifest_revision = manifest_record.revision.clone();

        // Step 4: draw a fresh salt and derive the new KEK. Every KEK has
        // its own salt so retired entries remain independently verifiable
        // and no single salt ever needs to be overwritten.
        let new_salt = random_bytes(DEFAULT_ARGON2_SALT_LEN)
            .map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?;
        let new_kek = derive_kek(
            new_password,
            &new_salt,
            manifest.time_cost,
            manifest.memory_kib,
            manifest.parallelism,
        )?;

        // Step 5: build the new verifier and manifest entry.
        let new_kek_id = next_kek_id(&manifest.keks)?;
        let verifier_nonce: [u8; NONCE_LEN] =
            random_array().map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?;
        let (verifier_ct, verifier_tag) = xchacha20_poly1305_aead_encrypt(
            &VERIFIER_PLAINTEXT,
            &new_kek,
            &verifier_nonce,
            b"vault-verifier",
        );
        for e in manifest.keks.iter_mut() {
            if e.id == old_kek_id {
                e.status = "retired";
            }
        }
        manifest.keks.push(KekEntry {
            id: new_kek_id.clone(),
            status: "active",
            source: KekSource::PasswordDerived,
            salt: Some(new_salt),
            verifier_nonce,
            verifier_tag,
            verifier_ct,
        });

        // Step 6: persist manifest first (CAS on its revision) — the
        // moment this returns, both old and new KEKs are valid for unseal.
        let new_manifest_json = build_manifest_json(
            MANIFEST_VERSION,
            manifest.time_cost,
            manifest.memory_kib,
            manifest.parallelism,
            &manifest.keks,
            now_ms_from_wallclock(),
        );
        let put_in = StoragePutInput::new(
            RESERVED_NAMESPACE.to_string(),
            MANIFEST_KEY.to_string(),
            MANIFEST_CONTENT_TYPE.to_string(),
            new_manifest_json,
            Vec::new(),
        )
        .map_err(SealedStoreError::Storage)?
        .with_if_revision(Some(manifest_revision));
        self.backend.put(put_in)?;

        // Step 7: iterate every registered external namespace and rewrap
        // each record's DEK under the new KEK. This is restartable:
        // records already wrapped under `new_kek_id` are left alone.
        let mut rewrapped = 0usize;
        let mut already_new = 0usize;
        for ns in self.list_registered_namespaces()? {
            let mut cursor: Option<String> = None;
            loop {
                let page = self.backend.list(
                    &ns,
                    StorageListOptions {
                        prefix: None,
                        recursive: true,
                        page_size: Some(128),
                        cursor: cursor.clone(),
                    },
                )?;
                for rec in page.records {
                    let meta = SealedRecordMeta::parse(&rec.metadata)?;
                    if meta.kek_id == new_kek_id {
                        already_new += 1;
                        continue;
                    }
                    if meta.kek_id != old_kek_id {
                        // Some other retired KEK — we don't own that key, skip.
                        continue;
                    }

                    // Unwrap under old KEK.
                    let old_wrap_aad = wrap_aad(&rec.namespace, &rec.key, &old_kek_id);
                    let dek = unwrap_dek(
                        &meta.wrapped_dek,
                        &old_kek,
                        &meta.wrap_nonce,
                        &old_wrap_aad,
                        &meta.wrap_tag,
                        &rec.namespace,
                        &rec.key,
                    )?;

                    // Rewrap under new KEK.
                    let new_wrap_nonce: [u8; NONCE_LEN] = random_array()
                        .map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?;
                    let new_wrap_aad = wrap_aad(&rec.namespace, &rec.key, &new_kek_id);
                    let (new_wrapped_dek, new_wrap_tag) = xchacha20_poly1305_aead_encrypt(
                        &*dek,
                        &new_kek,
                        &new_wrap_nonce,
                        &new_wrap_aad,
                    );
                    // `dek` drops at end of this loop iteration.

                    let new_meta = build_sealed_metadata(&SealedRecordMeta {
                        wrapped_dek: new_wrapped_dek,
                        wrap_nonce: new_wrap_nonce,
                        wrap_tag: new_wrap_tag,
                        kek_id: new_kek_id.clone(),
                        ..meta
                    });
                    let rewrite = StoragePutInput::new(
                        rec.namespace.clone(),
                        rec.key.clone(),
                        SEALED_CONTENT_TYPE.to_string(),
                        new_meta,
                        rec.body.clone(),
                    )
                    .map_err(SealedStoreError::Storage)?
                    .with_if_revision(Some(rec.revision.clone()));
                    self.backend.put(rewrite)?;
                    rewrapped += 1;
                }
                match page.next_cursor {
                    Some(next) => cursor = Some(next),
                    None => break,
                }
            }
            // F7: the namespace's freshness index is a sealed envelope too.
            // Left under the old KEK, every later read of the namespace
            // would fail as tamper.
            let index_key = freshness_key(&ns);
            if let Some(rec) = self.backend.get(RESERVED_NAMESPACE, &index_key)? {
                let meta = SealedRecordMeta::parse(&rec.metadata)?;
                if meta.kek_id == old_kek_id {
                    let dek = unwrap_dek(
                        &meta.wrapped_dek,
                        &old_kek,
                        &meta.wrap_nonce,
                        &wrap_aad(RESERVED_NAMESPACE, &index_key, &old_kek_id),
                        &meta.wrap_tag,
                        RESERVED_NAMESPACE,
                        &index_key,
                    )?;
                    let new_wrap_nonce: [u8; NONCE_LEN] = random_array()
                        .map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?;
                    let (new_wrapped_dek, new_wrap_tag) = xchacha20_poly1305_aead_encrypt(
                        &*dek,
                        &new_kek,
                        &new_wrap_nonce,
                        &wrap_aad(RESERVED_NAMESPACE, &index_key, &new_kek_id),
                    );
                    let new_meta = build_sealed_metadata(&SealedRecordMeta {
                        wrapped_dek: new_wrapped_dek,
                        wrap_nonce: new_wrap_nonce,
                        wrap_tag: new_wrap_tag,
                        kek_id: new_kek_id.clone(),
                        ..meta
                    });
                    let rewrite = StoragePutInput::new(
                        RESERVED_NAMESPACE.to_string(),
                        index_key.clone(),
                        FRESHNESS_CONTENT_TYPE.to_string(),
                        new_meta,
                        rec.body.clone(),
                    )
                    .map_err(SealedStoreError::Storage)?
                    .with_if_revision(Some(rec.revision.clone()));
                    self.backend.put(rewrite)?;
                }
            }
        }

        // Step 8: swap the in-memory KEK. Moving `new_kek` into the state
        // transfers ownership of the Zeroizing wrapper — no extra copy.
        self.state
            .lock()
            .expect("vault state mutex poisoned")
            .unsealed = Some(UnsealedKey {
            id: new_kek_id.clone(),
            key: new_kek,
        });

        // `old_kek` drops here, wiping the old KEK bytes.
        Ok(KekRotationReport {
            new_kek_id,
            records_rewrapped: rewrapped,
            records_already_new: already_new,
        })
    }

    // ---- freshness (VLT01 F1-F8) -------------------------------------------

    /// Read and authenticate a namespace's freshness index, if it has one.
    ///
    /// Any failure to open or decode an index that exists is `Tamper`. An
    /// index we cannot read is never treated as "no index", because "no
    /// index" accepts every v1 record (F3).
    fn load_index(
        &self,
        unsealed: &UnsealedKey,
        namespace: &str,
    ) -> Result<Option<(FreshnessIndex, Revision)>, SealedStoreError> {
        let index_key = freshness_key(namespace);
        let Some(record) = self.backend.get(RESERVED_NAMESPACE, &index_key)? else {
            // F11: an index the anchor remembers was deleted, not never made.
            if self.anchored_epoch(namespace)?.is_some() {
                return Err(tamper(RESERVED_NAMESPACE, &index_key));
            }
            return Ok(None);
        };
        let cached = {
            let cache = self.indexes.lock().expect("vault index cache poisoned");
            cache
                .get(namespace)
                .filter(|(revision, _)| *revision == record.revision)
                .map(|(_, index)| index.clone())
        };
        if let Some(index) = cached {
            // The revision string is the backend's to report, and a restored
            // file can carry the old one. A cache hit is still held to the
            // anchor, which another process may have moved past it.
            if self
                .anchored_epoch(namespace)?
                .is_some_and(|anchored| index.epoch < anchored)
            {
                return Err(tamper(RESERVED_NAMESPACE, &index_key));
            }
            return Ok(Some((index, record.revision)));
        }
        let invalid = || tamper(RESERVED_NAMESPACE, &index_key);
        let meta = SealedRecordMeta::parse(&record.metadata).map_err(|_| invalid())?;
        if meta.generation.is_some() {
            return Err(invalid());
        }
        let plaintext = open_envelope(
            unsealed,
            RESERVED_NAMESPACE,
            &index_key,
            &meta,
            &freshness_aad(namespace),
            &record.body,
        )?;
        let index = FreshnessIndex::decode(&plaintext).ok_or_else(invalid)?;
        // F11: an authentic index older than the anchor is an old copy put
        // back, across restarts as well as within this process.
        let anchored = self.anchored_epoch(namespace)?;
        if anchored.is_some_and(|anchored| index.epoch < anchored) {
            return Err(invalid());
        }
        // F11, raise on read: an authentic index carries an epoch this vault
        // wrote, so the anchor may safely move up to it. That gives a vault
        // written before anchoring existed its protection from the first
        // anchored *read* (trust on first use), not only from its next
        // write, and it repairs an advance a crash or a failed anchor write
        // skipped.
        if let Some(anchor) = &self.anchor {
            if anchored.is_none_or(|anchored| anchored < index.epoch) {
                anchor
                    .advance(namespace, index.epoch)
                    .map_err(anchor_error)?;
            }
        }
        let mut cache = self.indexes.lock().expect("vault index cache poisoned");
        // The epoch floor. Every index write advances the epoch, so an
        // authentic index older than one this process has already seen is
        // an old copy put back. Refusing it here stops it being read and,
        // more importantly, stops the next write building on it and sealing
        // it as the newest. This holds only for the life of the process;
        // across a restart it needs the external anchor (F10, P1.20b).
        if let Some((_, seen)) = cache.get(namespace) {
            if index.epoch < seen.epoch {
                return Err(invalid());
            }
        }
        cache.insert(
            namespace.to_string(),
            (record.revision.clone(), index.clone()),
        );
        Ok(Some((index, record.revision)))
    }

    /// Seal and write an index: CAS on `revision`, or create-only when
    /// `None`.
    fn store_index(
        &self,
        unsealed: &UnsealedKey,
        namespace: &str,
        index: &FreshnessIndex,
        revision: Option<&Revision>,
    ) -> Result<Revision, SealedStoreError> {
        let encoded =
            Zeroizing::new(index.encode().ok_or_else(|| SealedStoreError::Validation {
                field: "freshness".to_string(),
                message: "namespace index is full".to_string(),
            })?);
        let index_key = freshness_key(namespace);
        let (metadata, body, _) = seal_envelope(
            unsealed,
            RESERVED_NAMESPACE,
            &index_key,
            &freshness_aad(namespace),
            None,
            &encoded,
        )?;
        let input = StoragePutInput::new(
            RESERVED_NAMESPACE.to_string(),
            index_key,
            FRESHNESS_CONTENT_TYPE.to_string(),
            metadata,
            body,
        )
        .map_err(SealedStoreError::Storage)?;
        let input = match revision {
            Some(revision) => input.with_if_revision(Some(revision.clone())),
            None => input.with_if_absent(),
        };
        let record = self.backend.put(input)?;
        self.indexes
            .lock()
            .expect("vault index cache poisoned")
            .insert(
                namespace.to_string(),
                (record.revision.clone(), index.clone()),
            );
        // F11: index first, then anchor. A crash between them leaves the
        // anchor behind, which is weaker for one write but never refuses an
        // index this vault wrote.
        if let Some(anchor) = &self.anchor {
            anchor
                .advance(namespace, index.epoch)
                .map_err(anchor_error)?;
        }
        Ok(record.revision)
    }

    /// Apply `change` and write the index, re-reading and retrying on a CAS
    /// conflict (F8). Every write advances the epoch.
    fn update_index(
        &self,
        unsealed: &UnsealedKey,
        namespace: &str,
        current: (FreshnessIndex, Revision),
        change: impl Fn(&mut FreshnessIndex) -> Result<(), SealedStoreError>,
    ) -> Result<(), SealedStoreError> {
        let mut current = current;
        for _ in 0..INDEX_CAS_ATTEMPTS {
            let (mut index, revision) = current;
            change(&mut index)?;
            index.epoch = index.epoch.saturating_add(1);
            match self.store_index(unsealed, namespace, &index, Some(&revision)) {
                Ok(_) => return Ok(()),
                Err(SealedStoreError::Storage(StorageError::Conflict { .. })) => {
                    current = self
                        .load_index(unsealed, namespace)?
                        .ok_or_else(|| tamper(RESERVED_NAMESPACE, &freshness_key(namespace)))?;
                }
                Err(error) => return Err(error),
            }
        }
        Err(SealedStoreError::Storage(StorageError::Backend {
            message: "freshness index: too many CAS conflicts".to_string(),
        }))
    }

    /// The index a write may build on: migrated (F6), with no entry still
    /// `Legacy`, and caught up with every authentic record on disk (F4).
    fn ready_index(
        &self,
        unsealed: &UnsealedKey,
        namespace: &str,
    ) -> Result<(FreshnessIndex, Revision), SealedStoreError> {
        let mut current = match self.load_index(unsealed, namespace)? {
            Some(current) => current,
            None => self.begin_migration(unsealed, namespace)?,
        };
        if current.0.has_legacy() {
            current = self.finish_migration(unsealed, namespace, current)?;
        }
        self.reconcile(unsealed, namespace, current)
    }

    /// F4's catch-up step: absorb every authentic record that is ahead of
    /// the index before a write builds on it.
    ///
    /// Without this, a write would extend whatever index it found. If that
    /// index were stale (a `put` that crashed before its index update, or an
    /// old copy of the index put back by someone), the write would seal the
    /// stale floor as the newest index. Generations would then be reused, and
    /// a later single-file restore of an old record would pass. Absorbing
    /// first means the floor a write seals is never below a record a KEK
    /// holder actually wrote.
    ///
    /// Only records whose metadata claims to be ahead are opened, and only
    /// one that passes its AEAD is absorbed. An attacker can make this read
    /// more, but cannot make it accept anything they wrote. Junk files are
    /// skipped rather than failing the write; `get` refuses them anyway.
    fn reconcile(
        &self,
        unsealed: &UnsealedKey,
        namespace: &str,
        current: (FreshnessIndex, Revision),
    ) -> Result<(FreshnessIndex, Revision), SealedStoreError> {
        let mut absorbed: Vec<(String, Entry)> = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = self.backend.list(
                namespace,
                StorageListOptions {
                    prefix: None,
                    recursive: true,
                    page_size: Some(128),
                    cursor: cursor.clone(),
                },
            )?;
            for record in page.records {
                let Ok(meta) = SealedRecordMeta::parse(&record.metadata) else {
                    continue;
                };
                let Some(generation) = meta.generation else {
                    continue;
                };
                let ahead = match current.0.entries.get(&record.key) {
                    None => true,
                    Some(entry) => match entry.state {
                        EntryState::Live | EntryState::Tombstone => generation > entry.generation,
                        EntryState::Legacy => false,
                    },
                };
                if !ahead {
                    continue;
                }
                let authentic = open_envelope(
                    unsealed,
                    namespace,
                    &record.key,
                    &meta,
                    &record_aad_v2(namespace, &record.key, generation),
                    &record.body,
                )
                .is_ok();
                if authentic {
                    absorbed.push((
                        record.key,
                        Entry {
                            generation,
                            state: EntryState::Live,
                            tag: meta.body_tag,
                        },
                    ));
                }
            }
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        if absorbed.is_empty() {
            return Ok(current);
        }
        self.update_index(unsealed, namespace, current, |index| {
            for (key, entry) in &absorbed {
                let behind = index
                    .entries
                    .get(key)
                    .is_none_or(|existing| existing.generation < entry.generation);
                if behind {
                    index.entries.insert(key.clone(), *entry);
                }
            }
            Ok(())
        })?;
        self.load_index(unsealed, namespace)?
            .ok_or_else(|| tamper(RESERVED_NAMESPACE, &freshness_key(namespace)))
    }

    /// F6 steps 1-2: list the namespace and write an index with every key
    /// `legacy`.
    ///
    /// A v2 record here means the namespace was migrated once and its index
    /// has since disappeared. Adopting the records found now would accept
    /// whatever older files were put back alongside the deletion, so this
    /// refuses instead.
    fn begin_migration(
        &self,
        unsealed: &UnsealedKey,
        namespace: &str,
    ) -> Result<(FreshnessIndex, Revision), SealedStoreError> {
        let mut index = FreshnessIndex::default();
        let mut cursor: Option<String> = None;
        loop {
            let page = self.backend.list(
                namespace,
                StorageListOptions {
                    prefix: None,
                    recursive: true,
                    page_size: Some(128),
                    cursor: cursor.clone(),
                },
            )?;
            for record in page.records {
                if SealedRecordMeta::parse(&record.metadata)?
                    .generation
                    .is_some()
                {
                    return Err(tamper(namespace, &record.key));
                }
                index.entries.insert(
                    record.key,
                    Entry {
                        generation: 1,
                        state: EntryState::Legacy,
                        tag: NO_TAG,
                    },
                );
                if index.entries.len() > MAX_ENTRIES {
                    return Err(SealedStoreError::Validation {
                        field: "freshness".to_string(),
                        message: "namespace index is full".to_string(),
                    });
                }
            }
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        match self.store_index(unsealed, namespace, &index, None) {
            Ok(revision) => Ok((index, revision)),
            // Another process created the index first. Build on theirs.
            Err(SealedStoreError::Storage(StorageError::Conflict { .. })) => self
                .load_index(unsealed, namespace)?
                .ok_or_else(|| tamper(RESERVED_NAMESPACE, &freshness_key(namespace))),
            Err(error) => Err(error),
        }
    }

    /// F6 steps 3-4: re-seal each legacy key's v1 record as v2 generation 1,
    /// then mark those entries live, pinned to the re-sealed record's tag.
    ///
    /// Each step leaves a state F3 accepts, so a crash anywhere here is
    /// resumed by the next write. A v1 record is decrypted under the v1 AAD
    /// before it is re-sealed, and a v2 one left by an earlier crash is
    /// authenticated before its tag is pinned, so nothing an attacker wrote
    /// is laundered into an authentic-looking v2 record.
    ///
    /// A record that does not parse or decrypt is skipped. Its entry becomes
    /// live with no tag, which no generation-1 record matches, so `get` keeps
    /// refusing it exactly as before, and one bad file cannot block every
    /// write to the namespace. A v1 record wrapped under a *retired* KEK is
    /// different: it is recoverable by resuming `rotate_kek`, so migration
    /// stops with an error instead of stranding it.
    fn finish_migration(
        &self,
        unsealed: &UnsealedKey,
        namespace: &str,
        current: (FreshnessIndex, Revision),
    ) -> Result<(FreshnessIndex, Revision), SealedStoreError> {
        let legacy: Vec<String> = current
            .0
            .entries
            .iter()
            .filter(|(_, entry)| entry.state == EntryState::Legacy)
            .map(|(key, _)| key.clone())
            .collect();
        let mut pinned: HashMap<String, [u8; TAG_BYTES]> = HashMap::new();
        for key in &legacy {
            if let Some(tag) = self.migrate_one(unsealed, namespace, key)? {
                pinned.insert(key.clone(), tag);
            }
        }
        self.update_index(unsealed, namespace, current, |index| {
            for (key, entry) in index.entries.iter_mut() {
                if entry.state == EntryState::Legacy {
                    *entry = Entry {
                        generation: 1,
                        state: EntryState::Live,
                        tag: pinned.get(key).copied().unwrap_or(NO_TAG),
                    };
                }
            }
            Ok(())
        })?;
        self.load_index(unsealed, namespace)?
            .ok_or_else(|| tamper(RESERVED_NAMESPACE, &freshness_key(namespace)))
    }

    /// Whether `kek_id` names a retired entry in the manifest.
    fn is_retired_kek(&self, kek_id: &str) -> Result<bool, SealedStoreError> {
        let manifest = self
            .backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)?
            .ok_or(SealedStoreError::NotInitialized)?;
        Ok(Manifest::parse(&manifest.metadata)?
            .keks
            .iter()
            .any(|entry| entry.id == kek_id && entry.status == "retired"))
    }

    /// Bring one legacy key to v2 generation 1, and return the tag of its
    /// authentic generation-1 record. `None` means no such record now
    /// exists: the key is gone, its file does not open, or a concurrent
    /// writer replaced it first.
    fn migrate_one(
        &self,
        unsealed: &UnsealedKey,
        namespace: &str,
        key: &str,
    ) -> Result<Option<[u8; TAG_BYTES]>, SealedStoreError> {
        let Some(record) = self.backend.get(namespace, key)? else {
            return Ok(None);
        };
        let Ok(meta) = SealedRecordMeta::parse(&record.metadata) else {
            return Ok(None);
        };
        match meta.generation {
            // Re-sealed before an earlier crash: pin it only if it is
            // genuine.
            Some(1) => {
                let authentic = open_envelope(
                    unsealed,
                    namespace,
                    key,
                    &meta,
                    &record_aad_v2(namespace, key, 1),
                    &record.body,
                )
                .is_ok();
                Ok(authentic.then_some(meta.body_tag))
            }
            Some(_) => Ok(None),
            None => {
                if meta.kek_id != unsealed.id {
                    // `kek_id` is plaintext, so only a KEK the manifest
                    // actually lists as retired counts. That is a real
                    // interrupted rotation: resuming it recovers the record,
                    // and skipping would strand it, so migration stops. Any
                    // other id is junk and is skipped, or a planted file
                    // naming "kek-999" could block every write here.
                    if !self.is_retired_kek(&meta.kek_id)? {
                        return Ok(None);
                    }
                    return Err(SealedStoreError::Validation {
                        field: "kek_id".to_string(),
                        message: "record is under a retired KEK; resume rotate_kek first"
                            .to_string(),
                    });
                }
                let Ok(plaintext) = open_envelope(
                    unsealed,
                    namespace,
                    key,
                    &meta,
                    &record_aad(namespace, key),
                    &record.body,
                ) else {
                    return Ok(None);
                };
                let (metadata, body, tag) = seal_envelope(
                    unsealed,
                    namespace,
                    key,
                    &record_aad_v2(namespace, key, 1),
                    Some(1),
                    &plaintext,
                )?;
                let put = StoragePutInput::new(
                    namespace.to_string(),
                    key.to_string(),
                    SEALED_CONTENT_TYPE.to_string(),
                    metadata,
                    body,
                )
                .map_err(SealedStoreError::Storage)?
                .with_if_revision(Some(record.revision));
                match self.backend.put(put) {
                    Ok(_) => Ok(Some(tag)),
                    // A concurrent writer changed it; that writer owns it,
                    // and the next write's reconcile absorbs its record.
                    Err(StorageError::Conflict { .. }) => Ok(None),
                    Err(error) => Err(SealedStoreError::Storage(error)),
                }
            }
        }
    }

    // ---- internal namespace registry --------------------------------------
    //
    // storage-core does not enumerate namespaces, so `rotate_kek` needs an
    // out-of-band index of "namespaces the vault has ever written to".
    // We maintain it under (`__vault__`, `namespaces`) as a JSON array.
    // Every `put()` reads-modifies-writes this record via CAS if (and
    // only if) its namespace isn't already in the list.

    /// Append `namespace` to the registry if it isn't already present.
    /// Idempotent. Uses CAS to survive concurrent `put` on different
    /// namespaces; retries a bounded number of times on conflict.
    fn register_namespace(&self, namespace: &str) -> Result<(), SealedStoreError> {
        // Must only be called for external namespaces. `check_external_namespace`
        // has already run in the caller, so debug-assert here.
        debug_assert_ne!(namespace, RESERVED_NAMESPACE);

        const MAX_ATTEMPTS: usize = 8;
        for _ in 0..MAX_ATTEMPTS {
            let existing = self.backend.get(RESERVED_NAMESPACE, NAMESPACES_KEY)?;
            let (mut names, rev) = match existing {
                Some(rec) => (parse_namespaces(&rec.metadata), Some(rec.revision)),
                None => (Vec::new(), None),
            };
            if names.iter().any(|n| n == namespace) {
                return Ok(());
            }
            names.push(namespace.to_string());

            let meta = build_namespaces_json(&names);
            let put = StoragePutInput::new(
                RESERVED_NAMESPACE.to_string(),
                NAMESPACES_KEY.to_string(),
                NAMESPACES_CONTENT_TYPE.to_string(),
                meta,
                Vec::new(),
            )
            .map_err(SealedStoreError::Storage)?
            .with_if_revision(rev);

            match self.backend.put(put) {
                Ok(_) => return Ok(()),
                Err(StorageError::Conflict { .. }) => continue,
                Err(e) => return Err(SealedStoreError::Storage(e)),
            }
        }
        Err(SealedStoreError::Storage(StorageError::Backend {
            message: "namespace registry: too many CAS conflicts".to_string(),
        }))
    }

    /// Read the registered namespaces. Always filters out the reserved
    /// namespace even if an attacker managed to inject it into the list —
    /// rotation must never attempt to rewrap records inside `__vault__`.
    fn list_registered_namespaces(&self) -> Result<Vec<String>, SealedStoreError> {
        let rec = self.backend.get(RESERVED_NAMESPACE, NAMESPACES_KEY)?;
        let mut names = match rec {
            Some(r) => parse_namespaces(&r.metadata),
            None => Vec::new(),
        };
        names.retain(|n| n != RESERVED_NAMESPACE);
        Ok(names)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers.
// ---------------------------------------------------------------------------

fn check_external_namespace(namespace: &str) -> Result<(), SealedStoreError> {
    if namespace == RESERVED_NAMESPACE {
        return Err(SealedStoreError::Validation {
            field: "namespace".to_string(),
            message: "reserved namespace".to_string(),
        });
    }
    Ok(())
}

fn record_aad(namespace: &str, key: &str) -> Vec<u8> {
    // namespace || 0x00 || key — chosen over concat because 0x00 is not a
    // legal character in either string and therefore gives an unambiguous
    // delimiter.
    let mut v = Vec::with_capacity(namespace.len() + 1 + key.len());
    v.extend_from_slice(namespace.as_bytes());
    v.push(0);
    v.extend_from_slice(key.as_bytes());
    v
}

/// The format-2 body AAD (F2): the v1 address, then the generation.
///
/// Binding the generation here is what makes it tamper-evident. It is stored
/// in plaintext metadata, and editing it changes the AAD the AEAD checks.
fn record_aad_v2(namespace: &str, key: &str, generation: u64) -> Vec<u8> {
    let mut v = record_aad(namespace, key);
    v.push(0);
    v.extend_from_slice(&generation.to_be_bytes());
    v
}

/// Where a namespace's freshness index lives, under the reserved namespace.
fn freshness_key(namespace: &str) -> String {
    format!("{FRESHNESS_KEY_PREFIX}{namespace}")
}

/// The index's body AAD (F1), domain-separated from every record AAD.
fn freshness_aad(namespace: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(FRESHNESS_AAD_DOMAIN.len() + 1 + namespace.len());
    v.extend_from_slice(FRESHNESS_AAD_DOMAIN);
    v.push(0);
    v.extend_from_slice(namespace.as_bytes());
    v
}

/// An anchor failure, as a storage error with a fixed message. It is never
/// treated as "no anchor": an unreadable anchor must not quietly disable
/// the check it exists for.
fn anchor_error(error: AnchorError) -> SealedStoreError {
    SealedStoreError::Storage(StorageError::Backend {
        message: error.to_string(),
    })
}

fn tamper(namespace: &str, key: &str) -> SealedStoreError {
    SealedStoreError::Tamper {
        namespace: namespace.to_string(),
        key: key.to_string(),
    }
}

/// Encrypt `plaintext` for the address `(namespace, key)`: a fresh DEK
/// under `aad`, wrapped by the active KEK. Returns the metadata and the
/// ciphertext.
fn seal_envelope(
    unsealed: &UnsealedKey,
    namespace: &str,
    key: &str,
    aad: &[u8],
    generation: Option<u64>,
    plaintext: &[u8],
) -> Result<(JsonValue, Vec<u8>, [u8; TAG_BYTES]), SealedStoreError> {
    // Wrapped in Zeroizing at creation, so any `?` below wipes it.
    let dek: Zeroizing<[u8; KEY_LEN]> = Zeroizing::new(
        random_array().map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?,
    );
    let body_nonce: [u8; NONCE_LEN] =
        random_array().map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?;
    let (ciphertext, body_tag) = xchacha20_poly1305_aead_encrypt(plaintext, &dek, &body_nonce, aad);
    let wrap_nonce: [u8; NONCE_LEN] =
        random_array().map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?;
    let (wrapped_dek, wrap_tag) = xchacha20_poly1305_aead_encrypt(
        &*dek,
        &unsealed.key,
        &wrap_nonce,
        &wrap_aad(namespace, key, &unsealed.id),
    );
    let metadata = build_sealed_metadata(&SealedRecordMeta {
        version: SEALED_RECORD_VERSION_1,
        aead: "xchacha20poly1305".to_string(),
        body_nonce,
        body_tag,
        body_aad: aad.to_vec(),
        wrapped_dek,
        wrap_nonce,
        wrap_tag,
        kek_id: unsealed.id.clone(),
        generation,
    });
    Ok((metadata, ciphertext, body_tag))
}

/// Decrypt an envelope found at `(namespace, key)`, after checking that its
/// AAD is the one expected there and that it is wrapped by the active KEK.
fn open_envelope(
    unsealed: &UnsealedKey,
    namespace: &str,
    key: &str,
    meta: &SealedRecordMeta,
    expected_aad: &[u8],
    body: &[u8],
) -> Result<Zeroizing<Vec<u8>>, SealedStoreError> {
    // The AAD must be the one for where we found it.
    if meta.body_aad != expected_aad {
        return Err(tamper(namespace, key));
    }
    // We only unwrap envelopes wrapped under the in-memory KEK. One wrapped
    // under an earlier KEK (e.g. during an in-progress rotation) is surfaced
    // as Tamper, so the caller knows to resume rotation or unseal under the
    // older password.
    if meta.kek_id != unsealed.id {
        return Err(tamper(namespace, key));
    }
    let dek = unwrap_dek(
        &meta.wrapped_dek,
        &unsealed.key,
        &meta.wrap_nonce,
        &wrap_aad(namespace, key, &unsealed.id),
        &meta.wrap_tag,
        namespace,
        key,
    )?;
    let plaintext = xchacha20_poly1305_aead_decrypt(
        body,
        &dek,
        &meta.body_nonce,
        &meta.body_aad,
        &meta.body_tag,
    )
    .ok_or_else(|| tamper(namespace, key))?;
    // `dek` drops here, wiping the cleartext DEK bytes.
    Ok(Zeroizing::new(plaintext))
}

fn wrap_aad(namespace: &str, key: &str, kek_id: &str) -> Vec<u8> {
    // Binding the wrapped-DEK ciphertext to both the storage address AND the
    // KEK id means that a rotation-swap (old wrapped_dek copied on top of a
    // record that now lives under a new kek_id) fails the AEAD.
    let mut v = Vec::with_capacity(namespace.len() + 1 + key.len() + 1 + kek_id.len());
    v.extend_from_slice(namespace.as_bytes());
    v.push(0);
    v.extend_from_slice(key.as_bytes());
    v.push(0);
    v.extend_from_slice(kek_id.as_bytes());
    v
}

/// Run Argon2id and return a zeroizing 32-byte KEK. All intermediate
/// allocations holding key material are wiped on drop, including the
/// error paths.
fn derive_kek(
    password: &[u8],
    salt: &[u8],
    time_cost: u32,
    memory_kib: u32,
    parallelism: u32,
) -> Result<Zeroizing<[u8; KEY_LEN]>, SealedStoreError> {
    let tag = argon2id(
        password,
        salt,
        time_cost,
        memory_kib,
        parallelism,
        KEY_LEN as u32,
        &Argon2Options {
            key: None,
            associated_data: None,
            version: Some(ARGON2_VERSION),
        },
    )
    .map_err(|_| SealedStoreError::Crypto("argon2id derivation failed".into()))?;

    // Wrap the raw Vec<u8> from argon2id in Zeroizing *before* we touch
    // it any further, so early returns wipe it.
    let tag_z: Zeroizing<Vec<u8>> = Zeroizing::new(tag);
    if tag_z.len() != KEY_LEN {
        return Err(SealedStoreError::Crypto(
            "argon2id produced wrong tag length".into(),
        ));
    }
    let mut out = Zeroizing::new([0u8; KEY_LEN]);
    out.copy_from_slice(&tag_z);
    Ok(out)
}

/// Unwrap a wrapped DEK under the given KEK and return it as a fixed-size
/// zeroizing array. Collapses the length check + AEAD failure path into a
/// single `Tamper` outcome (no oracle leaked).
fn unwrap_dek(
    wrapped_dek: &[u8],
    kek: &[u8; KEY_LEN],
    wrap_nonce: &[u8; NONCE_LEN],
    wrap_aad: &[u8],
    wrap_tag: &[u8; TAG_LEN],
    namespace: &str,
    key: &str,
) -> Result<Zeroizing<[u8; KEY_LEN]>, SealedStoreError> {
    let dek_vec = xchacha20_poly1305_aead_decrypt(wrapped_dek, kek, wrap_nonce, wrap_aad, wrap_tag)
        .ok_or_else(|| SealedStoreError::Tamper {
            namespace: namespace.to_string(),
            key: key.to_string(),
        })?;

    // Wrap the Vec in Zeroizing *before* inspecting it so any error path
    // below wipes the bytes.
    let dek_vec_z: Zeroizing<Vec<u8>> = Zeroizing::new(dek_vec);
    if dek_vec_z.len() != KEY_LEN {
        return Err(SealedStoreError::Tamper {
            namespace: namespace.to_string(),
            key: key.to_string(),
        });
    }
    let mut out = Zeroizing::new([0u8; KEY_LEN]);
    out.copy_from_slice(&dek_vec_z);
    Ok(out)
}

fn now_ms_from_wallclock() -> u64 {
    // We do not have a clock abstraction here; the backend stamps its own
    // created_at/updated_at. The manifest stores "created_at_ms" purely for
    // operator visibility, so use the wall clock if available and fall back
    // to zero if not. (Zero is still a valid u64.)
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Pick the next stable KEK id. Rejects the (extraordinarily unlikely)
/// case of u64 overflow and also guards against an already-used id
/// (which would indicate a corrupted manifest).
fn next_kek_id(existing: &[KekEntry]) -> Result<String, SealedStoreError> {
    let mut max_n: u64 = 0;
    for e in existing {
        if let Some(rest) = e.id.strip_prefix("kek-") {
            if let Ok(n) = rest.parse::<u64>() {
                if n > max_n {
                    max_n = n;
                }
            }
        }
    }
    let next = max_n.checked_add(1).ok_or(SealedStoreError::Validation {
        field: "keks".to_string(),
        message: "kek id counter overflow".to_string(),
    })?;
    let candidate = format!("kek-{next}");
    if existing.iter().any(|e| e.id == candidate) {
        return Err(SealedStoreError::Validation {
            field: "keks".to_string(),
            message: "kek id collision".to_string(),
        });
    }
    Ok(candidate)
}

fn validate_argon2_params(
    time_cost: u32,
    memory_kib: u32,
    parallelism: u32,
) -> Result<(), SealedStoreError> {
    if !(1..=ARGON2_PARALLELISM_MAX).contains(&parallelism) {
        return Err(SealedStoreError::Validation {
            field: "argon2id_parallelism".to_string(),
            message: "out of range".to_string(),
        });
    }
    if !(1..=ARGON2_TIME_COST_MAX).contains(&time_cost) {
        return Err(SealedStoreError::Validation {
            field: "argon2id_time_cost".to_string(),
            message: "out of range".to_string(),
        });
    }
    // RFC 9106 §3.1: memory must be ≥ 8 × parallelism KiB.
    let min_memory = parallelism.saturating_mul(8);
    if memory_kib < min_memory || memory_kib > ARGON2_MEMORY_KIB_MAX {
        return Err(SealedStoreError::Validation {
            field: "argon2id_memory_kib".to_string(),
            message: "out of range".to_string(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// JSON metadata layout helpers.
//
// We use the bounded `JsonValue` directly instead of serde; storage-core's
// metadata field shares this exact repository-owned type.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct KekEntry {
    id: String,
    status: &'static str, // "active" | "retired"
    source: KekSource,
    /// Per-KEK Argon2id salt. Each entry is independently verifiable so an
    /// operator can always unseal under the password that minted *this*
    /// KEK, even after many rotations.
    salt: Option<Vec<u8>>,
    verifier_nonce: [u8; NONCE_LEN],
    verifier_tag: [u8; TAG_LEN],
    verifier_ct: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KekSource {
    PasswordDerived,
    Injected,
}

impl KekSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::PasswordDerived => "password-derived",
            Self::Injected => "injected",
        }
    }
}

#[derive(Debug, Clone)]
struct Manifest {
    time_cost: u32,
    memory_kib: u32,
    parallelism: u32,
    keks: Vec<KekEntry>,
}

struct SealedRecordMeta {
    version: u64,
    aead: String,
    body_nonce: [u8; NONCE_LEN],
    body_tag: [u8; TAG_LEN],
    body_aad: Vec<u8>,
    wrapped_dek: Vec<u8>,
    wrap_nonce: [u8; NONCE_LEN],
    wrap_tag: [u8; TAG_LEN],
    kek_id: String,
    /// `Some` exactly for a format-2 record (F2), and always `>= 1`.
    generation: Option<u64>,
}

fn build_manifest_json(
    version: u64,
    time_cost: u32,
    memory_kib: u32,
    parallelism: u32,
    keks: &[KekEntry],
    created_at_ms: u64,
) -> JsonValue {
    let keks_json: Vec<JsonValue> = keks
        .iter()
        .map(|e| {
            let mut fields = vec![
                ("id".to_string(), JsonValue::String(e.id.clone())),
                (
                    "status".to_string(),
                    JsonValue::String(e.status.to_string()),
                ),
                (
                    "source".to_string(),
                    JsonValue::String(e.source.as_str().to_string()),
                ),
                (
                    "verifier_nonce".to_string(),
                    JsonValue::String(hex_encode(&e.verifier_nonce)),
                ),
                (
                    "verifier_tag".to_string(),
                    JsonValue::String(hex_encode(&e.verifier_tag)),
                ),
                (
                    "verifier_ct".to_string(),
                    JsonValue::String(hex_encode(&e.verifier_ct)),
                ),
            ];
            if let Some(salt) = &e.salt {
                fields.push(("salt".to_string(), JsonValue::String(hex_encode(salt))));
            }
            JsonValue::Object(fields)
        })
        .collect();
    JsonValue::Object(vec![
        (
            "vault_manifest_version".to_string(),
            JsonValue::Number(JsonNumber::Integer(version as i64)),
        ),
        ("kdf".to_string(), JsonValue::String("argon2id".to_string())),
        (
            "kdf_version".to_string(),
            JsonValue::Number(JsonNumber::Integer(ARGON2_VERSION as i64)),
        ),
        (
            "kdf_time_cost".to_string(),
            JsonValue::Number(JsonNumber::Integer(time_cost as i64)),
        ),
        (
            "kdf_memory_cost_kib".to_string(),
            JsonValue::Number(JsonNumber::Integer(memory_kib as i64)),
        ),
        (
            "kdf_parallelism".to_string(),
            JsonValue::Number(JsonNumber::Integer(parallelism as i64)),
        ),
        (
            "kdf_tag_length".to_string(),
            JsonValue::Number(JsonNumber::Integer(KEY_LEN as i64)),
        ),
        ("keks".to_string(), JsonValue::Array(keks_json)),
        (
            "created_at_ms".to_string(),
            JsonValue::Number(JsonNumber::Integer(created_at_ms as i64)),
        ),
    ])
}

/// Serialize a record's envelope metadata.
///
/// `version` and `aead` on the input are ignored: the version is derived from
/// whether there is a generation (F2), and the AEAD is the one suite this
/// format commits to. Taking the struct the parser produces keeps the writer
/// and the reader describing the same fields.
fn build_sealed_metadata(meta: &SealedRecordMeta) -> JsonValue {
    let SealedRecordMeta {
        body_nonce,
        body_tag,
        body_aad,
        wrapped_dek,
        wrap_nonce,
        wrap_tag,
        kek_id,
        generation,
        ..
    } = meta;
    let generation = *generation;
    // The version follows the shape: a generation means format 2 (F2).
    let version = if generation.is_some() {
        SEALED_RECORD_VERSION
    } else {
        SEALED_RECORD_VERSION_1
    };
    let mut fields = vec![
        (
            "vault_sealed_version".to_string(),
            JsonValue::Number(JsonNumber::Integer(version as i64)),
        ),
        (
            "aead".to_string(),
            JsonValue::String("xchacha20poly1305".to_string()),
        ),
        (
            "body_nonce".to_string(),
            JsonValue::String(hex_encode(body_nonce)),
        ),
        (
            "body_tag".to_string(),
            JsonValue::String(hex_encode(body_tag)),
        ),
        (
            "body_aad".to_string(),
            JsonValue::String(hex_encode(body_aad)),
        ),
        (
            "wrapped_dek".to_string(),
            JsonValue::String(hex_encode(wrapped_dek)),
        ),
        (
            "wrapped_dek_nonce".to_string(),
            JsonValue::String(hex_encode(wrap_nonce)),
        ),
        (
            "wrapped_dek_tag".to_string(),
            JsonValue::String(hex_encode(wrap_tag)),
        ),
        ("kek_id".to_string(), JsonValue::String(kek_id.to_string())),
    ];
    if let Some(generation) = generation {
        // A JSON integer is an i64. A generation past i64::MAX would take
        // 9.2e18 writes to one key; saturate rather than wrap negative.
        fields.push((
            "generation".to_string(),
            JsonValue::Number(JsonNumber::Integer(
                i64::try_from(generation).unwrap_or(i64::MAX),
            )),
        ));
    }
    JsonValue::Object(fields)
}

fn build_namespaces_json(names: &[String]) -> JsonValue {
    JsonValue::Object(vec![
        (
            "vault_namespaces_version".to_string(),
            JsonValue::Number(JsonNumber::Integer(1)),
        ),
        (
            "names".to_string(),
            JsonValue::Array(names.iter().map(|s| JsonValue::String(s.clone())).collect()),
        ),
    ])
}

fn parse_namespaces(meta: &JsonValue) -> Vec<String> {
    let mut names = Vec::new();
    if let JsonValue::Object(obj) = meta {
        if let Some((_, JsonValue::Array(arr))) = obj.iter().find(|(k, _)| k == "names") {
            for v in arr {
                if let JsonValue::String(s) = v {
                    names.push(s.clone());
                }
            }
        }
    }
    names
}

impl Manifest {
    fn parse(meta: &JsonValue) -> Result<Self, SealedStoreError> {
        let obj = expect_object(meta, "manifest")?;
        let time_cost = get_u32(obj, "kdf_time_cost")?;
        let memory_kib = get_u32(obj, "kdf_memory_cost_kib")?;
        let parallelism = get_u32(obj, "kdf_parallelism")?;
        // Attacker-tampered bounds: the first defence against a swapped
        // manifest that tries to stall unseal forever.
        validate_argon2_params(time_cost, memory_kib, parallelism)?;

        let keks_raw = get_field(obj, "keks").map_err(|_| SealedStoreError::Validation {
            field: "keks".to_string(),
            message: "missing".to_string(),
        })?;
        let keks_arr = match keks_raw {
            JsonValue::Array(a) => a,
            _ => {
                return Err(SealedStoreError::Validation {
                    field: "keks".to_string(),
                    message: "not an array".to_string(),
                })
            }
        };
        if keks_arr.is_empty() {
            return Err(SealedStoreError::Validation {
                field: "keks".to_string(),
                message: "empty".to_string(),
            });
        }
        let mut keks = Vec::with_capacity(keks_arr.len());
        for entry in keks_arr {
            let eo = expect_object(entry, "keks_entry")?;
            let id = get_string(eo, "id")?.to_string();
            let status_raw = get_string(eo, "status")?;
            let status: &'static str = match status_raw {
                "active" => "active",
                "retired" => "retired",
                _ => {
                    return Err(SealedStoreError::Validation {
                        field: "status".to_string(),
                        message: "unsupported".to_string(),
                    })
                }
            };
            let source = match eo.iter().find(|(key, _)| key == "source") {
                None => KekSource::PasswordDerived,
                Some((_, JsonValue::String(value))) if value == "password-derived" => {
                    KekSource::PasswordDerived
                }
                Some((_, JsonValue::String(value))) if value == "injected" => KekSource::Injected,
                Some(_) => {
                    return Err(SealedStoreError::Validation {
                        field: "source".to_string(),
                        message: "unsupported".to_string(),
                    })
                }
            };
            let salt = if source == KekSource::PasswordDerived {
                let salt = hex_decode(get_string(eo, "salt")?).map_err(|_| {
                    SealedStoreError::Validation {
                        field: "salt".to_string(),
                        message: "invalid hex".to_string(),
                    }
                })?;
                if salt.len() < ARGON2_SALT_MIN_LEN || salt.len() > ARGON2_SALT_MAX_LEN {
                    return Err(SealedStoreError::Validation {
                        field: "salt".to_string(),
                        message: "length out of range".to_string(),
                    });
                }
                Some(salt)
            } else {
                None
            };
            let verifier_nonce =
                hex_decode_fixed::<NONCE_LEN>(get_string(eo, "verifier_nonce")?, "verifier_nonce")?;
            let verifier_tag =
                hex_decode_fixed::<TAG_LEN>(get_string(eo, "verifier_tag")?, "verifier_tag")?;
            let verifier_ct = hex_decode(get_string(eo, "verifier_ct")?).map_err(|_| {
                SealedStoreError::Validation {
                    field: "verifier_ct".to_string(),
                    message: "invalid hex".to_string(),
                }
            })?;
            // Known-plaintext verifier: verifier_ct is exactly VERIFIER_PLAINTEXT's
            // length (16 bytes). Reject anything else up front — avoids feeding
            // an attacker-sized blob into the AEAD.
            if verifier_ct.len() != VERIFIER_PLAINTEXT.len() {
                return Err(SealedStoreError::Validation {
                    field: "verifier_ct".to_string(),
                    message: "length mismatch".to_string(),
                });
            }
            keks.push(KekEntry {
                id,
                status,
                source,
                salt,
                verifier_nonce,
                verifier_tag,
                verifier_ct,
            });
        }
        // Disallow duplicate ids — a tampered manifest could otherwise put
        // two entries with the same id and confuse rotation.
        for i in 0..keks.len() {
            for j in (i + 1)..keks.len() {
                if keks[i].id == keks[j].id {
                    return Err(SealedStoreError::Validation {
                        field: "keks".to_string(),
                        message: "duplicate id".to_string(),
                    });
                }
            }
        }
        Ok(Self {
            time_cost,
            memory_kib,
            parallelism,
            keks,
        })
    }
}

impl SealedRecordMeta {
    fn parse(meta: &JsonValue) -> Result<Self, SealedStoreError> {
        let obj = expect_object(meta, "sealed_record")?;
        let version = get_u64(obj, "vault_sealed_version")?;
        // Version 1 has no generation and version 2 requires one. A field
        // that does not match its version is refused rather than ignored,
        // so a v1 record cannot be passed off as v2 or the other way round.
        let generation = match version {
            SEALED_RECORD_VERSION_1 => {
                if get_field(obj, "generation").is_ok() {
                    return Err(SealedStoreError::Validation {
                        field: "generation".to_string(),
                        message: "not allowed in version 1".to_string(),
                    });
                }
                None
            }
            SEALED_RECORD_VERSION => {
                let generation = get_u64(obj, "generation")?;
                if generation == 0 {
                    return Err(SealedStoreError::Validation {
                        field: "generation".to_string(),
                        message: "must be at least 1".to_string(),
                    });
                }
                Some(generation)
            }
            _ => {
                return Err(SealedStoreError::Validation {
                    field: "vault_sealed_version".to_string(),
                    message: "unsupported".to_string(),
                })
            }
        };
        let aead = get_string(obj, "aead")?.to_string();
        if aead != "xchacha20poly1305" {
            return Err(SealedStoreError::Validation {
                field: "aead".to_string(),
                message: "unsupported".to_string(),
            });
        }
        let body_nonce =
            hex_decode_fixed::<NONCE_LEN>(get_string(obj, "body_nonce")?, "body_nonce")?;
        let body_tag = hex_decode_fixed::<TAG_LEN>(get_string(obj, "body_tag")?, "body_tag")?;
        let body_aad =
            hex_decode(get_string(obj, "body_aad")?).map_err(|_| SealedStoreError::Validation {
                field: "body_aad".to_string(),
                message: "invalid hex".to_string(),
            })?;
        let wrapped_dek = hex_decode(get_string(obj, "wrapped_dek")?).map_err(|_| {
            SealedStoreError::Validation {
                field: "wrapped_dek".to_string(),
                message: "invalid hex".to_string(),
            }
        })?;
        let wrap_nonce = hex_decode_fixed::<NONCE_LEN>(
            get_string(obj, "wrapped_dek_nonce")?,
            "wrapped_dek_nonce",
        )?;
        let wrap_tag =
            hex_decode_fixed::<TAG_LEN>(get_string(obj, "wrapped_dek_tag")?, "wrapped_dek_tag")?;
        let kek_id = get_string(obj, "kek_id")?.to_string();
        Ok(Self {
            version,
            aead,
            body_nonce,
            body_tag,
            body_aad,
            wrapped_dek,
            wrap_nonce,
            wrap_tag,
            kek_id,
            generation,
        })
    }
}

fn expect_object<'a>(
    v: &'a JsonValue,
    field: &str,
) -> Result<&'a Vec<(String, JsonValue)>, SealedStoreError> {
    match v {
        JsonValue::Object(o) => Ok(o),
        _ => Err(SealedStoreError::Validation {
            field: field.to_string(),
            message: "not a JSON object".to_string(),
        }),
    }
}

fn get_field<'a>(
    obj: &'a [(String, JsonValue)],
    name: &str,
) -> Result<&'a JsonValue, SealedStoreError> {
    obj.iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
        .ok_or_else(|| SealedStoreError::Validation {
            field: name.to_string(),
            message: "missing".to_string(),
        })
}

fn get_string<'a>(obj: &'a [(String, JsonValue)], name: &str) -> Result<&'a str, SealedStoreError> {
    let v = get_field(obj, name)?;
    match v {
        JsonValue::String(s) => Ok(s),
        _ => Err(SealedStoreError::Validation {
            field: name.to_string(),
            message: "not a string".to_string(),
        }),
    }
}

fn get_u32(obj: &[(String, JsonValue)], name: &str) -> Result<u32, SealedStoreError> {
    let v = get_field(obj, name)?;
    match v {
        JsonValue::Number(JsonNumber::Integer(n)) if *n >= 0 && *n <= u32::MAX as i64 => {
            Ok(*n as u32)
        }
        _ => Err(SealedStoreError::Validation {
            field: name.to_string(),
            message: "not a non-negative 32-bit integer".to_string(),
        }),
    }
}

fn get_u64(obj: &[(String, JsonValue)], name: &str) -> Result<u64, SealedStoreError> {
    let v = get_field(obj, name)?;
    match v {
        JsonValue::Number(JsonNumber::Integer(n)) if *n >= 0 => Ok(*n as u64),
        _ => Err(SealedStoreError::Validation {
            field: name.to_string(),
            message: "not a non-negative integer".to_string(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Hex codec — tiny, constant-output-rate, dependency-free. We do NOT use
// ct-compare here because hex encoding is applied to *public* material
// (ciphertexts / nonces / tags).
// ---------------------------------------------------------------------------

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err(format!("odd hex length: {}", s.len()));
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.as_chunks::<2>().0 {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_nibble(c: u8) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(format!("invalid hex byte: 0x{c:02x}")),
    }
}

fn hex_decode_fixed<const N: usize>(s: &str, field: &str) -> Result<[u8; N], SealedStoreError> {
    let v = hex_decode(s).map_err(|_| SealedStoreError::Validation {
        field: field.to_string(),
        message: "invalid hex".to_string(),
    })?;
    if v.len() != N {
        return Err(SealedStoreError::Validation {
            field: field.to_string(),
            message: "length mismatch".to_string(),
        });
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&v);
    Ok(out)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_vault_key_custody::{
        fresh_random_key, KeyCustodian, PassphraseCustodian,
    };
    use storage_core::InMemoryStorageBackend;

    fn fast_opts() -> InitOptions {
        // Argon2 at default parameters is 64 MiB × 3 passes — ~180 ms on a
        // laptop per call, and each roundtrip test runs it twice. Tests use
        // the minimum legal parameters so the full suite stays fast.
        InitOptions {
            argon2id_time_cost: 1,
            argon2id_memory_kib: 32, // minimum for parallelism=4 is 4*8=32
            argon2id_parallelism: 4,
            salt_override: Some(vec![0x42u8; 16]),
        }
    }

    fn new_store() -> (SealedStore, Arc<dyn StorageBackend>) {
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        backend.initialize().unwrap();
        let store = SealedStore::new(Arc::clone(&backend));
        (store, backend)
    }

    fn valid_manifest_metadata() -> JsonValue {
        build_manifest_json(
            MANIFEST_VERSION,
            1,
            32,
            4,
            &[KekEntry {
                id: "kek-1".into(),
                status: "active",
                source: KekSource::PasswordDerived,
                salt: Some(vec![0x42; 16]),
                verifier_nonce: [0; NONCE_LEN],
                verifier_tag: [0; TAG_LEN],
                verifier_ct: vec![0; VERIFIER_PLAINTEXT.len()],
            }],
            0,
        )
    }

    fn set_object_field(value: &mut JsonValue, field: &str, replacement: JsonValue) {
        let JsonValue::Object(fields) = value else {
            panic!("expected object");
        };
        let (_, value) = fields.iter_mut().find(|(name, _)| name == field).unwrap();
        *value = replacement;
    }

    fn set_first_kek_field(value: &mut JsonValue, field: &str, replacement: JsonValue) {
        let JsonValue::Object(fields) = value else {
            panic!("expected manifest object");
        };
        let (_, JsonValue::Array(keks)) =
            fields.iter_mut().find(|(name, _)| name == "keks").unwrap()
        else {
            panic!("expected keks array");
        };
        let JsonValue::Object(first) = keks.first_mut().unwrap() else {
            panic!("expected KEK object");
        };
        let (_, value) = first.iter_mut().find(|(name, _)| name == field).unwrap();
        *value = replacement;
    }

    fn assert_validation_field(error: SealedStoreError, expected: &str) {
        assert!(matches!(
            error,
            SealedStoreError::Validation { ref field, .. } if field == expected
        ));
    }

    #[test]
    fn init_then_put_get_roundtrip() {
        let (store, _) = new_store();
        store.init(b"hunter2", &fast_opts()).unwrap();
        let rev = store.put("app", "k1", b"hello world", None).unwrap();
        let got = store.get("app", "k1").unwrap().unwrap();
        assert_eq!(&*got.plaintext, b"hello world");
        assert_eq!(got.revision, rev);
    }

    #[test]
    fn injected_kek_roundtrips_across_store_instances() {
        let (store, backend) = new_store();
        let root_kek = [0xA5; KEY_LEN];
        store.init_with_kek(&root_kek).unwrap();
        store
            .put("passwords", "example.com", b"secret", None)
            .unwrap();
        drop(store);

        let reopened = SealedStore::new(Arc::clone(&backend));
        reopened.unseal_with_kek(&root_kek).unwrap();
        assert_eq!(
            &*reopened
                .get("passwords", "example.com")
                .unwrap()
                .unwrap()
                .plaintext,
            b"secret"
        );
    }

    #[test]
    fn custody_wrapped_root_kek_composes_with_injected_seam() {
        let (store, backend) = new_store();
        let label = b"vault-pm/root-kek".to_vec();
        let custodian = PassphraseCustodian::with_params(b"correct horse", 1, 32, 4).unwrap();
        let root_kek = fresh_random_key().unwrap();
        let wrapped = custodian.wrap(&label, &root_kek).unwrap();

        store.init_with_kek(&root_kek).unwrap();
        store.put("items", "item-1", b"credential", None).unwrap();
        drop(root_kek);
        drop(store);

        let reopened = SealedStore::new(Arc::clone(&backend));
        let unwrapped = custodian.unwrap(&label, &wrapped).unwrap();
        reopened.unseal_with_kek(&unwrapped).unwrap();
        drop(unwrapped);
        assert_eq!(
            &*reopened.get("items", "item-1").unwrap().unwrap().plaintext,
            b"credential"
        );
    }

    #[test]
    fn injected_and_password_unseal_paths_are_isolated() {
        let (store, backend) = new_store();
        let root_kek = [0xA5; KEY_LEN];
        store.init_with_kek(&root_kek).unwrap();
        store.seal();

        assert!(matches!(
            store.unseal_with_kek(&[0x5A; KEY_LEN]),
            Err(SealedStoreError::InvalidKek)
        ));
        assert!(matches!(
            store.unseal(b"not-a-kek"),
            Err(SealedStoreError::BadPassword)
        ));

        let manifest = backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)
            .unwrap()
            .unwrap();
        let parsed = Manifest::parse(&manifest.metadata).unwrap();
        assert_eq!(parsed.keks[0].source, KekSource::Injected);
        assert!(parsed.keks[0].salt.is_none());
    }

    #[test]
    fn injected_unseal_rejects_password_derived_manifest() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.seal();
        assert!(matches!(
            store.unseal_with_kek(&[0xA5; KEY_LEN]),
            Err(SealedStoreError::InvalidKek)
        ));
        store.unseal(b"pw").unwrap();
    }

    #[test]
    fn legacy_manifest_without_source_defaults_to_password_derived() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        let manifest = backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)
            .unwrap()
            .unwrap();
        let mut metadata = match manifest.metadata.clone() {
            JsonValue::Object(fields) => fields,
            _ => panic!("bad manifest"),
        };
        for (_, value) in metadata.iter_mut().filter(|(key, _)| key == "keks") {
            if let JsonValue::Array(entries) = value {
                for entry in entries {
                    if let JsonValue::Object(fields) = entry {
                        fields.retain(|(key, _)| key != "source");
                    }
                }
            }
        }
        let update = StoragePutInput::new(
            RESERVED_NAMESPACE.to_string(),
            MANIFEST_KEY.to_string(),
            MANIFEST_CONTENT_TYPE.to_string(),
            JsonValue::Object(metadata),
            Vec::new(),
        )
        .unwrap()
        .with_if_revision(Some(manifest.revision));
        backend.put(update).unwrap();

        let reopened = SealedStore::new(Arc::clone(&backend));
        reopened.unseal(b"pw").unwrap();
    }

    #[test]
    fn seal_blocks_data_plane() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("a", "b", b"x", None).unwrap();
        store.seal();
        assert!(store.is_sealed());
        assert!(matches!(store.get("a", "b"), Err(SealedStoreError::Sealed)));
        store.unseal(b"pw").unwrap();
        assert_eq!(&*store.get("a", "b").unwrap().unwrap().plaintext, b"x");
    }

    #[test]
    fn status_tracks_manifest_namespaces_and_kek_history() {
        let (store, _) = new_store();
        assert_eq!(
            store.status().unwrap(),
            SealedStoreStatus {
                initialized: false,
                sealed: true,
                active_kek_id: None,
                kek_entries: 0,
                retired_keks: 0,
                registered_namespaces: 0,
            }
        );

        store.init(b"pw", &fast_opts()).unwrap();
        assert_eq!(
            store.status().unwrap(),
            SealedStoreStatus {
                initialized: true,
                sealed: false,
                active_kek_id: Some("kek-1".to_string()),
                kek_entries: 1,
                retired_keks: 0,
                registered_namespaces: 0,
            }
        );

        store.put("alpha", "k1", b"one", None).unwrap();
        store.put("beta", "k2", b"two", None).unwrap();
        store.seal();
        assert_eq!(
            store.status().unwrap(),
            SealedStoreStatus {
                initialized: true,
                sealed: true,
                active_kek_id: Some("kek-1".to_string()),
                kek_entries: 1,
                retired_keks: 0,
                registered_namespaces: 2,
            }
        );

        let rotation = store.rotate_kek(b"pw", b"new-pw").unwrap();
        assert_eq!(
            store.status().unwrap(),
            SealedStoreStatus {
                initialized: true,
                sealed: false,
                active_kek_id: Some(rotation.new_kek_id),
                kek_entries: 2,
                retired_keks: 1,
                registered_namespaces: 2,
            }
        );
    }

    #[test]
    fn wrong_password_is_rejected() {
        let (store, backend) = new_store();
        store.init(b"correct", &fast_opts()).unwrap();
        let store2 = SealedStore::new(Arc::clone(&backend));
        assert!(matches!(
            store2.unseal(b"wrong"),
            Err(SealedStoreError::BadPassword)
        ));
    }

    #[test]
    fn corrupt_body_is_detected() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"secret", None).unwrap();

        // Flip a bit in the ciphertext body, preserving metadata.
        let rec = backend.get("ns", "k").unwrap().unwrap();
        let mut bad_body = rec.body.clone();
        bad_body[0] ^= 0x01;
        let put = StoragePutInput::new(
            "ns".to_string(),
            "k".to_string(),
            SEALED_CONTENT_TYPE.to_string(),
            rec.metadata.clone(),
            bad_body,
        )
        .unwrap()
        .with_if_revision(Some(rec.revision));
        backend.put(put).unwrap();

        assert!(matches!(
            store.get("ns", "k"),
            Err(SealedStoreError::Tamper { .. })
        ));
    }

    #[test]
    fn swap_bodies_between_records_is_detected() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "a", b"aaaa", None).unwrap();
        store.put("ns", "b", b"bbbb", None).unwrap();

        let ra = backend.get("ns", "a").unwrap().unwrap();
        let rb = backend.get("ns", "b").unwrap().unwrap();

        // Copy a's body onto b (keeping b's metadata — bound to (ns,b)).
        let put = StoragePutInput::new(
            "ns".to_string(),
            "b".to_string(),
            SEALED_CONTENT_TYPE.to_string(),
            rb.metadata.clone(),
            ra.body.clone(),
        )
        .unwrap()
        .with_if_revision(Some(rb.revision));
        backend.put(put).unwrap();

        assert!(matches!(
            store.get("ns", "b"),
            Err(SealedStoreError::Tamper { .. })
        ));
    }

    #[test]
    fn rewriting_address_in_metadata_is_detected() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "a", b"body-a", None).unwrap();
        store.put("ns", "b", b"body-b", None).unwrap();

        let ra = backend.get("ns", "a").unwrap().unwrap();
        let rb_rev = backend.get("ns", "b").unwrap().unwrap().revision;
        let put = StoragePutInput::new(
            "ns".to_string(),
            "b".to_string(),
            SEALED_CONTENT_TYPE.to_string(),
            ra.metadata.clone(),
            ra.body.clone(),
        )
        .unwrap()
        .with_if_revision(Some(rb_rev));
        backend.put(put).unwrap();

        assert!(matches!(
            store.get("ns", "b"),
            Err(SealedStoreError::Tamper { .. })
        ));
    }

    #[test]
    fn double_init_is_rejected() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        assert!(matches!(
            store.init(b"pw", &fast_opts()),
            Err(SealedStoreError::AlreadyInitialized)
        ));
        assert!(matches!(
            store.init_with_kek(&[0xA5; KEY_LEN]),
            Err(SealedStoreError::AlreadyInitialized)
        ));

        let (store, _) = new_store();
        store.init_with_kek(&[0xA5; KEY_LEN]).unwrap();
        assert!(matches!(
            store.init(b"pw", &fast_opts()),
            Err(SealedStoreError::AlreadyInitialized)
        ));
    }

    #[test]
    fn ops_before_init_fail() {
        let (store, _) = new_store();
        assert!(matches!(
            store.unseal(b"pw"),
            Err(SealedStoreError::NotInitialized)
        ));
        assert!(matches!(
            store.get("ns", "k"),
            Err(SealedStoreError::Sealed)
        ));
        assert!(matches!(
            store.unseal_with_kek(&[0xA5; KEY_LEN]),
            Err(SealedStoreError::NotInitialized)
        ));
    }

    #[test]
    fn reserved_namespace_is_rejected() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        let err = store.put(RESERVED_NAMESPACE, "x", b"y", None).unwrap_err();
        assert!(matches!(err, SealedStoreError::Validation { .. }));
    }

    #[test]
    fn stale_if_revision_yields_conflict() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        let _rev1 = store.put("ns", "k", b"v1", None).unwrap();
        let rev2 = store.put("ns", "k", b"v2", None).unwrap();
        let _rev3 = store.put("ns", "k", b"v3", Some(rev2.clone())).unwrap();
        let err = store.put("ns", "k", b"v4", Some(rev2)).unwrap_err();
        assert!(matches!(
            err,
            SealedStoreError::Storage(StorageError::Conflict { .. })
        ));
    }

    #[test]
    fn put_if_absent_never_overwrites_an_existing_record() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        let revision = store.put_if_absent("ns", "k", b"original").unwrap();

        let error = store.put_if_absent("ns", "k", b"replacement").unwrap_err();

        assert!(matches!(
            error,
            SealedStoreError::Storage(StorageError::Conflict { .. })
        ));
        let record = store.get("ns", "k").unwrap().unwrap();
        assert_eq!(record.revision, revision);
        assert_eq!(&*record.plaintext, b"original");
    }

    #[test]
    fn empty_plaintext_roundtrip() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "e", b"", None).unwrap();
        let got = store.get("ns", "e").unwrap().unwrap();
        assert!(got.plaintext.is_empty());
    }

    #[test]
    fn large_plaintext_roundtrip() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        let big = vec![0xACu8; 256 * 1024]; // 256 KiB
        store.put("ns", "big", &big, None).unwrap();
        let got = store.get("ns", "big").unwrap().unwrap();
        assert_eq!(&*got.plaintext, &big[..]);
    }

    #[test]
    fn delete_removes_record() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"v", None).unwrap();
        store.delete("ns", "k", None).unwrap();
        assert!(backend.get("ns", "k").unwrap().is_none());
    }

    #[test]
    fn delete_requires_unseal() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"v", None).unwrap();
        store.seal();
        assert!(matches!(
            store.delete("ns", "k", None),
            Err(SealedStoreError::Sealed)
        ));
    }

    #[test]
    fn list_returns_stats_without_decrypting() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "a", b"aa", None).unwrap();
        store.put("ns", "b", b"bbbbbbbb", None).unwrap();
        let stats = store
            .list(
                "ns",
                StorageListOptions {
                    prefix: None,
                    recursive: true,
                    page_size: Some(10),
                    cursor: None,
                },
            )
            .unwrap();
        assert_eq!(stats.len(), 2);
        for s in &stats {
            assert_eq!(s.kek_id, "kek-1");
            assert!(s.ciphertext_len > 0);
        }
    }

    #[test]
    fn list_page_keeps_the_backend_cursor() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        for key in ["a", "b", "c"] {
            store.put("ns", key, b"v", None).unwrap();
        }
        let page = |cursor| {
            store
                .list_page(
                    "ns",
                    StorageListOptions {
                        prefix: None,
                        recursive: false,
                        page_size: Some(2),
                        cursor,
                    },
                )
                .unwrap()
        };
        let first = page(None);
        assert_eq!(first.records.len(), 2);
        assert_eq!(first.next_cursor.as_deref(), Some("b"));
        let second = page(first.next_cursor);
        assert_eq!(second.records.len(), 1);
        assert_eq!(second.records[0].key, "c");
        assert_eq!(second.next_cursor, None);

        store.seal();
        assert!(matches!(
            store.list_page("ns", StorageListOptions::default()),
            Err(SealedStoreError::Sealed)
        ));
    }

    #[test]
    fn summarize_returns_redacted_envelope_metadata() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        let secret = b"super-secret-value-that-must-not-appear-in-debug";
        let rev = store.put("ns", "k", secret, None).unwrap();

        let summary = store.summarize("ns", "k").unwrap().unwrap();
        assert_eq!(summary.namespace, "ns");
        assert_eq!(summary.key, "k");
        assert_eq!(summary.revision, rev);
        assert_eq!(summary.schema_version, SEALED_RECORD_VERSION);
        assert_eq!(summary.aead, "xchacha20poly1305");
        assert_eq!(summary.ciphertext_len, secret.len());
        assert_eq!(summary.body_nonce_len, NONCE_LEN);
        assert_eq!(summary.body_tag_len, TAG_LEN);
        // The first put makes generation 1, so the AAD is the v2 one (F2).
        assert_eq!(summary.body_aad_len, record_aad_v2("ns", "k", 1).len());
        assert_eq!(summary.wrapped_dek_len, KEY_LEN);
        assert_eq!(summary.wrap_nonce_len, NONCE_LEN);
        assert_eq!(summary.wrap_tag_len, TAG_LEN);
        assert_eq!(summary.kek_id, "kek-1");

        let record = backend.get("ns", "k").unwrap().unwrap();
        let debug = format!("{summary:?}");
        assert!(!debug.contains("super-secret-value"));
        assert!(!debug.contains(&hex_encode(&record.body)));
    }

    #[test]
    fn summarize_requires_unseal_and_handles_missing_records() {
        let (store, _) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.seal();
        assert!(matches!(
            store.summarize("ns", "missing"),
            Err(SealedStoreError::Sealed)
        ));

        store.unseal(b"pw").unwrap();
        assert!(store.summarize("ns", "missing").unwrap().is_none());
    }

    #[test]
    fn sealed_record_metadata_rejects_unknown_envelope_format() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"value", None).unwrap();

        let record = backend.get("ns", "k").unwrap().unwrap();
        let mut obj = match record.metadata.clone() {
            JsonValue::Object(o) => o,
            _ => panic!("bad metadata"),
        };
        for (k, v) in obj.iter_mut() {
            if k == "aead" {
                *v = JsonValue::String("unknown-aead".to_string());
            }
        }
        let tampered = JsonValue::Object(obj);
        let put = StoragePutInput::new(
            "ns".to_string(),
            "k".to_string(),
            SEALED_CONTENT_TYPE.to_string(),
            tampered,
            record.body,
        )
        .unwrap()
        .with_if_revision(Some(record.revision));
        backend.put(put).unwrap();

        assert!(matches!(
            store.summarize("ns", "k"),
            Err(SealedStoreError::Validation { .. })
        ));
    }

    #[test]
    fn new_store_instance_can_unseal_same_backend() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"hello", None).unwrap();
        drop(store);
        let store2 = SealedStore::new(Arc::clone(&backend));
        assert!(store2.is_sealed());
        store2.unseal(b"pw").unwrap();
        assert_eq!(
            &*store2.get("ns", "k").unwrap().unwrap().plaintext,
            b"hello"
        );
    }

    #[test]
    fn hex_roundtrip() {
        assert_eq!(hex_encode(&[0x00, 0xff, 0xde, 0xad]), "00ffdead");
        assert_eq!(
            hex_decode("00ffDEAD").unwrap(),
            vec![0x00, 0xff, 0xde, 0xad]
        );
        assert!(hex_decode("abc").is_err());
        assert!(hex_decode("xz").is_err());
    }

    #[test]
    fn public_defaults_and_error_display_paths_are_covered() {
        let defaults = InitOptions::default();
        assert_eq!(defaults.argon2id_time_cost, DEFAULT_ARGON2_TIME_COST);
        assert_eq!(defaults.argon2id_memory_kib, DEFAULT_ARGON2_MEMORY_KIB);
        assert_eq!(defaults.argon2id_parallelism, DEFAULT_ARGON2_PARALLELISM);
        assert!(defaults.salt_override.is_none());

        let errors = [
            SealedStoreError::AlreadyInitialized,
            SealedStoreError::NotInitialized,
            SealedStoreError::Sealed,
            SealedStoreError::BadPassword,
            SealedStoreError::InvalidKek,
            SealedStoreError::Tamper {
                namespace: "ns".into(),
                key: "key".into(),
            },
            SealedStoreError::Storage(StorageError::Backend {
                message: "backend unavailable".into(),
            }),
            SealedStoreError::Crypto("primitive rejected input".into()),
            SealedStoreError::Validation {
                field: "field".into(),
                message: "invalid".into(),
            },
        ];
        for error in errors {
            assert!(error.to_string().starts_with("vault-sealed-store"));
        }
    }

    #[test]
    fn manifest_parser_rejects_each_bounded_field_shape() {
        let mut missing_keks = valid_manifest_metadata();
        let JsonValue::Object(fields) = &mut missing_keks else {
            unreachable!();
        };
        fields.retain(|(name, _)| name != "keks");
        assert_validation_field(Manifest::parse(&missing_keks).unwrap_err(), "keks");

        let mut non_array_keks = valid_manifest_metadata();
        set_object_field(
            &mut non_array_keks,
            "keks",
            JsonValue::String("not-an-array".into()),
        );
        assert_validation_field(Manifest::parse(&non_array_keks).unwrap_err(), "keks");

        let mut empty_keks = valid_manifest_metadata();
        set_object_field(&mut empty_keks, "keks", JsonValue::Array(Vec::new()));
        assert_validation_field(Manifest::parse(&empty_keks).unwrap_err(), "keks");

        let mut unsupported_status = valid_manifest_metadata();
        set_first_kek_field(
            &mut unsupported_status,
            "status",
            JsonValue::String("unknown".into()),
        );
        assert_validation_field(Manifest::parse(&unsupported_status).unwrap_err(), "status");

        let mut invalid_salt_hex = valid_manifest_metadata();
        set_first_kek_field(
            &mut invalid_salt_hex,
            "salt",
            JsonValue::String("zz".into()),
        );
        assert_validation_field(Manifest::parse(&invalid_salt_hex).unwrap_err(), "salt");

        let mut short_salt = valid_manifest_metadata();
        set_first_kek_field(
            &mut short_salt,
            "salt",
            JsonValue::String(hex_encode(&[0; 7])),
        );
        assert_validation_field(Manifest::parse(&short_salt).unwrap_err(), "salt");

        let mut invalid_verifier_hex = valid_manifest_metadata();
        set_first_kek_field(
            &mut invalid_verifier_hex,
            "verifier_ct",
            JsonValue::String("zz".into()),
        );
        assert_validation_field(
            Manifest::parse(&invalid_verifier_hex).unwrap_err(),
            "verifier_ct",
        );

        let mut short_verifier = valid_manifest_metadata();
        set_first_kek_field(
            &mut short_verifier,
            "verifier_ct",
            JsonValue::String(hex_encode(&[0; 1])),
        );
        assert_validation_field(Manifest::parse(&short_verifier).unwrap_err(), "verifier_ct");
    }

    #[test]
    fn sealed_metadata_and_json_helpers_reject_malformed_values() {
        let valid = || {
            build_sealed_metadata(&SealedRecordMeta {
                version: 0,
                aead: String::new(),
                body_nonce: [0; NONCE_LEN],
                body_tag: [0; TAG_LEN],
                body_aad: b"aad".to_vec(),
                wrapped_dek: vec![0; KEY_LEN],
                wrap_nonce: [0; NONCE_LEN],
                wrap_tag: [0; TAG_LEN],
                kek_id: "kek-1".to_string(),
                generation: None,
            })
        };
        let parse_error = |metadata: &JsonValue| match SealedRecordMeta::parse(metadata) {
            Ok(_) => panic!("expected malformed metadata to fail"),
            Err(error) => error,
        };

        let mut bad_version = valid();
        set_object_field(
            &mut bad_version,
            "vault_sealed_version",
            JsonValue::Number(JsonNumber::Integer(999)),
        );
        assert_validation_field(parse_error(&bad_version), "vault_sealed_version");

        let mut invalid_aad = valid();
        set_object_field(&mut invalid_aad, "body_aad", JsonValue::String("zz".into()));
        assert_validation_field(parse_error(&invalid_aad), "body_aad");

        let mut invalid_wrapped_dek = valid();
        set_object_field(
            &mut invalid_wrapped_dek,
            "wrapped_dek",
            JsonValue::String("zz".into()),
        );
        assert_validation_field(parse_error(&invalid_wrapped_dek), "wrapped_dek");

        assert_validation_field(
            expect_object(&JsonValue::Null, "object").unwrap_err(),
            "object",
        );
        assert_validation_field(get_field(&[], "missing").unwrap_err(), "missing");
        let number_field = [(
            "value".to_string(),
            JsonValue::Number(JsonNumber::Integer(1)),
        )];
        assert_validation_field(get_string(&number_field, "value").unwrap_err(), "value");
        let negative_field = [(
            "value".to_string(),
            JsonValue::Number(JsonNumber::Integer(-1)),
        )];
        assert_validation_field(get_u32(&negative_field, "value").unwrap_err(), "value");
        assert_validation_field(get_u64(&negative_field, "value").unwrap_err(), "value");
        assert_validation_field(hex_decode_fixed::<2>("zz", "hex").unwrap_err(), "hex");
        assert_validation_field(hex_decode_fixed::<2>("00", "hex").unwrap_err(), "hex");
    }

    #[test]
    fn kek_id_increments_and_rejects_overflow() {
        let e = [
            KekEntry {
                id: "kek-1".into(),
                status: "retired",
                source: KekSource::PasswordDerived,
                salt: Some(vec![0; 16]),
                verifier_nonce: [0; NONCE_LEN],
                verifier_tag: [0; TAG_LEN],
                verifier_ct: vec![],
            },
            KekEntry {
                id: "kek-7".into(),
                status: "active",
                source: KekSource::PasswordDerived,
                salt: Some(vec![0; 16]),
                verifier_nonce: [0; NONCE_LEN],
                verifier_tag: [0; TAG_LEN],
                verifier_ct: vec![],
            },
        ];
        assert_eq!(next_kek_id(&e).unwrap(), "kek-8");
        assert_eq!(next_kek_id(&[]).unwrap(), "kek-1");

        // Overflow case.
        let overflow = [KekEntry {
            id: format!("kek-{}", u64::MAX),
            status: "active",
            source: KekSource::PasswordDerived,
            salt: Some(vec![0; 16]),
            verifier_nonce: [0; NONCE_LEN],
            verifier_tag: [0; TAG_LEN],
            verifier_ct: vec![],
        }];
        assert!(matches!(
            next_kek_id(&overflow),
            Err(SealedStoreError::Validation { .. })
        ));
    }

    #[test]
    fn init_validates_short_salt_override() {
        let (store, _) = new_store();
        let mut opts = fast_opts();
        opts.salt_override = Some(vec![1, 2, 3]); // too short
        let err = store.init(b"pw", &opts).unwrap_err();
        assert!(matches!(err, SealedStoreError::Validation { .. }));
    }

    #[test]
    fn init_validates_bogus_argon2_params() {
        let (store, _) = new_store();
        let mut opts = fast_opts();
        opts.argon2id_time_cost = 999; // way above the max
        let err = store.init(b"pw", &opts).unwrap_err();
        assert!(matches!(err, SealedStoreError::Validation { .. }));

        let (store, _) = new_store();
        let mut opts = fast_opts();
        opts.argon2id_parallelism = 0;
        let err = store.init(b"pw", &opts).unwrap_err();
        assert!(matches!(err, SealedStoreError::Validation { .. }));

        let (store, _) = new_store();
        let mut opts = fast_opts();
        // memory must be >= 8 * parallelism
        opts.argon2id_parallelism = 4;
        opts.argon2id_memory_kib = 8;
        let err = store.init(b"pw", &opts).unwrap_err();
        assert!(matches!(err, SealedStoreError::Validation { .. }));
    }

    #[test]
    fn tampered_manifest_with_huge_memory_is_rejected() {
        // Attacker rewrites the manifest to demand 4 GiB of RAM at unseal
        // time. parse() must reject it before we call Argon2.
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        let mf = backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)
            .unwrap()
            .unwrap();
        let mut obj = match mf.metadata.clone() {
            JsonValue::Object(o) => o,
            _ => panic!("bad manifest"),
        };
        for (k, v) in obj.iter_mut() {
            if k == "kdf_memory_cost_kib" {
                *v = JsonValue::Number(JsonNumber::Integer(8 * 1024 * 1024)); // 8 GiB
            }
        }
        let tampered = JsonValue::Object(obj);
        let put = StoragePutInput::new(
            RESERVED_NAMESPACE.to_string(),
            MANIFEST_KEY.to_string(),
            MANIFEST_CONTENT_TYPE.to_string(),
            tampered,
            Vec::new(),
        )
        .unwrap()
        .with_if_revision(Some(mf.revision));
        backend.put(put).unwrap();

        let store2 = SealedStore::new(Arc::clone(&backend));
        let err = store2.unseal(b"pw").unwrap_err();
        assert!(matches!(err, SealedStoreError::Validation { .. }));
    }

    #[test]
    fn rotate_kek_rewraps_records_across_namespaces() {
        let (store, _) = new_store();
        store.init(b"old-pw", &fast_opts()).unwrap();
        store.put("ns1", "a", b"A", None).unwrap();
        store.put("ns2", "b", b"B", None).unwrap();

        let report = store.rotate_kek(b"old-pw", b"new-pw").unwrap();
        assert_eq!(report.records_rewrapped, 2);
        assert_eq!(report.new_kek_id, "kek-2");

        // Reads under new KEK (just unsealed in-place by rotate) must succeed.
        assert_eq!(&*store.get("ns1", "a").unwrap().unwrap().plaintext, b"A");
        assert_eq!(&*store.get("ns2", "b").unwrap().unwrap().plaintext, b"B");

        // Sealing + unseal with new password.
        store.seal();
        store.unseal(b"new-pw").unwrap();
        assert_eq!(&*store.get("ns1", "a").unwrap().unwrap().plaintext, b"A");

        // Old password must also still unseal (retired entry remains), and
        // under that unseal, get() returns Tamper because records now have
        // kek-2 but unsealed is kek-1.
        store.seal();
        store.unseal(b"old-pw").unwrap();
        match store.get("ns1", "a") {
            Err(SealedStoreError::Tamper { .. }) => {}
            other => panic!("expected Tamper, got {:?}", other.map(|_| "Ok(..)").err()),
        }
    }

    #[test]
    fn namespace_registry_filters_reserved() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("real", "k", b"v", None).unwrap();

        // Check what ended up in the side record.
        let rec = backend
            .get(RESERVED_NAMESPACE, NAMESPACES_KEY)
            .unwrap()
            .unwrap();
        let names = parse_namespaces(&rec.metadata);
        assert_eq!(names, vec!["real".to_string()]);

        // Even if an attacker injects the reserved namespace into the list,
        // list_registered_namespaces must filter it out.
        let tampered = build_namespaces_json(&["real".to_string(), RESERVED_NAMESPACE.to_string()]);
        let put = StoragePutInput::new(
            RESERVED_NAMESPACE.to_string(),
            NAMESPACES_KEY.to_string(),
            NAMESPACES_CONTENT_TYPE.to_string(),
            tampered,
            Vec::new(),
        )
        .unwrap()
        .with_if_revision(Some(rec.revision));
        backend.put(put).unwrap();

        let names = store.list_registered_namespaces().unwrap();
        assert_eq!(names, vec!["real".to_string()]);
    }

    #[test]
    fn put_on_sealed_store_does_not_write_registry() {
        // Regression guard: an unauthenticated caller holding a handle on a
        // sealed store must not be able to mutate the namespace registry by
        // spamming puts that each return Sealed.
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.seal();
        let err = store.put("evilns", "k", b"v", None).unwrap_err();
        assert!(matches!(err, SealedStoreError::Sealed));
        // Registry must not exist (init never created one).
        assert!(backend
            .get(RESERVED_NAMESPACE, NAMESPACES_KEY)
            .unwrap()
            .is_none());
    }

    #[test]
    fn manifest_with_duplicate_kek_ids_is_rejected() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        let mf = backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)
            .unwrap()
            .unwrap();
        let mut obj = match mf.metadata.clone() {
            JsonValue::Object(o) => o,
            _ => panic!("bad manifest"),
        };
        for (k, v) in obj.iter_mut() {
            if k == "keks" {
                if let JsonValue::Array(arr) = v {
                    if let Some(first) = arr.first().cloned() {
                        arr.push(first); // inject duplicate
                    }
                }
            }
        }
        let tampered = JsonValue::Object(obj);
        let put = StoragePutInput::new(
            RESERVED_NAMESPACE.to_string(),
            MANIFEST_KEY.to_string(),
            MANIFEST_CONTENT_TYPE.to_string(),
            tampered,
            Vec::new(),
        )
        .unwrap()
        .with_if_revision(Some(mf.revision));
        backend.put(put).unwrap();

        let store2 = SealedStore::new(Arc::clone(&backend));
        let err = store2.unseal(b"pw").unwrap_err();
        assert!(matches!(err, SealedStoreError::Validation { .. }));
    }

    #[test]
    fn manifest_with_unknown_kek_source_is_rejected() {
        let (store, backend) = new_store();
        store.init_with_kek(&[0xA5; KEY_LEN]).unwrap();
        let manifest = backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)
            .unwrap()
            .unwrap();
        let mut metadata = match manifest.metadata.clone() {
            JsonValue::Object(fields) => fields,
            _ => panic!("bad manifest"),
        };
        for (_, value) in metadata.iter_mut().filter(|(key, _)| key == "keks") {
            if let JsonValue::Array(entries) = value {
                if let Some(JsonValue::Object(fields)) = entries.first_mut() {
                    for (_, value) in fields.iter_mut().filter(|(key, _)| key == "source") {
                        *value = JsonValue::String("attacker-controlled".to_string());
                    }
                }
            }
        }
        let update = StoragePutInput::new(
            RESERVED_NAMESPACE.to_string(),
            MANIFEST_KEY.to_string(),
            MANIFEST_CONTENT_TYPE.to_string(),
            JsonValue::Object(metadata),
            Vec::new(),
        )
        .unwrap()
        .with_if_revision(Some(manifest.revision));
        backend.put(update).unwrap();

        let reopened = SealedStore::new(Arc::clone(&backend));
        assert!(matches!(
            reopened.unseal_with_kek(&[0xA5; KEY_LEN]),
            Err(SealedStoreError::Validation { ref field, .. }) if field == "source"
        ));
    }

    // ---- freshness (VLT01 F1-F10) ------------------------------------------
    //
    // Each attack below is performed exactly the way someone with write
    // access to the storage directory, and no KEK, would perform it: by
    // copying record files aside and putting them back.

    /// A record file, exactly as stored, to put back later.
    fn snapshot(backend: &Arc<dyn StorageBackend>, namespace: &str, key: &str) -> StoragePutInput {
        let record = backend.get(namespace, key).unwrap().unwrap();
        StoragePutInput::new(
            record.namespace,
            record.key,
            record.content_type,
            record.metadata,
            record.body,
        )
        .unwrap()
    }

    fn restore(backend: &Arc<dyn StorageBackend>, file: StoragePutInput) {
        backend.put(file).unwrap();
    }

    fn assert_tamper(result: Result<Option<SealedRecord>, SealedStoreError>) {
        assert!(
            matches!(result, Err(SealedStoreError::Tamper { .. })),
            "expected Tamper, got {:?}",
            result.map(|record| record.map(|record| record.plaintext.to_vec()))
        );
    }

    fn read(store: &SealedStore, namespace: &str, key: &str) -> Vec<u8> {
        store
            .get(namespace, key)
            .unwrap()
            .unwrap()
            .plaintext
            .to_vec()
    }

    /// Write a format-1 record the way the previous release did: no
    /// generation, the v1 AAD, and no index.
    fn put_v1(store: &SealedStore, backend: &Arc<dyn StorageBackend>, key: &str, plaintext: &[u8]) {
        let guard = store.state.lock().unwrap();
        let unsealed = guard.unsealed.as_ref().unwrap();
        let (metadata, body, _) =
            seal_envelope(unsealed, "ns", key, &record_aad("ns", key), None, plaintext).unwrap();
        backend
            .put(
                StoragePutInput::new(
                    "ns".to_string(),
                    key.to_string(),
                    SEALED_CONTENT_TYPE.to_string(),
                    metadata,
                    body,
                )
                .unwrap(),
            )
            .unwrap();
    }

    fn index_of(store: &SealedStore, namespace: &str) -> Option<FreshnessIndex> {
        let guard = store.state.lock().unwrap();
        let unsealed = guard.unsealed.as_ref().unwrap();
        store
            .load_index(unsealed, namespace)
            .unwrap()
            .map(|(index, _)| index)
    }

    #[test]
    fn rolling_back_one_record_file_is_tamper() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"leaked", None).unwrap();
        let old = snapshot(&backend, "ns", "k");
        store.put("ns", "k", b"rotated", None).unwrap();
        assert_eq!(read(&store, "ns", "k"), b"rotated");

        restore(&backend, old);
        assert_tamper(store.get("ns", "k"));
    }

    #[test]
    fn resurrecting_a_deleted_record_is_tamper() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"revoked", None).unwrap();
        let old = snapshot(&backend, "ns", "k");
        store.delete("ns", "k", None).unwrap();
        assert!(store.get("ns", "k").unwrap().is_none());

        restore(&backend, old);
        assert_tamper(store.get("ns", "k"));

        // A put after the delete continues past the tombstone and is read.
        store.put("ns", "k", b"reissued", None).unwrap();
        assert_eq!(read(&store, "ns", "k"), b"reissued");
        let entry = index_of(&store, "ns").unwrap().entries["k"];
        // Generation 1, tombstone 2 (one past, F5), then the put at 3.
        assert_eq!((entry.generation, entry.state), (3, EntryState::Live));
    }

    #[test]
    fn a_record_ahead_of_its_index_is_accepted() {
        // A crash between the record write and the index write (F4).
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"one", None).unwrap();
        let index_then = snapshot(&backend, RESERVED_NAMESPACE, &freshness_key("ns"));
        store.put("ns", "k", b"two", None).unwrap();

        restore(&backend, index_then);
        // A restarted process has no epoch floor, sees the stale index, and
        // still reads the newer record.
        let restarted = SealedStore::new(Arc::clone(&backend));
        restarted.unseal(b"pw").unwrap();
        assert_eq!(read(&restarted, "ns", "k"), b"two");
        // The next put absorbs it first, then moves past it (F4).
        restarted.put("ns", "k", b"three", None).unwrap();
        assert_eq!(read(&restarted, "ns", "k"), b"three");
        assert_eq!(
            index_of(&restarted, "ns").unwrap().entries["k"].generation,
            3
        );
    }

    fn reopen(backend: &Arc<dyn StorageBackend>) -> SealedStore {
        let store = SealedStore::new(Arc::clone(backend));
        store.unseal(b"pw").unwrap();
        store
    }

    #[test]
    fn a_put_after_a_lost_index_update_never_reuses_a_generation() {
        // Review finding HIGH-1(b). "two" is written but its index update
        // is lost; "three" must not be written at the same generation, or
        // putting the saved "two" back would pass.
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"one", None).unwrap();
        let index_then = snapshot(&backend, RESERVED_NAMESPACE, &freshness_key("ns"));
        store.put("ns", "k", b"two", None).unwrap();
        restore(&backend, index_then); // the lost index update
        let two = snapshot(&backend, "ns", "k");

        let restarted = reopen(&backend);
        restarted.put("ns", "k", b"three", None).unwrap();
        restore(&backend, two);
        assert_tamper(reopen(&backend).get("ns", "k"));
    }

    #[test]
    fn a_reused_generation_is_told_apart_by_its_tag() {
        // Review finding HIGH-1(a), with the newer record deleted first, so
        // there is nothing on disk for the next put to absorb: the put does
        // reuse generation 2, and only the pinned tag tells the files apart.
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"one", None).unwrap();
        let index_then = snapshot(&backend, RESERVED_NAMESPACE, &freshness_key("ns"));
        store.put("ns", "k", b"two", None).unwrap();
        let two = snapshot(&backend, "ns", "k");
        restore(&backend, index_then);
        backend.delete("ns", "k", None).unwrap();

        let restarted = reopen(&backend);
        restarted.put("ns", "k", b"three", None).unwrap();
        assert_eq!(
            index_of(&restarted, "ns").unwrap().entries["k"].generation,
            2
        );
        restore(&backend, two);
        assert_tamper(reopen(&backend).get("ns", "k"));
    }

    #[test]
    fn an_old_index_put_back_is_not_laundered_by_a_later_write() {
        // Review finding MEDIUM-2: restore the index, let the owner write
        // something else, then restore the target's old record.
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "j", b"j-leaked", None).unwrap();
        let index_then = snapshot(&backend, RESERVED_NAMESPACE, &freshness_key("ns"));
        let leaked = snapshot(&backend, "ns", "j");
        store.put("ns", "j", b"j-rotated", None).unwrap();

        // In the same process the epoch floor refuses the old index outright.
        restore(&backend, index_then);
        assert!(matches!(
            store.put("ns", "other", b"x", None),
            Err(SealedStoreError::Tamper { .. })
        ));

        // After a restart there is no floor. The write absorbs j's newer
        // record before building on the stale index, so the floor it seals
        // is j's real one.
        let restarted = reopen(&backend);
        restarted.put("ns", "other", b"x", None).unwrap();
        restore(&backend, leaked);
        assert_tamper(reopen(&backend).get("ns", "j"));
    }

    #[test]
    fn a_forged_generation_cannot_set_a_tombstone() {
        // Review finding L1: delete must not take its tombstone from
        // plaintext metadata.
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"v", None).unwrap();
        let record = backend.get("ns", "k").unwrap().unwrap();
        let mut metadata = record.metadata.clone();
        set_object_field(
            &mut metadata,
            "generation",
            JsonValue::Number(JsonNumber::Integer(i64::MAX)),
        );
        backend
            .put(
                StoragePutInput::new(
                    record.namespace,
                    record.key,
                    record.content_type,
                    metadata,
                    record.body,
                )
                .unwrap(),
            )
            .unwrap();
        store.delete("ns", "k", None).unwrap();
        // One past the authentic generation 1, not the forged one.
        assert_eq!(index_of(&store, "ns").unwrap().entries["k"].generation, 2);
        store.put("ns", "k", b"after", None).unwrap();
        assert_eq!(read(&store, "ns", "k"), b"after");
    }

    #[test]
    fn one_unopenable_v1_file_does_not_block_writes() {
        // Review finding L4.
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        put_v1(&store, &backend, "good", b"fine");
        put_v1(&store, &backend, "bad", b"soon corrupt");
        let bad = backend.get("ns", "bad").unwrap().unwrap();
        let mut body = bad.body.clone();
        body[0] ^= 1;
        backend
            .put(
                StoragePutInput::new(bad.namespace, bad.key, bad.content_type, bad.metadata, body)
                    .unwrap(),
            )
            .unwrap();

        store.put("ns", "new", b"x", None).unwrap();
        assert_eq!(read(&store, "ns", "good"), b"fine");
        assert_tamper(store.get("ns", "bad"));
        store.delete("ns", "bad", None).unwrap();
        store.put("ns", "bad", b"replaced", None).unwrap();
        assert_eq!(read(&store, "ns", "bad"), b"replaced");
    }

    #[test]
    fn deleting_the_index_of_a_migrated_namespace_is_tamper() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"value", None).unwrap();
        backend
            .delete(RESERVED_NAMESPACE, &freshness_key("ns"), None)
            .unwrap();

        // A fresh store instance, so nothing is cached.
        let reopened = SealedStore::new(Arc::clone(&backend));
        reopened.unseal(b"pw").unwrap();
        assert_tamper(reopened.get("ns", "k"));
        // And a write refuses to adopt what it finds (F6 step 1).
        assert!(matches!(
            reopened.put("ns", "other", b"x", None),
            Err(SealedStoreError::Tamper { .. })
        ));
    }

    #[test]
    fn a_tampered_index_is_tamper_not_absent() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"value", None).unwrap();
        let record = backend
            .get(RESERVED_NAMESPACE, &freshness_key("ns"))
            .unwrap()
            .unwrap();
        let mut body = record.body.clone();
        body[0] ^= 1;
        backend
            .put(
                StoragePutInput::new(
                    record.namespace,
                    record.key,
                    record.content_type,
                    record.metadata,
                    body,
                )
                .unwrap(),
            )
            .unwrap();
        let reopened = SealedStore::new(Arc::clone(&backend));
        reopened.unseal(b"pw").unwrap();
        assert_tamper(reopened.get("ns", "k"));
    }

    #[test]
    fn format_one_records_read_until_migrated_and_never_after() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        put_v1(&store, &backend, "old", b"from the last release");
        let original_v1 = snapshot(&backend, "ns", "old");

        // No index yet: the v1 record reads as before.
        assert!(index_of(&store, "ns").is_none());
        assert_eq!(read(&store, "ns", "old"), b"from the last release");

        // The first write migrates the namespace (F6).
        store.put("ns", "new", b"fresh", None).unwrap();
        let index = index_of(&store, "ns").unwrap();
        assert!(!index.has_legacy());
        assert_eq!(index.entries["old"].generation, 1);
        let migrated = backend.get("ns", "old").unwrap().unwrap();
        assert_eq!(
            SealedRecordMeta::parse(&migrated.metadata)
                .unwrap()
                .generation,
            Some(1)
        );
        assert_eq!(read(&store, "ns", "old"), b"from the last release");
        assert_eq!(read(&store, "ns", "new"), b"fresh");

        // Putting the pre-migration v1 file back is now a rollback.
        restore(&backend, original_v1);
        assert_tamper(store.get("ns", "old"));
    }

    #[test]
    fn a_crash_mid_migration_is_resumed_by_the_next_write() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        put_v1(&store, &backend, "a", b"alpha");
        put_v1(&store, &backend, "b", b"beta");
        // Steps 1-2 only, then "crash".
        {
            let guard = store.state.lock().unwrap();
            store
                .begin_migration(guard.unsealed.as_ref().unwrap(), "ns")
                .unwrap();
        }
        assert!(index_of(&store, "ns").unwrap().has_legacy());
        // Legacy entries still read their v1 records.
        assert_eq!(read(&store, "ns", "a"), b"alpha");

        store.put("ns", "c", b"gamma", None).unwrap();
        let index = index_of(&store, "ns").unwrap();
        assert!(!index.has_legacy());
        for key in ["a", "b", "c"] {
            assert_eq!(index.entries[key].state, EntryState::Live, "{key}");
        }
        assert_eq!(read(&store, "ns", "b"), b"beta");
    }

    #[test]
    fn a_delete_that_fails_its_revision_check_leaves_the_record_readable() {
        let (store, _backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        let first = store.put("ns", "k", b"one", None).unwrap();
        store.put("ns", "k", b"two", None).unwrap();
        assert!(matches!(
            store.delete("ns", "k", Some(first)),
            Err(SealedStoreError::Storage(StorageError::Conflict { .. }))
        ));
        assert_eq!(read(&store, "ns", "k"), b"two");
    }

    #[test]
    fn deleting_an_unknown_key_still_tombstones_it() {
        let (store, _backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"v", None).unwrap();
        store.delete("ns", "never-seen", None).unwrap();
        let entry = index_of(&store, "ns").unwrap().entries["never-seen"];
        assert_eq!((entry.generation, entry.state), (1, EntryState::Tombstone));
    }

    /// A put whose record landed but whose index update did not: put
    /// `value`, then put the index back as it was before.
    fn put_with_lost_index_update(
        store: &SealedStore,
        backend: &Arc<dyn StorageBackend>,
        key: &str,
        value: &[u8],
    ) {
        let index_before = snapshot(backend, RESERVED_NAMESPACE, &freshness_key("ns"));
        store.put("ns", key, value, None).unwrap();
        restore(backend, index_before);
    }

    #[test]
    fn a_delete_cannot_be_undone_by_hiding_an_uncommitted_record() {
        // Re-review finding: the uncommitted record is at n + 1, it is hidden
        // while the delete runs, then put back. Tombstoning at n would let it
        // through; F5 tombstones at n + 1.
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"one", None).unwrap();
        put_with_lost_index_update(&store, &backend, "k", b"uncommitted");
        let restarted = reopen(&backend); // no epoch floor across a restart
        let hidden = snapshot(&backend, "ns", "k");
        backend.delete("ns", "k", None).unwrap();
        restarted.delete("ns", "k", None).unwrap();
        restore(&backend, hidden);
        assert_tamper(restarted.get("ns", "k"));
        restarted.put("ns", "other", b"x", None).unwrap();
        assert_tamper(reopen(&backend).get("ns", "k"));
    }

    #[test]
    fn a_delete_of_a_new_key_cannot_be_undone_by_hiding_its_record() {
        // The same, for a key the index has never seen.
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "anchor", b"a", None).unwrap();
        put_with_lost_index_update(&store, &backend, "fresh", b"uncommitted");
        let restarted = reopen(&backend);
        let hidden = snapshot(&backend, "ns", "fresh");
        backend.delete("ns", "fresh", None).unwrap();
        restarted.delete("ns", "fresh", None).unwrap();
        restore(&backend, hidden);
        assert_tamper(restarted.get("ns", "fresh"));
    }

    #[test]
    fn a_planted_unknown_kek_id_does_not_block_migration() {
        // Third-review LOW-1: only a KEK the manifest lists as retired stops
        // migration.
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        put_v1(&store, &backend, "planted", b"junk");
        let planted = backend.get("ns", "planted").unwrap().unwrap();
        let mut metadata = planted.metadata.clone();
        set_object_field(&mut metadata, "kek_id", JsonValue::String("kek-999".into()));
        backend
            .put(
                StoragePutInput::new(
                    planted.namespace,
                    planted.key,
                    planted.content_type,
                    metadata,
                    planted.body,
                )
                .unwrap(),
            )
            .unwrap();
        store.put("ns", "real", b"ok", None).unwrap();
        assert_eq!(read(&store, "ns", "real"), b"ok");
        assert_tamper(store.get("ns", "planted"));
    }

    #[test]
    fn migration_waits_for_an_interrupted_rotation() {
        // Re-review LOW-b: a v1 record under a retired KEK is recoverable by
        // resuming rotation, so migration must not strand it.
        let (store, backend) = new_store();
        store.init(b"old", &fast_opts()).unwrap();
        // Written straight to the backend, so the namespace is unregistered
        // and the rotation below leaves this record under the old KEK.
        put_v1(&store, &backend, "stranded", b"keep me");
        store.rotate_kek(b"old", b"new").unwrap();
        assert!(matches!(
            store.put("ns", "new", b"x", None),
            Err(SealedStoreError::Validation { ref field, .. }) if field == "kek_id"
        ));
        assert!(index_of(&store, "ns").unwrap().has_legacy());
    }

    #[test]
    fn rotation_rewraps_the_freshness_index() {
        let (store, backend) = new_store();
        store.init(b"old", &fast_opts()).unwrap();
        store.put("ns", "k", b"value", None).unwrap();
        let report = store.rotate_kek(b"old", b"new").unwrap();

        let index = backend
            .get(RESERVED_NAMESPACE, &freshness_key("ns"))
            .unwrap()
            .unwrap();
        assert_eq!(
            SealedRecordMeta::parse(&index.metadata).unwrap().kek_id,
            report.new_kek_id
        );
        let reopened = SealedStore::new(Arc::clone(&backend));
        reopened.unseal(b"new").unwrap();
        assert_eq!(read(&reopened, "ns", "k"), b"value");
        reopened.put("ns", "k", b"after", None).unwrap();
        assert_eq!(read(&reopened, "ns", "k"), b"after");
    }

    #[test]
    fn record_metadata_binds_version_to_generation() {
        let base = || {
            build_sealed_metadata(&SealedRecordMeta {
                version: 0,
                aead: String::new(),
                body_nonce: [0; NONCE_LEN],
                body_tag: [0; TAG_LEN],
                body_aad: b"aad".to_vec(),
                wrapped_dek: vec![0; KEY_LEN],
                wrap_nonce: [0; NONCE_LEN],
                wrap_tag: [0; TAG_LEN],
                kek_id: "kek-1".to_string(),
                generation: Some(3),
            })
        };
        assert_eq!(
            SealedRecordMeta::parse(&base()).unwrap().generation,
            Some(3)
        );

        let mut zero = base();
        set_object_field(
            &mut zero,
            "generation",
            JsonValue::Number(JsonNumber::Integer(0)),
        );
        assert_validation_field(SealedRecordMeta::parse(&zero).err().unwrap(), "generation");

        let mut v1_with_generation = base();
        set_object_field(
            &mut v1_with_generation,
            "vault_sealed_version",
            JsonValue::Number(JsonNumber::Integer(1)),
        );
        assert_validation_field(
            SealedRecordMeta::parse(&v1_with_generation).err().unwrap(),
            "generation",
        );

        let JsonValue::Object(mut fields) = base() else {
            unreachable!()
        };
        fields.retain(|(name, _)| name != "generation");
        assert_validation_field(
            SealedRecordMeta::parse(&JsonValue::Object(fields))
                .err()
                .unwrap(),
            "generation",
        );
    }

    #[test]
    fn editing_the_generation_in_plaintext_metadata_is_caught_by_the_aad() {
        let (store, backend) = new_store();
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"one", None).unwrap();
        let old = backend.get("ns", "k").unwrap().unwrap();
        store.put("ns", "k", b"two", None).unwrap();
        // Bump the stale file's generation to look current.
        let mut metadata = old.metadata.clone();
        set_object_field(
            &mut metadata,
            "generation",
            JsonValue::Number(JsonNumber::Integer(2)),
        );
        backend
            .put(
                StoragePutInput::new(old.namespace, old.key, old.content_type, metadata, old.body)
                    .unwrap(),
            )
            .unwrap();
        assert_tamper(store.get("ns", "k"));
    }

    // ---- the freshness anchor (VLT01 F11) ----------------------------------

    /// An anchor in memory, shared across "restarts" of the store.
    #[derive(Default)]
    struct MemoryAnchor(Mutex<HashMap<String, u64>>);

    impl FreshnessAnchor for MemoryAnchor {
        fn load(&self, namespace: &str) -> Result<Option<u64>, AnchorError> {
            Ok(self.0.lock().unwrap().get(namespace).copied())
        }
        fn advance(&self, namespace: &str, epoch: u64) -> Result<(), AnchorError> {
            let mut map = self.0.lock().unwrap();
            let entry = map.entry(namespace.to_string()).or_insert(epoch);
            *entry = (*entry).max(epoch);
            Ok(())
        }
    }

    fn anchored(backend: &Arc<dyn StorageBackend>, anchor: &Arc<MemoryAnchor>) -> SealedStore {
        let store = SealedStore::with_anchor(Arc::clone(backend), anchor.clone());
        if store.status().unwrap().initialized {
            store.unseal(b"pw").unwrap();
        } else {
            store.init(b"pw", &fast_opts()).unwrap();
        }
        store
    }

    #[test]
    fn the_anchor_refuses_a_restored_pair_across_a_restart() {
        // F10's "consistent pair", which the index alone accepts after a
        // restart.
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        backend.initialize().unwrap();
        let anchor = Arc::new(MemoryAnchor::default());
        let store = anchored(&backend, &anchor);
        store.put("ns", "k", b"leaked", None).unwrap();
        let old_index = snapshot(&backend, RESERVED_NAMESPACE, &freshness_key("ns"));
        let old_record = snapshot(&backend, "ns", "k");
        store.put("ns", "k", b"rotated", None).unwrap();
        drop(store);

        restore(&backend, old_index);
        restore(&backend, old_record);
        // Without the anchor, a restarted store accepts the pair...
        assert_eq!(
            reopen(&backend)
                .get("ns", "k")
                .unwrap()
                .unwrap()
                .plaintext
                .to_vec(),
            b"leaked"
        );
        // ...and with it, it does not.
        assert_tamper(anchored(&backend, &anchor).get("ns", "k"));
    }

    #[test]
    fn the_anchor_refuses_a_deleted_index_and_a_restored_v1_file() {
        // F10's "no index" case.
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        backend.initialize().unwrap();
        let anchor = Arc::new(MemoryAnchor::default());
        let store = anchored(&backend, &anchor);
        put_v1(&store, &backend, "k", b"pre-migration");
        let v1 = snapshot(&backend, "ns", "k");
        store.put("ns", "other", b"x", None).unwrap(); // migrates
        store.put("ns", "k", b"current", None).unwrap();
        drop(store);

        backend
            .delete(RESERVED_NAMESPACE, &freshness_key("ns"), None)
            .unwrap();
        restore(&backend, v1);
        assert_eq!(
            reopen(&backend)
                .get("ns", "k")
                .unwrap()
                .unwrap()
                .plaintext
                .to_vec(),
            b"pre-migration"
        );
        assert_tamper(anchored(&backend, &anchor).get("ns", "k"));
        // A write refuses too, rather than migrating the restored file.
        assert!(matches!(
            anchored(&backend, &anchor).put("ns", "k", b"y", None),
            Err(SealedStoreError::Tamper { .. })
        ));
    }

    #[test]
    fn a_damaged_anchor_fails_closed() {
        let dir = std::env::temp_dir().join(format!(
            "vault-sealed-anchor-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        backend.initialize().unwrap();
        let anchor = Arc::new(FileFreshnessAnchor::open(&dir).unwrap());
        let store = SealedStore::with_anchor(Arc::clone(&backend), anchor);
        store.init(b"pw", &fast_opts()).unwrap();
        store.put("ns", "k", b"v", None).unwrap();
        assert_eq!(read(&store, "ns", "k"), b"v");
        // The anchor file for "ns" is hex("ns") = "6e73".
        std::fs::write(dir.join("6e73"), b"not an epoch").unwrap();
        let reopened = SealedStore::with_anchor(
            Arc::clone(&backend),
            Arc::new(FileFreshnessAnchor::open(&dir).unwrap()),
        );
        reopened.unseal(b"pw").unwrap();
        assert!(matches!(
            reopened.get("ns", "k"),
            Err(SealedStoreError::Storage(StorageError::Backend { .. }))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_first_anchored_read_protects_a_vault_written_before_anchoring() {
        // Review finding MEDIUM-1: history built without an anchor, then
        // only read under one, must still be protected after the next
        // restart.
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        backend.initialize().unwrap();
        let unanchored = SealedStore::new(Arc::clone(&backend));
        unanchored.init(b"pw", &fast_opts()).unwrap();
        unanchored.put("ns", "k", b"leaked", None).unwrap();
        let old_index = snapshot(&backend, RESERVED_NAMESPACE, &freshness_key("ns"));
        let old_record = snapshot(&backend, "ns", "k");
        unanchored.put("ns", "k", b"rotated", None).unwrap();
        drop(unanchored);

        // The upgrade: an anchored store that only ever reads.
        let anchor = Arc::new(MemoryAnchor::default());
        assert_eq!(read(&anchored(&backend, &anchor), "ns", "k"), b"rotated");

        restore(&backend, old_index);
        restore(&backend, old_record);
        assert_tamper(anchored(&backend, &anchor).get("ns", "k"));
    }
}
