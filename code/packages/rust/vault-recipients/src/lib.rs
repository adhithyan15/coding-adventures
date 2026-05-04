//! # coding_adventures_vault_recipients — VLT04
//!
//! ## What this crate does
//!
//! VLT01 wraps each per-record DEK under a single KEK. That single-
//! recipient model breaks the moment you want any of:
//!
//! * **Sharing**: Alice creates an item; Bob reads it. The server
//!   is zero-knowledge, so Alice must wrap the DEK under a key Bob
//!   owns — Bob's X25519 public key.
//! * **Multi-device**: laptop, phone, browser extension — each has
//!   its own device key; each unwraps the vault key independently.
//!   Adding a phone is one wrap operation, not a re-encrypt.
//! * **Recovery**: a printed-at-signup recovery key is just another
//!   recipient on every wrap. Lose your password? Type the recovery
//!   key.
//! * **GitOps secrets** (SOPS pattern): same DEK wrapped to a list
//!   of KMS / age / GPG recipients.
//! * **Sealed Secrets** (Bitnami pattern): cluster controller's RSA
//!   pubkey is a recipient.
//!
//! VLT04 is the layer that generalises "wrap this file key" to
//! "wrap this file key for *each of* these recipients." Same wire
//! shape as age:
//!
//! ```text
//!   record =
//!     file_key (random 32 bytes, used to AEAD the body)
//!     wrap_set: [(recipient_id, wrapped_for_recipient), …]
//! ```
//!
//! Adding a grantee = one more wrap operation appended to wrap_set.
//! Re-keying = re-wrap the file_key under a fresh ephemeral; old
//! wraps for the same record stay valid until rotated.
//!
//! ## What's in this crate (v0.1)
//!
//! - `Recipient` trait: every recipient has an opaque `recipient_id`
//!   and methods `wrap(file_key) -> wrapped_bytes` and
//!   `try_unwrap(wrapped, identity) -> Option<file_key>` (None =
//!   "not for me," `Err` = "for me but tamper / wrong identity").
//! - `PassphraseRecipient` — Argon2id-derived KEK, AEAD wrap. Used
//!   for "encrypt to a passphrase" the way age does with
//!   `--passphrase`.
//! - `X25519Recipient` — age's standard recipient: pick a fresh
//!   ephemeral X25519 keypair per wrap, do ECDH with the
//!   recipient's pubkey, HKDF-derive a wrap key, AEAD-encrypt the
//!   file key. Sender holds nothing afterwards (the ephemeral
//!   private key is dropped); recipient unwraps with their own
//!   X25519 private key.
//!
//! ## Wire format
//!
//! Each wrap output is opaque to upper layers, but for record we
//! document the layouts:
//!
//! ```text
//!   PassphraseRecipient wrap blob:
//!     magic(2) || salt(16) || nonce(24) || ct(32) || tag(16)
//!     magic = b"PR"  ("Passphrase Recipient v1")
//!     AAD   = magic                                           ; 90 bytes total
//!
//!   X25519Recipient wrap blob:
//!     magic(2) || ephemeral_pubkey(32) || nonce(24) || ct(32) || tag(16)
//!     magic = b"X1"  ("X25519 Recipient v1")
//!     AAD   = magic || ephemeral_pubkey || recipient_pubkey   ; 106 bytes total
//! ```
//!
//! The X25519 AAD binds the wrapped key to *both* the ephemeral
//! and the recipient pubkey, so a malicious sender cannot rebind a
//! captured ephemeral against a different recipient.
//!
//! ## What this crate does *not* do
//!
//! - **No identity / KDF for recipient lookup.** A `recipient_id`
//!   is just a `Vec<u8>` produced by the recipient itself; how the
//!   record persistence layer dispatches "who am I in this list" is
//!   one layer up.
//! - **No RSA-OAEP recipient.** Deferred to a follow-up PR (it's
//!   how Bitwarden / 1Password share between users today; Sealed
//!   Secrets uses RSA in-cluster).
//! - **No KMS recipients (AWS / GCP / Azure).** Same future work.
//! - **No revocation lists.** Adding a recipient is appending a
//!   wrap; revoking is re-keying the file_key (out of scope here).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_argon2id::{argon2id, Options as ArgonOptions};
use coding_adventures_chacha20_poly1305::{
    xchacha20_poly1305_aead_decrypt, xchacha20_poly1305_aead_encrypt,
};
use coding_adventures_csprng::fill_random;
use coding_adventures_hkdf::{hkdf, HashAlgorithm};
use coding_adventures_x25519::{x25519, x25519_base};
use coding_adventures_zeroize::{Zeroize, Zeroizing};

// ─────────────────────────────────────────────────────────────────────
// 1. Constants & types
// ─────────────────────────────────────────────────────────────────────

/// Length of the file_key (and every other 256-bit symmetric key) in bytes.
pub const KEY_LEN: usize = 32;
/// Length of an XChaCha20-Poly1305 nonce, in bytes.
pub const NONCE_LEN: usize = 24;
/// Length of a Poly1305 tag, in bytes.
pub const TAG_LEN: usize = 16;
/// Length of an X25519 public key (and private scalar), in bytes.
pub const X25519_KEY_LEN: usize = 32;
/// Argon2id salt length, in bytes.
pub const SALT_LEN: usize = 16;
/// Default Argon2id time cost (RFC 9106 §4 "second recommended" baseline).
pub const DEFAULT_ARGON_TIME_COST: u32 = 3;
/// Default Argon2id memory cost (KiB) — 64 MiB.
pub const DEFAULT_ARGON_MEMORY_KIB: u32 = 64 * 1024;
/// Default Argon2id parallelism.
pub const DEFAULT_ARGON_PARALLELISM: u32 = 4;

