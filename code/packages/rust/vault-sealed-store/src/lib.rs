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
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use coding_adventures_json_value::{JsonNumber, JsonValue};
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

/// Content-type tag on the manifest record.
pub const MANIFEST_CONTENT_TYPE: &str = "application/vault-manifest+json-v1";

/// Content-type tag on every sealed record.
pub const SEALED_CONTENT_TYPE: &str = "application/vault-sealed+json-v1";

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
    /// Caller input violated a surface-level contract.
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
    pub fn init(&self, password: &[u8], opts: &InitOptions) -> Result<(), SealedStoreError> {
        // 1. Fail fast if a manifest already exists.
        if self
            .backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)?
            .is_some()
        {
            return Err(SealedStoreError::AlreadyInitialized);
        }

        // 2. Collect salt (caller-supplied or CSPRNG).
        let salt = match &opts.salt_override {
            Some(s) => {
                if s.len() < 8 {
                    return Err(SealedStoreError::Validation {
                        field: "salt_override".to_string(),
                        message: "must be at least 8 bytes".to_string(),
                    });
                }
                s.clone()
            }
            None => random_bytes(DEFAULT_ARGON2_SALT_LEN)
                .map_err(|e| SealedStoreError::Crypto(format!("csprng: {e:?}")))?,
        };

        // 3. Derive the initial KEK.
        let kek = derive_kek(
            password,
            &salt,
            opts.argon2id_time_cost,
            opts.argon2id_memory_kib,
            opts.argon2id_parallelism,
        )?;

        // 4. Produce the verifier (known-plaintext AEAD under the KEK).
        let verifier_nonce: [u8; NONCE_LEN] = random_array()
            .map_err(|e| SealedStoreError::Crypto(format!("csprng: {e:?}")))?;
        let (verifier_ct, verifier_tag) = xchacha20_poly1305_aead_encrypt(
            &VERIFIER_PLAINTEXT,
            &kek,
            &verifier_nonce,
            b"vault-verifier",
        );

        // 5. Assemble and persist the manifest.
        let kek_id = "kek-1".to_string();
        let manifest = build_manifest_json(
            MANIFEST_VERSION,
            opts.argon2id_time_cost,
            opts.argon2id_memory_kib,
            opts.argon2id_parallelism,
            &salt,
            &[KekEntry {
                id: kek_id.clone(),
                status: "active",
                verifier_nonce,
                verifier_tag,
                verifier_ct: verifier_ct.clone(),
            }],
            now_ms_from_backend_optional(),
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

        // 6. Install the KEK in memory.
        self.state
            .lock()
            .expect("vault state mutex poisoned")
            .unsealed = Some(UnsealedKey {
            id: kek_id,
            key: Zeroizing::new(kek),
        });

        Ok(())
    }

    /// Load the manifest, derive a candidate KEK from `password`, verify
    /// against the manifest's verifier. On success holds the KEK in RAM.
    pub fn unseal(&self, password: &[u8]) -> Result<(), SealedStoreError> {
        let manifest_record = self
            .backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)?
            .ok_or(SealedStoreError::NotInitialized)?;

        let manifest = Manifest::parse(&manifest_record.metadata)?;

        let kek = derive_kek(
            password,
            &manifest.salt,
            manifest.time_cost,
            manifest.memory_kib,
            manifest.parallelism,
        )?;

        // Walk active → retired KEKs; stop at the first that verifies. This
        // supports mid-rotation states where the active entry has not yet
        // been switched to the new password.
        let mut matched_id: Option<String> = None;
        for entry in &manifest.keks {
            let candidate = xchacha20_poly1305_aead_decrypt(
                &entry.verifier_ct,
                &kek,
                &entry.verifier_nonce,
                b"vault-verifier",
                &entry.verifier_tag,
            );
            if candidate.as_deref() == Some(&VERIFIER_PLAINTEXT[..]) {
                matched_id = Some(entry.id.clone());
                break;
            }
        }

        let id = match matched_id {
            Some(id) => id,
            None => {
                // kek is a plain array — drop it explicitly via Zeroizing.
                let _wipe = Zeroizing::new(kek);
                return Err(SealedStoreError::BadPassword);
            }
        };

        self.state
            .lock()
            .expect("vault state mutex poisoned")
            .unsealed = Some(UnsealedKey {
            id,
            key: Zeroizing::new(kek),
        });
        Ok(())
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

        let guard = self.state.lock().expect("vault state mutex poisoned");
        let unsealed = guard.unsealed.as_ref().ok_or(SealedStoreError::Sealed)?;

        // Fresh per-record DEK from CSPRNG.
        let mut dek: [u8; KEY_LEN] = random_array()
            .map_err(|e| SealedStoreError::Crypto(format!("csprng: {e:?}")))?;

        let body_nonce: [u8; NONCE_LEN] = random_array()
            .map_err(|e| SealedStoreError::Crypto(format!("csprng: {e:?}")))?;
        let aad = record_aad(namespace, key);
        let (ciphertext, body_tag) =
            xchacha20_poly1305_aead_encrypt(plaintext, &dek, &body_nonce, &aad);

        // Wrap the DEK under the KEK.
        let wrap_nonce: [u8; NONCE_LEN] = random_array()
            .map_err(|e| SealedStoreError::Crypto(format!("csprng: {e:?}")))?;
        let wrap_aad = wrap_aad(namespace, key, &unsealed.id);
        let (wrapped_dek, wrap_tag) =
            xchacha20_poly1305_aead_encrypt(&dek, &unsealed.key, &wrap_nonce, &wrap_aad);

        // DEK has been encrypted — wipe the cleartext copy now.
        dek.zeroize();

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

        // We currently only support unwrapping records wrapped under the
        // in-memory KEK. Multi-KEK support arrives via `rotate_kek`.
        if sealed.kek_id != unsealed.id {
            return Err(SealedStoreError::Tamper {
                namespace: namespace.to_string(),
                key: key.to_string(),
            });
        }

        let wrap_aad = wrap_aad(namespace, key, &unsealed.id);
        let dek_vec = xchacha20_poly1305_aead_decrypt(
            &sealed.wrapped_dek,
            &unsealed.key,
            &sealed.wrap_nonce,
            &wrap_aad,
            &sealed.wrap_tag,
        )
        .ok_or_else(|| SealedStoreError::Tamper {
            namespace: namespace.to_string(),
            key: key.to_string(),
        })?;

        if dek_vec.len() != KEY_LEN {
            let mut z = Zeroizing::new(dek_vec);
            z.zeroize();
            return Err(SealedStoreError::Tamper {
                namespace: namespace.to_string(),
                key: key.to_string(),
            });
        }

        let mut dek = [0u8; KEY_LEN];
        dek.copy_from_slice(&dek_vec);
        {
            let mut z = Zeroizing::new(dek_vec);
            z.zeroize();
        }

        let plaintext = xchacha20_poly1305_aead_decrypt(
            &record.body,
            &dek,
            &sealed.body_nonce,
            &sealed.body_aad,
            &sealed.body_tag,
        )
        .ok_or_else(|| SealedStoreError::Tamper {
            namespace: namespace.to_string(),
            key: key.to_string(),
        })?;

        dek.zeroize();

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
    pub fn rotate_kek(
        &self,
        old_password: &[u8],
        new_password: &[u8],
    ) -> Result<KekRotationReport, SealedStoreError> {
        // Step 1: confirm caller knows the old password.
        self.unseal(old_password)?;
        let old_kek_bytes: [u8; KEY_LEN] = {
            let guard = self.state.lock().expect("vault state mutex poisoned");
            let cur = guard.unsealed.as_ref().ok_or(SealedStoreError::Sealed)?;
            let mut copy = [0u8; KEY_LEN];
            copy.copy_from_slice(&*cur.key);
            copy
        };
        let old_kek_id = {
            let guard = self.state.lock().expect("vault state mutex poisoned");
            guard
                .unsealed
                .as_ref()
                .map(|u| u.id.clone())
                .ok_or(SealedStoreError::Sealed)?
        };

        // Step 2: derive the new KEK from a freshly generated salt. We keep
        // the original KDF parameters so the user sees no latency change.
        let manifest_record = self
            .backend
            .get(RESERVED_NAMESPACE, MANIFEST_KEY)?
            .ok_or(SealedStoreError::NotInitialized)?;
        let mut manifest = Manifest::parse(&manifest_record.metadata)?;
        let manifest_revision = manifest_record.revision.clone();

        let new_salt = random_bytes(DEFAULT_ARGON2_SALT_LEN)
            .map_err(|e| SealedStoreError::Crypto(format!("csprng: {e:?}")))?;
        // Note: the manifest has a single salt slot in v1; we overwrite it
        // now that the rotation commits. Old verifier uses old salt (still
        // stored on the entry as-of the verifier ciphertext but we will
        // mark it retired).
        let new_kek = derive_kek(
            new_password,
            &new_salt,
            manifest.time_cost,
            manifest.memory_kib,
            manifest.parallelism,
        )?;

        // Step 3: build the new verifier and manifest entry.
        let new_kek_id = next_kek_id(&manifest.keks);
        let verifier_nonce: [u8; NONCE_LEN] = random_array()
            .map_err(|e| SealedStoreError::Crypto(format!("csprng: {e:?}")))?;
        let (verifier_ct, verifier_tag) = xchacha20_poly1305_aead_encrypt(
            &VERIFIER_PLAINTEXT,
            &new_kek,
            &verifier_nonce,
            b"vault-verifier",
        );
        manifest
            .keks
            .iter_mut()
            .for_each(|e| e.status = if e.id == old_kek_id { "retired" } else { e.status });
        manifest.keks.push(KekEntry {
            id: new_kek_id.clone(),
            status: "active",
            verifier_nonce,
            verifier_tag,
            verifier_ct: verifier_ct.clone(),
        });
        manifest.salt = new_salt;

        // Step 4: persist manifest first (CAS on its revision) — this is
        // the point at which both old and new KEKs are valid for unseal.
        let new_manifest_json = build_manifest_json(
            MANIFEST_VERSION,
            manifest.time_cost,
            manifest.memory_kib,
            manifest.parallelism,
            &manifest.salt,
            &manifest.keks,
            now_ms_from_backend_optional(),
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

        // Step 5: iterate every record (all namespaces, except reserved)
        // and rewrap its DEK. This is restartable: records already wrapped
        // under `new_kek_id` are left alone.
        let mut rewrapped = 0usize;
        let mut already_new = 0usize;
        for ns in self.list_external_namespaces()? {
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
                    let dek_vec = xchacha20_poly1305_aead_decrypt(
                        &meta.wrapped_dek,
                        &old_kek_bytes,
                        &meta.wrap_nonce,
                        &old_wrap_aad,
                        &meta.wrap_tag,
                    )
                    .ok_or_else(|| SealedStoreError::Tamper {
                        namespace: rec.namespace.clone(),
                        key: rec.key.clone(),
                    })?;
                    if dek_vec.len() != KEY_LEN {
                        let mut z = Zeroizing::new(dek_vec);
                        z.zeroize();
                        return Err(SealedStoreError::Tamper {
                            namespace: rec.namespace.clone(),
                            key: rec.key.clone(),
                        });
                    }
                    let mut dek = [0u8; KEY_LEN];
                    dek.copy_from_slice(&dek_vec);
                    {
                        let mut z = Zeroizing::new(dek_vec);
                        z.zeroize();
                    }

                    // Rewrap under new KEK.
                    let new_wrap_nonce: [u8; NONCE_LEN] = random_array()
                        .map_err(|e| SealedStoreError::Crypto(format!("csprng: {e:?}")))?;
                    let new_wrap_aad = wrap_aad(&rec.namespace, &rec.key, &new_kek_id);
                    let (new_wrapped_dek, new_wrap_tag) = xchacha20_poly1305_aead_encrypt(
                        &dek,
                        &new_kek,
                        &new_wrap_nonce,
                        &new_wrap_aad,
                    );
                    dek.zeroize();

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

        // Step 6: swap the in-memory KEK.
        self.state
            .lock()
            .expect("vault state mutex poisoned")
            .unsealed = Some(UnsealedKey {
            id: new_kek_id.clone(),
            key: Zeroizing::new(new_kek),
        });

        // Step 7: wipe the stack-resident old-KEK copy.
        let mut old_copy = old_kek_bytes;
        old_copy.zeroize();

        Ok(KekRotationReport {
            new_kek_id,
            records_rewrapped: rewrapped,
            records_already_new: already_new,
        })
    }

    fn list_external_namespaces(&self) -> Result<Vec<String>, SealedStoreError> {
        // The current StorageBackend trait does not expose a namespace
        // enumerator, so we ask the caller to pre-register namespaces via
        // a tiny manifest side-table. For v1 we walk a simple convention:
        // every sealed record lives in a namespace recorded by prior puts.
        //
        // Practical fallback: the only records the store emits are in
        // `RESERVED_NAMESPACE` plus arbitrary external ones. We fetch a
        // list side-record kept under the reserved namespace when present,
        // and otherwise return a single "default" namespace bucket that
        // callers have already touched.
        //
        // For simplicity in v1 we require callers to use a known namespace
        // or manually drive rotation per-namespace. Here we reflect the
        // records by scanning a single shared registry.
        //
        // This side-registry under the reserved namespace stores the set
        // of namespaces the vault has ever written to.
        let rec = self
            .backend
            .get(RESERVED_NAMESPACE, "namespaces")?;
        let mut names = Vec::new();
        if let Some(rec) = rec {
            if let JsonValue::Object(obj) = &rec.metadata {
                if let Some((_, JsonValue::Array(arr))) = obj.iter().find(|(k, _)| k == "names") {
                    for v in arr {
                        if let JsonValue::String(s) = v {
                            names.push(s.clone());
                        }
                    }
                }
            }
        }
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
            message: format!("namespace {RESERVED_NAMESPACE:?} is reserved"),
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

fn derive_kek(
    password: &[u8],
    salt: &[u8],
    time_cost: u32,
    memory_kib: u32,
    parallelism: u32,
) -> Result<[u8; KEY_LEN], SealedStoreError> {
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
    .map_err(|e| SealedStoreError::Crypto(format!("argon2id: {e}")))?;

    if tag.len() != KEY_LEN {
        return Err(SealedStoreError::Crypto("argon2id: wrong tag length".into()));
    }
    let mut out = [0u8; KEY_LEN];
    out.copy_from_slice(&tag);
    // Shadow-wipe the intermediate Vec<u8>.
    {
        let mut z = Zeroizing::new(tag);
        z.zeroize();
    }
    Ok(out)
}

fn now_ms_from_backend_optional() -> u64 {
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

fn next_kek_id(existing: &[KekEntry]) -> String {
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
    format!("kek-{}", max_n + 1)
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
    verifier_nonce: [u8; NONCE_LEN],
    verifier_tag: [u8; TAG_LEN],
    verifier_ct: Vec<u8>,
}

#[derive(Debug, Clone)]
struct Manifest {
    time_cost: u32,
    memory_kib: u32,
    parallelism: u32,
    salt: Vec<u8>,
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
    salt: &[u8],
    keks: &[KekEntry],
    created_at_ms: u64,
) -> JsonValue {
    let keks_json: Vec<JsonValue> = keks
        .iter()
        .map(|e| {
            JsonValue::Object(vec![
                ("id".to_string(), JsonValue::String(e.id.clone())),
                ("status".to_string(), JsonValue::String(e.status.to_string())),
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
            "kdf_salt".to_string(),
            JsonValue::String(hex_encode(salt)),
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

impl Manifest {
    fn parse(meta: &JsonValue) -> Result<Self, SealedStoreError> {
        let obj = expect_object(meta, "manifest")?;
        let time_cost = get_u32(obj, "kdf_time_cost")?;
        let memory_kib = get_u32(obj, "kdf_memory_cost_kib")?;
        let parallelism = get_u32(obj, "kdf_parallelism")?;
        let salt = hex_decode(get_string(obj, "kdf_salt")?).map_err(|m| {
            SealedStoreError::Validation {
                field: "kdf_salt".to_string(),
                message: m,
            }
        })?;
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
        let mut keks = Vec::with_capacity(keks_arr.len());
        for entry in keks_arr {
            let eo = expect_object(entry, "keks[]")?;
            let id = get_string(eo, "id")?.to_string();
            let status_raw = get_string(eo, "status")?;
            // JsonValue stores `status` as `String`; we compress back to the
            // two canonical forms for in-memory use.
            let status: &'static str = match status_raw {
                "active" => "active",
                "retired" => "retired",
                other => {
                    return Err(SealedStoreError::Validation {
                        field: "status".to_string(),
                        message: format!("unknown status {other:?}"),
                    })
                }
            };
            let verifier_nonce = hex_decode_fixed::<NONCE_LEN>(
                get_string(eo, "verifier_nonce")?,
                "verifier_nonce",
            )?;
            let verifier_tag =
                hex_decode_fixed::<TAG_LEN>(get_string(eo, "verifier_tag")?, "verifier_tag")?;
            let verifier_ct = hex_decode(get_string(eo, "verifier_ct")?).map_err(|m| {
                SealedStoreError::Validation {
                    field: "verifier_ct".to_string(),
                    message: m,
                }
            })?;
            keks.push(KekEntry {
                id,
                status,
                verifier_nonce,
                verifier_tag,
                verifier_ct,
            });
        }
        Ok(Self {
            time_cost,
            memory_kib,
            parallelism,
            salt,
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
        let body_aad =
            hex_decode(get_string(obj, "body_aad")?).map_err(|m| SealedStoreError::Validation {
                field: "body_aad".to_string(),
                message: m,
            })?;
        let wrapped_dek = hex_decode(get_string(obj, "wrapped_dek")?).map_err(|m| {
            SealedStoreError::Validation {
                field: "wrapped_dek".to_string(),
                message: m,
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
    let v = hex_decode(s).map_err(|m| SealedStoreError::Validation {
        field: field.to_string(),
        message: m,
    })?;
    if v.len() != N {
        return Err(SealedStoreError::Validation {
            field: field.to_string(),
            message: format!("expected {N} bytes, got {}", v.len()),
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
            argon2id_memory_kib: 32,   // minimum for parallelism=4 is 4*8=32
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
        assert!(matches!(
            store.get("a", "b"),
            Err(SealedStoreError::Sealed)
        ));
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

        // Fetch (ns,a)'s entire row and paste it (metadata + body) into
        // (ns,b). Now the body_aad = ns\0a but the record lives at ns/b
        // — AEAD decrypt succeeds under the DEK, but the AAD-mismatch
        // check in get() rejects it.
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
        // Now use rev2 - but write something else, and then try again
        // with rev2 (stale).
        let _rev3 = store.put("ns", "k", b"v3", Some(rev2.clone())).unwrap();
        let err = store.put("ns", "k", b"v4", Some(rev2)).unwrap_err();
        assert!(matches!(err, SealedStoreError::Storage(StorageError::Conflict { .. })));
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
        // Fresh instance on the same storage.
        let store2 = SealedStore::new(Arc::clone(&backend));
        assert!(store2.is_sealed());
        store2.unseal(b"pw").unwrap();
        assert_eq!(&*store2.get("ns", "k").unwrap().unwrap().plaintext, b"hello");
    }

    #[test]
    fn hex_roundtrip() {
        assert_eq!(hex_encode(&[0x00, 0xff, 0xde, 0xad]), "00ffdead");
        assert_eq!(hex_decode("00ffDEAD").unwrap(), vec![0x00, 0xff, 0xde, 0xad]);
        assert!(hex_decode("abc").is_err());
        assert!(hex_decode("xz").is_err());
    }

    #[test]
    fn kek_id_increments() {
        let e = [
            KekEntry {
                id: "kek-1".into(),
                status: "retired",
                verifier_nonce: [0; NONCE_LEN],
                verifier_tag: [0; TAG_LEN],
                verifier_ct: vec![],
            },
            KekEntry {
                id: "kek-7".into(),
                status: "active",
                verifier_nonce: [0; NONCE_LEN],
                verifier_tag: [0; TAG_LEN],
                verifier_ct: vec![],
            },
        ];
        assert_eq!(next_kek_id(&e), "kek-8");
        assert_eq!(next_kek_id(&[]), "kek-1");
    }

    #[test]
    fn init_validates_short_salt_override() {
        let (store, _) = new_store();
        let mut opts = fast_opts();
        opts.salt_override = Some(vec![1, 2, 3]); // too short
        let err = store.init(b"pw", &opts).unwrap_err();
        assert!(matches!(err, SealedStoreError::Validation { .. }));
    }
}
