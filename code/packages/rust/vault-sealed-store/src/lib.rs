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
//! It is derived from an operator password via Argon2id, verified
//! against a known-plaintext verifier stored in the manifest, and wiped
//! on `seal()` or drop.

#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};

use coding_adventures_argon2id::{argon2id, Options as Argon2Options, VERSION as ARGON2_VERSION};
use coding_adventures_chacha20_poly1305::{
    xchacha20_poly1305_aead_decrypt, xchacha20_poly1305_aead_encrypt,
};
use coding_adventures_csprng::{random_array, random_bytes};
use coding_adventures_ct_compare::ct_eq;
use coding_adventures_json_value::{JsonNumber, JsonValue};
use coding_adventures_zeroize::Zeroizing;
use storage_core::{
    Revision, StorageBackend, StorageError, StorageListOptions, StoragePutInput,
};

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

/// Sealed record schema version. Increments on any breaking on-disk change.
const SEALED_RECORD_VERSION: u64 = 1;

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

/// Result of a completed `rotate_kek` call. Useful for tests and telemetry.
#[derive(Debug, Clone)]
pub struct KekRotationReport {
    pub new_kek_id: String,
    pub records_rewrapped: usize,
    pub records_already_new: usize,
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
            Self::Tamper { namespace, key } => {
                write!(f, "vault-sealed-store tamper detected for {namespace}/{key}")
            }
            Self::Storage(e) => write!(f, "vault-sealed-store storage error: {e}"),
            Self::Crypto(m) => write!(f, "vault-sealed-store crypto error: {m}"),
            Self::Validation { field, message } => {
                write!(f, "vault-sealed-store validation failed for {field}: {message}")
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

        // 5. Produce the verifier (known-plaintext AEAD under the KEK).
        let verifier_nonce: [u8; NONCE_LEN] = random_array()
            .map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?;
        let (verifier_ct, verifier_tag) = xchacha20_poly1305_aead_encrypt(
            &VERIFIER_PLAINTEXT,
            &*kek,
            &verifier_nonce,
            b"vault-verifier",
        );

        // 6. Assemble and persist the manifest.
        let kek_id = "kek-1".to_string();
        let manifest = build_manifest_json(
            MANIFEST_VERSION,
            opts.argon2id_time_cost,
            opts.argon2id_memory_kib,
            opts.argon2id_parallelism,
            &[KekEntry {
                id: kek_id.clone(),
                status: "active",
                salt: salt.clone(),
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

        // 7. Install the KEK in memory. We move the `Zeroizing<[u8;32]>`
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
            // Derive under *this* entry's salt.
            let candidate = derive_kek(
                password,
                &entry.salt,
                manifest.time_cost,
                manifest.memory_kib,
                manifest.parallelism,
            )?;
            let decrypted = xchacha20_poly1305_aead_decrypt(
                &entry.verifier_ct,
                &*candidate,
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

    // ---- data plane -------------------------------------------------------

    /// Encrypt and write one record. The reserved namespace is rejected.
    pub fn put(
        &self,
        namespace: &str,
        key: &str,
        plaintext: &[u8],
        if_revision: Option<Revision>,
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

        let guard = self.state.lock().expect("vault state mutex poisoned");
        let unsealed = guard.unsealed.as_ref().ok_or(SealedStoreError::Sealed)?;

        // Fresh per-record DEK from CSPRNG. Wrapped in Zeroizing *at
        // creation* so any `?` below wipes on the way out.
        let dek: Zeroizing<[u8; KEY_LEN]> = Zeroizing::new(
            random_array().map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?,
        );

        let body_nonce: [u8; NONCE_LEN] = random_array()
            .map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?;
        let aad = record_aad(namespace, key);
        let (ciphertext, body_tag) =
            xchacha20_poly1305_aead_encrypt(plaintext, &*dek, &body_nonce, &aad);

        // Wrap the DEK under the KEK.
        let wrap_nonce: [u8; NONCE_LEN] = random_array()
            .map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?;
        let wrap_aad = wrap_aad(namespace, key, &unsealed.id);
        let (wrapped_dek, wrap_tag) =
            xchacha20_poly1305_aead_encrypt(&*dek, &*unsealed.key, &wrap_nonce, &wrap_aad);

        let metadata = build_sealed_metadata(
            &body_nonce,
            &body_tag,
            &aad,
            &wrapped_dek,
            &wrap_nonce,
            &wrap_tag,
            &unsealed.id,
        );

        let mut put_in = StoragePutInput::new(
            namespace.to_string(),
            key.to_string(),
            SEALED_CONTENT_TYPE.to_string(),
            metadata,
            ciphertext,
        )
        .map_err(SealedStoreError::Storage)?;
        put_in = put_in.with_if_revision(if_revision);

        let rec = self.backend.put(put_in)?;
        // `dek` drops here, wiping the cleartext DEK bytes.
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

        // Guard: the AAD-bound namespace/key must match where we found it.
        let expected_aad = record_aad(namespace, key);
        if sealed.body_aad != expected_aad {
            return Err(SealedStoreError::Tamper {
                namespace: namespace.to_string(),
                key: key.to_string(),
            });
        }

        // We only unwrap records wrapped under the in-memory KEK. Records
        // wrapped under an earlier KEK (e.g. during an in-progress rotation)
        // are surfaced as Tamper so the caller knows to resume rotation or
        // unseal under the older password.
        if sealed.kek_id != unsealed.id {
            return Err(SealedStoreError::Tamper {
                namespace: namespace.to_string(),
                key: key.to_string(),
            });
        }

        let wrap_aad = wrap_aad(namespace, key, &unsealed.id);
        let dek = unwrap_dek(
            &sealed.wrapped_dek,
            &*unsealed.key,
            &sealed.wrap_nonce,
            &wrap_aad,
            &sealed.wrap_tag,
            namespace,
            key,
        )?;

        let plaintext = xchacha20_poly1305_aead_decrypt(
            &record.body,
            &*dek,
            &sealed.body_nonce,
            &sealed.body_aad,
            &sealed.body_tag,
        )
        .ok_or_else(|| SealedStoreError::Tamper {
            namespace: namespace.to_string(),
            key: key.to_string(),
        })?;

        // `dek` drops here, wiping the cleartext DEK bytes.
        Ok(Some(SealedRecord {
            namespace: record.namespace,
            key: record.key,
            revision: record.revision,
            created_at_ms: record.created_at,
            updated_at_ms: record.updated_at,
            plaintext: Zeroizing::new(plaintext),
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
        // Deletion does not require a decrypt but does require unseal —
        // otherwise a sealed vault could be used as a "destroy records"
        // oracle without proving knowledge of the password.
        {
            let guard = self.state.lock().expect("vault state mutex poisoned");
            if guard.unsealed.is_none() {
                return Err(SealedStoreError::Sealed);
            }
        }
        self.backend
            .delete(namespace, key, if_revision.as_ref())?;
        Ok(())
    }

    /// List sealed records by prefix within a namespace. Returns
    /// ciphertext-side metadata only; the AEAD is not run.
    pub fn list(
        &self,
        namespace: &str,
        options: StorageListOptions,
    ) -> Result<Vec<SealedStat>, SealedStoreError> {
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
        Ok(out)
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
        let verifier_nonce: [u8; NONCE_LEN] = random_array()
            .map_err(|_| SealedStoreError::Crypto("csprng failure".into()))?;
        let (verifier_ct, verifier_tag) = xchacha20_poly1305_aead_encrypt(
            &VERIFIER_PLAINTEXT,
            &*new_kek,
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
            salt: new_salt,
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
                        &*old_kek,
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
                        &*new_kek,
                        &new_wrap_nonce,
                        &new_wrap_aad,
                    );
                    // `dek` drops at end of this loop iteration.

                    let new_meta = build_sealed_metadata(
                        &meta.body_nonce,
                        &meta.body_tag,
                        &meta.body_aad,
                        &new_wrapped_dek,
                        &new_wrap_nonce,
                        &new_wrap_tag,
                        &new_kek_id,
                    );
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
    let dek_vec = xchacha20_poly1305_aead_decrypt(
        wrapped_dek,
        kek,
        wrap_nonce,
        wrap_aad,
        wrap_tag,
    )
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
// We use `JsonValue` directly instead of serde, because the crate tree does
// not depend on serde — and storage-core's metadata field is typed as
// JsonValue anyway.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct KekEntry {
    id: String,
    status: &'static str, // "active" | "retired"
    /// Per-KEK Argon2id salt. Each entry is independently verifiable so an
    /// operator can always unseal under the password that minted *this*
    /// KEK, even after many rotations.
    salt: Vec<u8>,
    verifier_nonce: [u8; NONCE_LEN],
    verifier_tag: [u8; TAG_LEN],
    verifier_ct: Vec<u8>,
}

#[derive(Debug, Clone)]
struct Manifest {
    time_cost: u32,
    memory_kib: u32,
    parallelism: u32,
    keks: Vec<KekEntry>,
}

struct SealedRecordMeta {
    body_nonce: [u8; NONCE_LEN],
    body_tag: [u8; TAG_LEN],
    body_aad: Vec<u8>,
    wrapped_dek: Vec<u8>,
    wrap_nonce: [u8; NONCE_LEN],
    wrap_tag: [u8; TAG_LEN],
    kek_id: String,
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
            JsonValue::Object(vec![
                ("id".to_string(), JsonValue::String(e.id.clone())),
                ("status".to_string(), JsonValue::String(e.status.to_string())),
                ("salt".to_string(), JsonValue::String(hex_encode(&e.salt))),
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
            ])
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

fn build_sealed_metadata(
    body_nonce: &[u8; NONCE_LEN],
    body_tag: &[u8; TAG_LEN],
    body_aad: &[u8],
    wrapped_dek: &[u8],
    wrap_nonce: &[u8; NONCE_LEN],
    wrap_tag: &[u8; TAG_LEN],
    kek_id: &str,
) -> JsonValue {
    JsonValue::Object(vec![
        (
            "vault_sealed_version".to_string(),
            JsonValue::Number(JsonNumber::Integer(SEALED_RECORD_VERSION as i64)),
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
    ])
}

fn build_namespaces_json(names: &[String]) -> JsonValue {
    JsonValue::Object(vec![
        (
            "vault_namespaces_version".to_string(),
            JsonValue::Number(JsonNumber::Integer(1)),
        ),
        (
            "names".to_string(),
            JsonValue::Array(
                names
                    .iter()
                    .map(|s| JsonValue::String(s.clone()))
                    .collect(),
            ),
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
            let verifier_nonce = hex_decode_fixed::<NONCE_LEN>(
                get_string(eo, "verifier_nonce")?,
                "verifier_nonce",
            )?;
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
        let body_nonce =
            hex_decode_fixed::<NONCE_LEN>(get_string(obj, "body_nonce")?, "body_nonce")?;
        let body_tag = hex_decode_fixed::<TAG_LEN>(get_string(obj, "body_tag")?, "body_tag")?;
        let body_aad = hex_decode(get_string(obj, "body_aad")?).map_err(|_| {
            SealedStoreError::Validation {
                field: "body_aad".to_string(),
                message: "invalid hex".to_string(),
            }
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
            body_nonce,
            body_tag,
            body_aad,
            wrapped_dek,
            wrap_nonce,
            wrap_tag,
            kek_id,
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
    for chunk in bytes.chunks_exact(2) {
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
        assert_eq!(hex_decode("00ffDEAD").unwrap(), vec![0x00, 0xff, 0xde, 0xad]);
        assert!(hex_decode("abc").is_err());
        assert!(hex_decode("xz").is_err());
    }

    #[test]
    fn kek_id_increments_and_rejects_overflow() {
        let e = [
            KekEntry {
                id: "kek-1".into(),
                status: "retired",
                salt: vec![0; 16],
                verifier_nonce: [0; NONCE_LEN],
                verifier_tag: [0; TAG_LEN],
                verifier_ct: vec![],
            },
            KekEntry {
                id: "kek-7".into(),
                status: "active",
                salt: vec![0; 16],
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
            salt: vec![0; 16],
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
        let mf = backend.get(RESERVED_NAMESPACE, MANIFEST_KEY).unwrap().unwrap();
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
        let tampered = build_namespaces_json(&[
            "real".to_string(),
            RESERVED_NAMESPACE.to_string(),
        ]);
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
        let mf = backend.get(RESERVED_NAMESPACE, MANIFEST_KEY).unwrap().unwrap();
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
}