/// 32-byte symmetric key (e.g. file_key) — held in `Zeroizing`.
pub type Key = Zeroizing<[u8; KEY_LEN]>;

/// Generate a fresh random file key. The vault uses this for the
/// per-record DEK that gets wrapped to one or more recipients.
pub fn fresh_file_key() -> Result<Key, RecipientError> {
    let mut k = Zeroizing::new([0u8; KEY_LEN]);
    fill_random(&mut k[..])?;
    Ok(k)
}

// ─────────────────────────────────────────────────────────────────────
// 2. The `Recipient` and `Identity` traits
// ─────────────────────────────────────────────────────────────────────

/// A "recipient" — an entity that can wrap a file key for itself.
///
/// In age terms: a recipient owns a public key (X25519) or a
/// passphrase, and their `wrap(file_key)` produces the per-recipient
/// blob that goes into the record's wrap set.
pub trait Recipient {
    /// Stable string identifying the recipient kind, e.g.
    /// `"passphrase"`, `"x25519"`, `"kms-aws"`. Telemetry, not
    /// security.
    fn kind(&self) -> &'static str;

    /// Opaque per-recipient identifier. The persistence layer uses
    /// this to label the wrap so the right `Identity` can find
    /// "their" wrap on unwrap. The semantics of these bytes are
    /// recipient-kind-specific (e.g. for X25519 it's the public
    /// key; for passphrase it's a salted hash so the same
    /// passphrase recipient identifies its own wrap; for KMS it's
    /// the CMK ARN).
    fn recipient_id(&self) -> Vec<u8>;

    /// Wrap a file key for this recipient. Returns opaque bytes the
    /// matching `Identity` can later unwrap.
    fn wrap(&self, file_key: &Key) -> Result<WrappedKey, RecipientError>;
}

/// An "identity" — the unwrap side of a `Recipient`. In age terms:
/// for an X25519 public key, the identity is the corresponding
/// X25519 secret key; for a passphrase recipient, the identity is
/// the passphrase. Each identity may attempt to unwrap any wrap
/// from the record's wrap set — the `Ok(None)` return signals
/// "this wrap isn't for me," `Ok(Some(_))` signals success, and
/// `Err(...)` signals "this wrap *is* for me but it's broken."
pub trait Identity {
    /// Same string as the matching `Recipient::kind`. The set of
    /// `kind`s an identity can unwrap is implementation-specific
    /// (typically just one).
    fn kind(&self) -> &'static str;

    /// Opaque id matching the corresponding `Recipient::recipient_id`.
    /// Used by [`try_unwrap_any`] to dispatch only the wraps that
    /// were addressed to this identity, avoiding the "two
    /// identities of the same kind both attempt every wrap" failure
    /// mode (where an unrelated identity's failed AEAD looks
    /// indistinguishable from genuine tamper). Default
    /// implementation returns an empty Vec, which means
    /// `try_unwrap_any` falls back to "try every wrap of matching
    /// kind" — accept this only if your identity is the unique
    /// one of its kind in the system.
    fn recipient_id(&self) -> Vec<u8> {
        Vec::new()
    }

    /// Try to unwrap. Three outcomes:
    ///
    /// * `Ok(None)` — this wrap is for someone else. Caller should
    ///   try the next wrap or the next identity.
    /// * `Ok(Some(file_key))` — success.
    /// * `Err(...)` — for-me-but-broken (tamper, wrong identity
    ///   data within this kind, etc.). Caller should not silently
    ///   continue; this is a security event.
    fn try_unwrap(&self, wrapped: &WrappedKey) -> Result<Option<Key>, RecipientError>;
}

/// Opaque wrapped-key bytes. Inner layout is recipient-specific;
/// upper layers treat as a byte blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedKey(pub Vec<u8>);

// ─────────────────────────────────────────────────────────────────────
// 3. Errors
// ─────────────────────────────────────────────────────────────────────

/// Errors from any [`Recipient`] / [`Identity`] implementation.
///
/// `Display` strings are sourced exclusively from this crate's
/// literals; attacker-controlled bytes never appear in error output.
#[derive(Debug)]
pub enum RecipientError {
    /// AEAD verification failed — wrong identity, tamper, or
    /// wrong-recipient blob handed in. Always fail-closed; we never
    /// silently produce garbage.
    UnwrapFailed,
    /// Wrap blob is malformed (length / magic mismatch).
    MalformedWrappedKey,
    /// Caller passed an empty passphrase or a key of invalid length.
    InvalidParameter {
        /// Static description.
        what: &'static str,
    },
    /// CSPRNG failure during random generation.
    Csprng,
    /// Argon2id KDF failure.
    Kdf,
    /// HKDF failure.
    Hkdf,
    /// X25519 scalarmult failure (e.g. low-order point — the
    /// underlying crate rejects the all-zero identity element).
    X25519,
    /// AEAD encrypt/decrypt failure (typically only on encrypt; on
    /// decrypt we fold tag-mismatch into [`UnwrapFailed`]).
    Aead,
}

impl core::fmt::Display for RecipientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            RecipientError::UnwrapFailed => {
                "vault-recipients: unwrap failed (wrong identity, tamper, or wrong-recipient blob)"
            }
            RecipientError::MalformedWrappedKey => "vault-recipients: wrapped-key blob is malformed",
            RecipientError::InvalidParameter { what } => {
                return write!(f, "vault-recipients: invalid parameter: {}", what);
            }
            RecipientError::Csprng => "vault-recipients: CSPRNG failure",
            RecipientError::Kdf => "vault-recipients: Argon2id KDF failure",
            RecipientError::Hkdf => "vault-recipients: HKDF failure",
            RecipientError::X25519 => "vault-recipients: X25519 scalar-mult failure",
            RecipientError::Aead => "vault-recipients: AEAD encryption failure",
        };
        write!(f, "{}", s)
    }
}

impl std::error::Error for RecipientError {}

impl From<coding_adventures_csprng::CsprngError> for RecipientError {
    fn from(_: coding_adventures_csprng::CsprngError) -> Self {
        RecipientError::Csprng
    }
}

// ─────────────────────────────────────────────────────────────────────
// 4. PassphraseRecipient
// ─────────────────────────────────────────────────────────────────────
//
// Wire format:
//   magic(2) "PR" || salt(16) || nonce(24) || ct(32) || tag(16)  = 90 bytes
//   AAD = magic   (no label here — the recipient's own salted-id
//                  serves as a "this wrap is mine" marker)
//
// The recipient_id is HKDF(passphrase, "PR-id", 32 bytes) — a
// stable, salted "I am this passphrase" identifier so the wrap-set
// layer can dispatch to the right unwrap attempt without trial-
// decrypting every entry.
//
// Drop wipes the passphrase via `Zeroizing<Vec<u8>>`.

const PASSPHRASE_MAGIC: &[u8; 2] = b"PR";

/// A `Recipient` whose wrapping key is derived from a passphrase.
pub struct PassphraseRecipient {
    passphrase: Zeroizing<Vec<u8>>,
    time_cost: u32,
    memory_cost: u32,
    parallelism: u32,
}

impl Drop for PassphraseRecipient {
    fn drop(&mut self) {
        self.passphrase.zeroize();
    }
}

impl PassphraseRecipient {
    /// Build with default Argon2id parameters.
    pub fn with_default_params(passphrase: impl Into<Vec<u8>>) -> Result<Self, RecipientError> {
        Self::with_params(
            passphrase,
            DEFAULT_ARGON_TIME_COST,
            DEFAULT_ARGON_MEMORY_KIB,
            DEFAULT_ARGON_PARALLELISM,
        )
    }

    /// Build with explicit Argon2id parameters.
    pub fn with_params(
        passphrase: impl Into<Vec<u8>>,
        time_cost: u32,
        memory_cost: u32,
        parallelism: u32,
    ) -> Result<Self, RecipientError> {
        // Wrap into Zeroizing immediately so even a rejected
        // passphrase is wiped on drop.
        let pw: Zeroizing<Vec<u8>> = Zeroizing::new(passphrase.into());
        if pw.is_empty() {
            return Err(RecipientError::InvalidParameter { what: "passphrase is empty" });
        }
        if time_cost == 0 || memory_cost < 8 || parallelism == 0 {
            return Err(RecipientError::InvalidParameter {
                what: "Argon2id parameters too small (time>=1, memory>=8 KiB, parallelism>=1)",
            });
        }
        Ok(Self { passphrase: pw, time_cost, memory_cost, parallelism })
    }

    /// Stable identity bytes for this passphrase recipient. Computed
    /// as `HKDF(salt = "PR-id-v1", ikm = passphrase, info = "",
    /// length = 32, SHA-256)` — same passphrase yields same id.
    fn compute_id(&self) -> Result<Vec<u8>, RecipientError> {
        hkdf(b"PR-id-v1", &self.passphrase, b"", 32, HashAlgorithm::Sha256)
            .map_err(|_| RecipientError::Hkdf)
    }

    /// Derive the AEAD-wrap KEK from passphrase + salt via Argon2id.
    fn derive_wrap_key(&self, salt: &[u8]) -> Result<Key, RecipientError> {
        let opts = ArgonOptions { key: None, associated_data: None, version: None };
        let tag = argon2id(
            &self.passphrase,
            salt,
            self.time_cost,
            self.memory_cost,
            self.parallelism,
            KEY_LEN as u32,
            &opts,
        )
        .map_err(|_| RecipientError::Kdf)?;
        if tag.len() != KEY_LEN {
            return Err(RecipientError::Kdf);
        }
        let mut k = Zeroizing::new([0u8; KEY_LEN]);
        k.copy_from_slice(&tag);
        let mut tag_z = Zeroizing::new(tag);
        tag_z.zeroize();
        Ok(k)
    }
}

impl Recipient for PassphraseRecipient {
    fn kind(&self) -> &'static str {
        "passphrase"
    }

    fn recipient_id(&self) -> Vec<u8> {
        // Suppress the (very rare) Hkdf error path by falling back
        // to an empty id — callers see this as "no id" and can
        // treat the wrap as untagged. We deliberately don't propagate
        // the error here because recipient_id is supposed to be
        // infallible from the trait's perspective.
        self.compute_id().unwrap_or_default()
    }

    fn wrap(&self, file_key: &Key) -> Result<WrappedKey, RecipientError> {
        let mut salt = [0u8; SALT_LEN];
        fill_random(&mut salt)?;
        let mut nonce = [0u8; NONCE_LEN];
        fill_random(&mut nonce)?;
        let kek = self.derive_wrap_key(&salt)?;
        let aad = PASSPHRASE_MAGIC.as_slice();
        let (ct, tag) = xchacha20_poly1305_aead_encrypt(&**file_key, &*kek, &nonce, aad);
        if ct.len() != KEY_LEN {
            return Err(RecipientError::Aead);
        }
        let mut blob = Vec::with_capacity(2 + SALT_LEN + NONCE_LEN + KEY_LEN + TAG_LEN);
        blob.extend_from_slice(PASSPHRASE_MAGIC);
        blob.extend_from_slice(&salt);
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ct);
        blob.extend_from_slice(&tag);
        Ok(WrappedKey(blob))
    }
}

impl Identity for PassphraseRecipient {
    fn kind(&self) -> &'static str {
        "passphrase"
    }

    fn recipient_id(&self) -> Vec<u8> {
        // Same as the `Recipient` impl above — HKDF of the passphrase.
        self.compute_id().unwrap_or_default()
    }

    fn try_unwrap(&self, wrapped: &WrappedKey) -> Result<Option<Key>, RecipientError> {
        let blob = &wrapped.0;
        let want_len = 2 + SALT_LEN + NONCE_LEN + KEY_LEN + TAG_LEN;
        // "Not for me" if magic doesn't match. We don't return an
        // error here — different magic = different recipient kind.
        if blob.len() < 2 {
            return Err(RecipientError::MalformedWrappedKey);
        }
        if &blob[..2] != PASSPHRASE_MAGIC {
            return Ok(None);
        }
        if blob.len() != want_len {
            return Err(RecipientError::MalformedWrappedKey);
        }
        let mut p = 2;
        let salt = &blob[p..p + SALT_LEN];
        p += SALT_LEN;
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&blob[p..p + NONCE_LEN]);
        p += NONCE_LEN;
        let ct = &blob[p..p + KEY_LEN];
        p += KEY_LEN;
        let mut tag = [0u8; TAG_LEN];
        tag.copy_from_slice(&blob[p..p + TAG_LEN]);

        let kek = self.derive_wrap_key(salt)?;
        let aad = PASSPHRASE_MAGIC.as_slice();
        let pt = xchacha20_poly1305_aead_decrypt(ct, &*kek, &nonce, aad, &tag)
            .ok_or(RecipientError::UnwrapFailed)?;
        if pt.len() != KEY_LEN {
            return Err(RecipientError::Aead);
        }
        let mut k = Zeroizing::new([0u8; KEY_LEN]);
        k.copy_from_slice(&pt);
        let mut pt_z = Zeroizing::new(pt);
        pt_z.zeroize();
        Ok(Some(k))
    }
}

// ─────────────────────────────────────────────────────────────────────
// 5. X25519Recipient
// ─────────────────────────────────────────────────────────────────────
//
// age-style asymmetric wrap. Each wrap:
//
//   1. Sender draws a fresh ephemeral X25519 keypair (e_sk, e_pk).
//   2. Computes shared = X25519(e_sk, recipient_pk).
//   3. wrap_key = HKDF(salt = e_pk || recipient_pk,
//                      ikm = shared,
//                      info = "VLT04-X25519-wrap-v1",
//                      length = 32, SHA-256).
//   4. AEAD-encrypts file_key under wrap_key with AAD = magic ||
//      e_pk || recipient_pk.
//   5. Stores e_pk in the blob so the recipient can recompute.
//
// Receiver:
//   1. Reads e_pk from the blob.
//   2. Computes shared = X25519(my_sk, e_pk).  (DH symmetry.)
//   3. Re-derives wrap_key with the same HKDF salt/info.
//   4. AEAD-decrypts.
//
// Properties:
//   * Forward secrecy of *the sender's identity*: the sender
//     destroys e_sk after the wrap; an attacker who later seizes
//     the sender's storage learns nothing about which file_keys it
//     produced.
//   * The AAD binds the ciphertext to (e_pk, recipient_pk), so a
//     captured wrap can't be rebound to a different recipient.

const X25519_MAGIC: &[u8; 2] = b"X1";

/// A `Recipient` that wraps to a single X25519 public key.
pub struct X25519Recipient {
    public_key: [u8; X25519_KEY_LEN],
}

impl X25519Recipient {
    /// Build a recipient from a 32-byte X25519 public key.
    pub fn from_public_key(pk: [u8; X25519_KEY_LEN]) -> Self {
        Self { public_key: pk }
    }
}

impl Recipient for X25519Recipient {
    fn kind(&self) -> &'static str {
        "x25519"
    }

    fn recipient_id(&self) -> Vec<u8> {
        self.public_key.to_vec()
    }

    fn wrap(&self, file_key: &Key) -> Result<WrappedKey, RecipientError> {
        // 1. Fresh ephemeral keypair.
        let mut e_sk = Zeroizing::new([0u8; X25519_KEY_LEN]);
        fill_random(&mut e_sk[..])?;
        let e_pk = x25519_base(&e_sk).map_err(|_| RecipientError::X25519)?;

        // 2. ECDH.
        let mut shared = Zeroizing::new(x25519(&e_sk, &self.public_key).map_err(|_| RecipientError::X25519)?);
        // shared is now the 32-byte raw X25519 output.

        // 3. HKDF derive wrap_key.
        let mut hkdf_salt = Vec::with_capacity(2 * X25519_KEY_LEN);
        hkdf_salt.extend_from_slice(&e_pk);
        hkdf_salt.extend_from_slice(&self.public_key);
        let okm = hkdf(
            &hkdf_salt,
            &shared[..],
            b"VLT04-X25519-wrap-v1",
            KEY_LEN,
            HashAlgorithm::Sha256,
        )
        .map_err(|_| RecipientError::Hkdf)?;
        if okm.len() != KEY_LEN {
            return Err(RecipientError::Hkdf);
        }
        let mut wrap_key = Zeroizing::new([0u8; KEY_LEN]);
        wrap_key.copy_from_slice(&okm);
        let mut okm_z = Zeroizing::new(okm);
        okm_z.zeroize();
        // shared is dropped (zeroized) when we leave this scope.
        shared.zeroize();

        // 4. AEAD encrypt file_key.
        let mut nonce = [0u8; NONCE_LEN];
        fill_random(&mut nonce)?;
        let aad = build_x25519_aad(&e_pk, &self.public_key);
        let (ct, tag) = xchacha20_poly1305_aead_encrypt(&**file_key, &*wrap_key, &nonce, &aad);
        if ct.len() != KEY_LEN {
            return Err(RecipientError::Aead);
        }

        // 5. Compose blob.
        let mut blob = Vec::with_capacity(2 + X25519_KEY_LEN + NONCE_LEN + KEY_LEN + TAG_LEN);
        blob.extend_from_slice(X25519_MAGIC);
        blob.extend_from_slice(&e_pk);
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ct);
        blob.extend_from_slice(&tag);
        Ok(WrappedKey(blob))
        // e_sk dropped (zeroized) here — sender no longer holds it.
    }
}

/// The unwrap side of an `X25519Recipient`. Owns the recipient's
/// X25519 *secret* key. Drop wipes it via `Zeroizing`.
pub struct X25519Identity {
    secret_key: Zeroizing<[u8; X25519_KEY_LEN]>,
    /// Cached public key — derived from secret_key in the constructor.
    public_key: [u8; X25519_KEY_LEN],
}

impl X25519Identity {
    /// Build an identity from a 32-byte secret key. The caller passes
    /// the key inside `Zeroizing` so that *no* stack-resident copy of
    /// the bytes survives this function — the wrapper drops at end
    /// of the function unless we move it into the new identity.
    /// The corresponding public key is derived via the X25519 base
    /// scalarmult.
    pub fn from_secret_key(sk: Zeroizing<[u8; X25519_KEY_LEN]>) -> Result<Self, RecipientError> {
        let pk = x25519_base(&sk).map_err(|_| RecipientError::X25519)?;
        // Move the wrapped key into the identity. No further copy of
        // the underlying [u8; 32] is taken.
        Ok(Self { secret_key: sk, public_key: pk })
    }

    /// Generate a fresh X25519 identity. The new secret key is
    /// allocated inside `Zeroizing` from the start; no plain
    /// `[u8; 32]` stack copy is ever created.
    pub fn generate() -> Result<Self, RecipientError> {
        let mut sk = Zeroizing::new([0u8; X25519_KEY_LEN]);
        fill_random(&mut sk[..])?;
        Self::from_secret_key(sk)
    }

    /// Public-key view of this identity (the corresponding
    /// `X25519Recipient` for `wrap`).
    pub fn recipient(&self) -> X25519Recipient {
        X25519Recipient::from_public_key(self.public_key)
    }
}

impl Identity for X25519Identity {
    fn kind(&self) -> &'static str {
        "x25519"
    }

    fn recipient_id(&self) -> Vec<u8> {
        // Matches X25519Recipient::recipient_id.
        self.public_key.to_vec()
    }

    fn try_unwrap(&self, wrapped: &WrappedKey) -> Result<Option<Key>, RecipientError> {
        let blob = &wrapped.0;
        let want_len = 2 + X25519_KEY_LEN + NONCE_LEN + KEY_LEN + TAG_LEN;
        if blob.len() < 2 {
            return Err(RecipientError::MalformedWrappedKey);
        }
        if &blob[..2] != X25519_MAGIC {
            return Ok(None);
        }
        if blob.len() != want_len {
            return Err(RecipientError::MalformedWrappedKey);
        }
        let mut p = 2;
        let mut e_pk = [0u8; X25519_KEY_LEN];
        e_pk.copy_from_slice(&blob[p..p + X25519_KEY_LEN]);
        p += X25519_KEY_LEN;
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&blob[p..p + NONCE_LEN]);
        p += NONCE_LEN;
        let ct = &blob[p..p + KEY_LEN];
        p += KEY_LEN;
        let mut tag = [0u8; TAG_LEN];
        tag.copy_from_slice(&blob[p..p + TAG_LEN]);

        // ECDH on the receive side.
        let mut shared = Zeroizing::new(x25519(&self.secret_key, &e_pk).map_err(|_| RecipientError::X25519)?);
        let mut hkdf_salt = Vec::with_capacity(2 * X25519_KEY_LEN);
        hkdf_salt.extend_from_slice(&e_pk);
        hkdf_salt.extend_from_slice(&self.public_key);
        let okm = hkdf(&hkdf_salt, &shared[..], b"VLT04-X25519-wrap-v1", KEY_LEN, HashAlgorithm::Sha256)
            .map_err(|_| RecipientError::Hkdf)?;
        if okm.len() != KEY_LEN {
            return Err(RecipientError::Hkdf);
        }
        let mut wrap_key = Zeroizing::new([0u8; KEY_LEN]);
        wrap_key.copy_from_slice(&okm);
        let mut okm_z = Zeroizing::new(okm);
        okm_z.zeroize();
        shared.zeroize();

        let aad = build_x25519_aad(&e_pk, &self.public_key);
        let pt = xchacha20_poly1305_aead_decrypt(ct, &*wrap_key, &nonce, &aad, &tag)
            .ok_or(RecipientError::UnwrapFailed)?;
        if pt.len() != KEY_LEN {
            return Err(RecipientError::Aead);
        }
        let mut k = Zeroizing::new([0u8; KEY_LEN]);
        k.copy_from_slice(&pt);
        let mut pt_z = Zeroizing::new(pt);
        pt_z.zeroize();
        Ok(Some(k))
    }
}

fn build_x25519_aad(e_pk: &[u8; X25519_KEY_LEN], r_pk: &[u8; X25519_KEY_LEN]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(2 + 2 * X25519_KEY_LEN);
    aad.extend_from_slice(X25519_MAGIC);
    aad.extend_from_slice(e_pk);
    aad.extend_from_slice(r_pk);
    aad
}

// ─────────────────────────────────────────────────────────────────────
// 6. Multi-recipient helpers
// ─────────────────────────────────────────────────────────────────────

/// Wrap a single file key for every recipient in a list. Returns
/// the wrap-set: `[(recipient_id, wrapped), …]` in the same order
/// as `recipients`.
pub fn wrap_for_all(
    file_key: &Key,
    recipients: &[&dyn Recipient],
) -> Result<Vec<(Vec<u8>, WrappedKey)>, RecipientError> {
    let mut out = Vec::with_capacity(recipients.len());
    for r in recipients {
        let id = r.recipient_id();
        let w = r.wrap(file_key)?;
        out.push((id, w));
    }
    Ok(out)
}

/// Try the wraps addressed to each identity (matched by
/// `recipient_id`) and return the first successfully recovered
/// file key.
///
/// Dispatch rule:
/// * For each identity, only wraps whose stored `recipient_id`
///   equals the identity's own `recipient_id()` are attempted.
///   This avoids the "two identities of the same kind both attempt
///   every wrap and one's failed AEAD looks like genuine tamper"
///   failure mode.
/// * If an identity's `recipient_id()` returns the empty Vec
///   (default impl), every wrap of matching `kind` is tried —
///   safe only when the identity is the unique one of its kind.
///
/// Outcomes:
/// * `Ok(Some(file_key))` — first successful unwrap.
/// * `Ok(None)` — no identity had a matching wrap (or matching
///   wraps cleanly returned `Ok(None)` from the underlying
///   `try_unwrap`).
/// * `Err(...)` — a matching wrap was attempted and failed
///   (`UnwrapFailed`, `MalformedWrappedKey`, etc.). Tamper of a
///   wrap addressed to me is propagated as a security event, not
///   silently skipped.
pub fn try_unwrap_any(
    identities: &[&dyn Identity],
    wrap_set: &[(Vec<u8>, WrappedKey)],
) -> Result<Option<Key>, RecipientError> {
    for id in identities {
        let my_id = id.recipient_id();
        for (rid, w) in wrap_set {
            // Skip wraps not addressed to this identity. Empty
            // my_id means "try every wrap" (default-impl path).
            if !my_id.is_empty() && rid != &my_id {
                continue;
            }
            match id.try_unwrap(w)? {
                Some(k) => return Ok(Some(k)),
                None => continue,
            }
        }
    }
    Ok(None)
}

// ─────────────────────────────────────────────────────────────────────
// 7. Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_ct_compare::ct_eq;

    fn fast_pr(pw: &[u8]) -> PassphraseRecipient {
        PassphraseRecipient::with_params(pw.to_vec(), 1, 8, 1).unwrap()
    }

    fn random_key() -> Key {
        fresh_file_key().unwrap()
    }

    // --- PassphraseRecipient ---

    #[test]
    fn passphrase_wrap_unwrap_roundtrip() {
        let r = fast_pr(b"correct horse battery staple");
        let fk = random_key();
        let wrapped = r.wrap(&fk).unwrap();
        let recovered = r.try_unwrap(&wrapped).unwrap().expect("for me");
        assert!(ct_eq(&*fk, &*recovered));
    }

    #[test]
    fn passphrase_wrap_distinct_blobs_each_call() {
        let r = fast_pr(b"pw");
        let fk = random_key();
        let a = r.wrap(&fk).unwrap();
        let b = r.wrap(&fk).unwrap();
        assert_ne!(a.0, b.0); // fresh salt + nonce per wrap
    }

    #[test]
    fn passphrase_recipient_id_is_stable() {
        let r1 = fast_pr(b"my passphrase");
        let r2 = fast_pr(b"my passphrase");
        // Disambiguate: PassphraseRecipient implements both
        // Recipient and Identity, both with `recipient_id`.
        assert_eq!(Recipient::recipient_id(&r1), Recipient::recipient_id(&r2));
        let r3 = fast_pr(b"different");
        assert_ne!(Recipient::recipient_id(&r1), Recipient::recipient_id(&r3));
        // The Identity impl agrees:
        assert_eq!(Recipient::recipient_id(&r1), Identity::recipient_id(&r1));
    }

    #[test]
    fn passphrase_wrong_passphrase_unwrap_fails() {
        let good = fast_pr(b"good");
        let bad = fast_pr(b"bad");
        let fk = random_key();
        let wrapped = good.wrap(&fk).unwrap();
        match bad.try_unwrap(&wrapped) {
            Err(RecipientError::UnwrapFailed) => {}
            other => panic!("expected UnwrapFailed, got something else"),
        }
    }

    #[test]
    fn passphrase_tamper_fails() {
        let r = fast_pr(b"pw");
        let fk = random_key();
        let mut wrapped = r.wrap(&fk).unwrap();
        wrapped.0[2 + SALT_LEN + NONCE_LEN] ^= 0x01; // flip ct byte
        match r.try_unwrap(&wrapped) {
            Err(RecipientError::UnwrapFailed) => {}
            other => panic!("expected UnwrapFailed, got something else"),
        }
    }

    // --- X25519Recipient / X25519Identity ---

    #[test]
    fn x25519_wrap_unwrap_roundtrip() {
        let id = X25519Identity::generate().unwrap();
        let recip = id.recipient();
        let fk = random_key();
        let wrapped = recip.wrap(&fk).unwrap();
        let recovered = id.try_unwrap(&wrapped).unwrap().expect("for me");
        assert!(ct_eq(&*fk, &*recovered));
    }

    #[test]
    fn x25519_wrap_blob_has_expected_length() {
        let id = X25519Identity::generate().unwrap();
        let recip = id.recipient();
        let fk = random_key();
        let wrapped = recip.wrap(&fk).unwrap();
        // 2 + 32 + 24 + 32 + 16 = 106
        assert_eq!(wrapped.0.len(), 106);
    }

    #[test]
    fn x25519_wrap_distinct_blobs_each_call() {
        let id = X25519Identity::generate().unwrap();
        let recip = id.recipient();
        let fk = random_key();
        let a = recip.wrap(&fk).unwrap();
        let b = recip.wrap(&fk).unwrap();
        assert_ne!(a.0, b.0); // fresh ephemeral keypair per wrap
    }

    #[test]
    fn x25519_unwrap_with_wrong_identity_fails() {
        let alice = X25519Identity::generate().unwrap();
        let bob = X25519Identity::generate().unwrap();
        let fk = random_key();
        let wrapped = alice.recipient().wrap(&fk).unwrap();
        // Bob tries to unwrap a wrap intended for Alice. The magic
        // matches (both X25519), so Bob attempts it and AEAD fails.
        match bob.try_unwrap(&wrapped) {
            Err(RecipientError::UnwrapFailed) => {}
            other => panic!("expected UnwrapFailed, got something else"),
        }
    }

    #[test]
    fn x25519_unwrap_tamper_fails() {
        let id = X25519Identity::generate().unwrap();
        let fk = random_key();
        let mut wrapped = id.recipient().wrap(&fk).unwrap();
        // Flip a byte in the ephemeral pubkey.
        wrapped.0[3] ^= 0x01;
        match id.try_unwrap(&wrapped) {
            Err(RecipientError::UnwrapFailed) => {}
            other => panic!("expected UnwrapFailed, got something else"),
        }
    }

    #[test]
    fn x25519_recipient_id_equals_pubkey() {
        let id = X25519Identity::generate().unwrap();
        let r = id.recipient();
        assert_eq!(r.recipient_id(), id.public_key.to_vec());
    }

    // --- Cross-kind: identity says "not for me" on wrong magic ---

    #[test]
    fn passphrase_identity_returns_none_on_x25519_blob() {
        let pr = fast_pr(b"pw");
        let id = X25519Identity::generate().unwrap();
        let fk = random_key();
        let x25519_blob = id.recipient().wrap(&fk).unwrap();
        match pr.try_unwrap(&x25519_blob) {
            Ok(None) => {}
            other => panic!("expected Ok(None), got something else"),
        }
    }

    #[test]
    fn x25519_identity_returns_none_on_passphrase_blob() {
        let pr = fast_pr(b"pw");
        let id = X25519Identity::generate().unwrap();
        let fk = random_key();
        let pw_blob = pr.wrap(&fk).unwrap();
        match id.try_unwrap(&pw_blob) {
            Ok(None) => {}
            other => panic!("expected Ok(None), got something else"),
        }
    }

    // --- Multi-recipient flow ---

    #[test]
    fn multi_recipient_alice_and_bob_both_unwrap() {
        let alice = X25519Identity::generate().unwrap();
        let bob = X25519Identity::generate().unwrap();
        let alice_r = alice.recipient();
        let bob_r = bob.recipient();
        let fk = random_key();
        let wrap_set = wrap_for_all(&fk, &[&alice_r as &dyn Recipient, &bob_r]).unwrap();
        assert_eq!(wrap_set.len(), 2);

        // Each can recover.
        let recovered_a = try_unwrap_any(&[&alice as &dyn Identity], &wrap_set).unwrap().unwrap();
        let recovered_b = try_unwrap_any(&[&bob as &dyn Identity], &wrap_set).unwrap().unwrap();
        assert!(ct_eq(&*fk, &*recovered_a));
        assert!(ct_eq(&*fk, &*recovered_b));
    }

    #[test]
    fn multi_recipient_third_party_recovers_nothing() {
        let alice = X25519Identity::generate().unwrap();
        let bob = X25519Identity::generate().unwrap();
        let eve = X25519Identity::generate().unwrap();
        let fk = random_key();
        let wrap_set = wrap_for_all(&fk, &[&alice.recipient() as &dyn Recipient, &bob.recipient()])
            .unwrap();
        // Eve's recipient_id (her own pubkey) doesn't match either
        // Alice's or Bob's wrap label, so try_unwrap_any skips both
        // and returns Ok(None). Eve learns nothing — and the loop
        // is short-circuited so Eve doesn't even get to attempt
        // ECDH/AEAD against either wrap.
        match try_unwrap_any(&[&eve as &dyn Identity], &wrap_set) {
            Ok(None) => {}
            other => panic!("expected Ok(None), got something else (got {})",
                            if matches!(other, Ok(Some(_))) { "Ok(Some)" }
                            else if matches!(other, Err(_)) { "Err" }
                            else { "Ok(None)" }),
        }
    }

    #[test]
    fn multi_recipient_mixed_kinds() {
        let pr = fast_pr(b"shared password");
        let xid = X25519Identity::generate().unwrap();
        let fk = random_key();
        let wrap_set = wrap_for_all(&fk, &[&pr as &dyn Recipient, &xid.recipient()]).unwrap();
        assert_eq!(wrap_set.len(), 2);

        // Passphrase identity recovers from its wrap, returns None on the X25519 wrap.
        let r1 = try_unwrap_any(&[&pr as &dyn Identity], &wrap_set).unwrap().unwrap();
        // X25519 identity recovers from its wrap.
        let r2 = try_unwrap_any(&[&xid as &dyn Identity], &wrap_set).unwrap().unwrap();
        assert!(ct_eq(&*fk, &*r1));
        assert!(ct_eq(&*fk, &*r2));
    }

    #[test]
    fn try_unwrap_any_propagates_tamper_for_addressed_wrap() {
        // A tampered wrap addressed to me is a security event, not a
        // "skip and try the next one" condition.
        let alice = X25519Identity::generate().unwrap();
        let fk = random_key();
        let mut wrap_set = wrap_for_all(&fk, &[&alice.recipient() as &dyn Recipient]).unwrap();
        // Tamper inside Alice's wrap.
        wrap_set[0].1 .0[5] ^= 0x01;
        match try_unwrap_any(&[&alice as &dyn Identity], &wrap_set) {
            Err(RecipientError::UnwrapFailed) => {}
            other => panic!("expected UnwrapFailed, got something else"),
        }
    }

    // --- Parameter validation ---

    #[test]
    fn empty_passphrase_rejected() {
        match PassphraseRecipient::with_default_params(Vec::new()) {
            Err(RecipientError::InvalidParameter { .. }) => {}
            other => panic!("expected InvalidParameter, got something else"),
        }
    }

    // --- Errors come from literals ---

    #[test]
    fn error_messages_are_static_literals() {
        let errs: Vec<RecipientError> = vec![
            RecipientError::UnwrapFailed,
            RecipientError::MalformedWrappedKey,
            RecipientError::InvalidParameter { what: "x" },
            RecipientError::Csprng,
            RecipientError::Kdf,
            RecipientError::Hkdf,
            RecipientError::X25519,
            RecipientError::Aead,
        ];
        for e in &errs {
            let s = e.to_string();
            assert!(s.starts_with("vault-recipients:"));
        }
    }

    // --- Drop is wired ---

    #[test]
    fn passphrase_recipient_drop_safe() {
        let r = fast_pr(b"top secret");
        let _ = Recipient::recipient_id(&r);
        drop(r);
    }

    #[test]
    fn x25519_identity_drop_safe() {
        let id = X25519Identity::generate().unwrap();
        let _ = id.recipient();
        drop(id);
    }

    // --- Malformed blobs ---

    #[test]
    fn unwrap_too_short_blob_is_malformed() {
        let r = fast_pr(b"pw");
        let bad = WrappedKey(vec![]);
        match r.try_unwrap(&bad) {
            Err(RecipientError::MalformedWrappedKey) => {}
            other => panic!("expected MalformedWrappedKey, got something else"),
        }
    }

    #[test]
    fn unwrap_truncated_passphrase_blob_is_malformed() {
        let r = fast_pr(b"pw");
        let fk = random_key();
        let mut wrapped = r.wrap(&fk).unwrap();
        wrapped.0.truncate(10);
        match r.try_unwrap(&wrapped) {
            Err(RecipientError::MalformedWrappedKey) => {}
            other => panic!("expected MalformedWrappedKey, got something else"),
        }
    }

    #[test]
    fn unwrap_truncated_x25519_blob_is_malformed() {
        let id = X25519Identity::generate().unwrap();
        let fk = random_key();
        let mut wrapped = id.recipient().wrap(&fk).unwrap();
        wrapped.0.truncate(50);
        match id.try_unwrap(&wrapped) {
            Err(RecipientError::MalformedWrappedKey) => {}
            other => panic!("expected MalformedWrappedKey, got something else"),
        }
    }
}
