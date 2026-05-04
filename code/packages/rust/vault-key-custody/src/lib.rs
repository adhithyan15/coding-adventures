//! # coding_adventures_vault_key_custody — VLT03
//!
//! ## What this crate does
//!
//! VLT01 (the sealed store) needs a 32-byte **master KEK** to wrap
//! per-record DEKs under. *Where* that KEK lives is a separate
//! question with a wide design space:
//!
//! * a passphrase typed by the user, stretched with Argon2id;
//! * a key inside the OS keystore (macOS Keychain, Windows DPAPI,
//!   libsecret) released only on user login;
//! * a key inside a hardware token: TPM 2.0, Apple Secure Enclave,
//!   Android StrongBox, YubiKey FIDO2-PRF;
//! * a key inside a PKCS#11 HSM (CloudHSM, SoftHSM, smartcards);
//! * a key inside a Cloud KMS (AWS KMS, GCP Cloud KMS, Azure Key
//!   Vault Managed HSM) — the auto-unseal pattern from HashiCorp
//!   Vault.
//!
//! Each of these has very different operational semantics — some are
//! extractable, some are not; some require user presence on every
//! unwrap, some don't; some are bound to a hardware secret, some are
//! not. VLT03 abstracts these behind one trait so the rest of the
//! Vault stack doesn't have to care which is in use.
//!
//! ## TPM-first / hardware-preferred (refinement 2026-05-04)
//!
//! When the host machine has a hardware custodian available (TPM,
//! Secure Enclave, …), the vault **refuses** to instantiate a
//! software (passphrase) custodian unless the caller explicitly
//! passes a `force_software` flag. The intent is that on a normal
//! laptop with a TPM, secrets that come out of the custodian
//! *never* live in user-space process heap in extractable form —
//! all unwrap operations cross the TPM boundary, the unwrapped key
//! is held only briefly inside the wrapping `Zeroizing<…>`, and the
//! application can never accidentally hand a software-derived key
//! to a hardware-bound deployment. Side-channel attack surface is
//! correspondingly reduced (cold-boot RAM scraping, debugger
//! attaches, swap files, core dumps).
//!
//! ## What's in this crate (v0.1)
//!
//! - `KeyCustodian` trait: capability-reporting + wrap + unwrap.
//! - `CustodianCaps` capability struct — what the vault can ask
//!   ("is this hardware-bound?", "can the key escape the
//!   custodian?", "does unwrap need user presence?").
//! - `PassphraseCustodian` — full implementation, Argon2id KDF +
//!   XChaCha20-Poly1305 wrap.
//! - `TpmCustodian` — *scaffold* that reports the right capability
//!   shape but returns `Unimplemented` from `wrap` / `unwrap` until
//!   the platform-specific TPM 2.0 backend lands in a follow-up PR.
//!   Capability-reporting is wired now so downstream callers
//!   (`select_custodian`) can already make TPM-first / fallback
//!   decisions.
//! - `select_custodian` — the policy helper that, given a list of
//!   candidates and a `force_software` flag, picks the preferred
//!   custodian per the TPM-first rule.
//!
//! Future PRs add: `OSKeystoreCustodian` (Keychain / DPAPI /
//! libsecret), `SecureEnclaveCustodian` (Apple), `Pkcs11Custodian`,
//! `AwsKmsCustodian`, `GcpKmsCustodian`, `AzureKvCustodian`,
//! `YubikeyPrfCustodian`, `DistributedFragmentCustodian` (Shamir-
//! based quorum unseal).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_argon2id::{argon2id, Options as ArgonOptions};
use coding_adventures_chacha20_poly1305::{
    xchacha20_poly1305_aead_decrypt, xchacha20_poly1305_aead_encrypt,
};
use coding_adventures_csprng::fill_random;
use coding_adventures_zeroize::{Zeroize, Zeroizing};

// ─────────────────────────────────────────────────────────────────────
// 1. Constants & types
// ─────────────────────────────────────────────────────────────────────

/// Length of every key this crate handles, in bytes (256-bit).
pub const KEY_LEN: usize = 32;
/// Length of an XChaCha20-Poly1305 nonce, in bytes (192-bit).
pub const NONCE_LEN: usize = 24;
/// Length of a Poly1305 tag, in bytes.
pub const TAG_LEN: usize = 16;
/// Default Argon2id time-cost (number of passes). Tuned for
/// >= 250 ms on modern hardware.
pub const DEFAULT_ARGON_TIME_COST: u32 = 3;
/// Default Argon2id memory in KiB.
pub const DEFAULT_ARGON_MEMORY_KIB: u32 = 64 * 1024;
/// Default Argon2id parallelism.
pub const DEFAULT_ARGON_PARALLELISM: u32 = 4;
/// Argon2id salt length, in bytes.
pub const SALT_LEN: usize = 16;

/// What an opaque label looks like — opaque to this crate, used
/// only by the custodian to find the right wrapping key. For
/// `PassphraseCustodian` it's any byte string; for
/// `TpmCustodian` it's typically a TPM persistent handle.
pub type Label = Vec<u8>;

/// A 32-byte symmetric key — held inside `Zeroizing` everywhere we
/// can to ensure on-drop wiping.
pub type Key = Zeroizing<[u8; KEY_LEN]>;

/// Returns a fresh fully-random `Key`. Used by callers that need to
/// ask the custodian to wrap an ephemeral key (e.g. a vault
/// master KEK).
pub fn fresh_random_key() -> Result<Key, CustodyError> {
    let mut k = Zeroizing::new([0u8; KEY_LEN]);
    fill_random(&mut k[..])?;
    Ok(k)
}

// ─────────────────────────────────────────────────────────────────────
// 2. CustodianCaps — what does this custodian guarantee?
// ─────────────────────────────────────────────────────────────────────

/// Capability flags reported by a custodian. The vault uses these
/// to make security decisions: e.g. "we have a hardware custodian
/// available, refuse to fall back to passphrase."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CustodianCaps {
    /// True iff the wrapping key material is bound to a hardware
    /// secret (TPM, Secure Enclave, HSM, YubiKey). When true, the
    /// custodian's unwrap operation crosses a hardware boundary
    /// and the wrapping key cannot be exfiltrated by reading
    /// process memory.
    pub hardware_bound: bool,
    /// True iff the wrapping key material can be exported from the
    /// custodian. Software passphrase custodians have this true
    /// (the derived key is in heap during the operation). HSMs and
    /// TPMs typically have this false.
    pub extractable: bool,
    /// True iff every unwrap requires explicit user presence (e.g.
    /// touch-the-YubiKey, biometric prompt). Affects UX, not
    /// security per se, but the vault must know to plan around it.
    pub requires_user_presence: bool,
    /// True iff this custodian's unwrap is plausibly remote
    /// (network round-trip to a Cloud KMS). Affects performance
    /// budgets and offline behaviour.
    pub remote: bool,
}

impl CustodianCaps {
    /// All-software, host-only baseline (what a `PassphraseCustodian`
    /// reports).
    pub const SOFTWARE: CustodianCaps = CustodianCaps {
        hardware_bound: false,
        extractable: true,
        requires_user_presence: false,
        remote: false,
    };
    /// What a TPM 2.0 / Secure Enclave-backed custodian reports.
    pub const HARDWARE_LOCAL: CustodianCaps = CustodianCaps {
        hardware_bound: true,
        extractable: false,
        requires_user_presence: false,
        remote: false,
    };
}

// ─────────────────────────────────────────────────────────────────────
// 3. The `KeyCustodian` trait
// ─────────────────────────────────────────────────────────────────────

/// Pluggable key custodian. Implementations: `PassphraseCustodian`,
/// `TpmCustodian` (stub), `OSKeystoreCustodian` (future), etc.
///
/// The trait is intentionally narrow — three methods. Anything more
/// complex (lifecycle, multi-step ceremonies, user-presence prompts)
/// belongs in the implementation, not in the trait.
pub trait KeyCustodian {
    /// Stable identifier for telemetry and error messages, e.g.
    /// `"passphrase"`, `"tpm-2.0"`, `"yubikey-prf"`.
    fn name(&self) -> &str;

    /// What does this custodian guarantee? Used by
    /// [`select_custodian`] to enforce the TPM-first preference.
    fn capabilities(&self) -> CustodianCaps;

    /// Wrap a 32-byte key. The result is some opaque byte string
    /// that this same custodian can later `unwrap` — exactly what
    /// is in the bytes is implementation-specific.
    fn wrap(&self, label: &Label, key: &Key) -> Result<WrappedKey, CustodyError>;

    /// Unwrap a previously-wrapped key. Tampering or wrong-label
    /// returns an error; we never silently produce garbage.
    fn unwrap(&self, label: &Label, wrapped: &WrappedKey) -> Result<Key, CustodyError>;
}

/// Opaque wrapped-key bytes. The concrete layout is up to the
/// custodian implementation; consumers treat it as a byte blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedKey(pub Vec<u8>);

// ─────────────────────────────────────────────────────────────────────
// 4. Errors
// ─────────────────────────────────────────────────────────────────────

/// Errors from any [`KeyCustodian`] implementation.
///
/// `Display` strings come exclusively from this crate's literals;
/// attacker-controlled bytes never appear in error output.
#[derive(Debug)]
pub enum CustodyError {
    /// Wrong passphrase (or tamper / wrong label / wrong custodian).
    /// Custodians **always** fail closed; we never decrypt to garbage.
    InvalidPassphrase,
    /// The wrapped-key blob is malformed (wrong length, wrong header).
    MalformedWrappedKey,
    /// Caller passed an empty passphrase or a salt of unexpected length.
    InvalidParameter {
        /// Static description of which parameter is bad.
        what: &'static str,
    },
    /// CSPRNG failure during salt or nonce generation.
    Csprng,
    /// Argon2id KDF failure.
    Kdf,
    /// AEAD encrypt/decrypt failure.
    Aead,
    /// `select_custodian` was asked to fall back to a software
    /// custodian when a hardware custodian was available, and
    /// `force_software` was not set. Refused.
    HardwareAvailableButSoftwareRequested,
    /// `select_custodian` got an empty list of candidates.
    NoCandidates,
    /// `select_custodian` was called with `force_software = true`
    /// but the candidate list contained no software custodians.
    /// Failing closed here (rather than silently returning a
    /// hardware custodian) keeps "I asked for software" from
    /// getting "I got hardware" as a footgun.
    NoSoftwareCandidate,
    /// Custodian implementation is a scaffold not yet usable.
    /// Returned by `TpmCustodian` until the platform-specific TPM
    /// backend lands.
    Unimplemented {
        /// Static name of the backend that needs implementing.
        backend: &'static str,
    },
}

impl core::fmt::Display for CustodyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CustodyError::InvalidPassphrase => write!(
                f,
                "vault-key-custody: unwrap failed (invalid passphrase, wrong label, or tampered ciphertext)"
            ),
            CustodyError::MalformedWrappedKey => {
                write!(f, "vault-key-custody: wrapped-key blob is malformed")
            }
            CustodyError::InvalidParameter { what } => {
                write!(f, "vault-key-custody: invalid parameter: {}", what)
            }
            CustodyError::Csprng => write!(f, "vault-key-custody: CSPRNG failure"),
            CustodyError::Kdf => write!(f, "vault-key-custody: Argon2id KDF failure"),
            CustodyError::Aead => write!(f, "vault-key-custody: AEAD encrypt/decrypt failure"),
            CustodyError::HardwareAvailableButSoftwareRequested => write!(
                f,
                "vault-key-custody: a hardware custodian is available but a software one was requested without force_software"
            ),
            CustodyError::NoCandidates => {
                write!(f, "vault-key-custody: select_custodian called with no candidates")
            }
            CustodyError::NoSoftwareCandidate => write!(
                f,
                "vault-key-custody: force_software was set but the candidate list contains no software custodians"
            ),
            CustodyError::Unimplemented { backend } => {
                write!(f, "vault-key-custody: {} backend not yet implemented in this build", backend)
            }
        }
    }
}

impl std::error::Error for CustodyError {}

impl From<coding_adventures_csprng::CsprngError> for CustodyError {
    fn from(_: coding_adventures_csprng::CsprngError) -> Self {
        CustodyError::Csprng
    }
}

// ─────────────────────────────────────────────────────────────────────
// 5. PassphraseCustodian — the software baseline
// ─────────────────────────────────────────────────────────────────────
//
// Wire format of a wrapped key produced by PassphraseCustodian:
//
//   wrapped = magic(2) || salt(SALT_LEN) || nonce(NONCE_LEN) || ct(KEY_LEN) || tag(TAG_LEN)
//
// Total length = 2 + 16 + 24 + 32 + 16 = 90 bytes.
//
// Magic = b"P1" — "Passphrase v1." Lets the decoder fail-fast on a
// blob from another custodian.
//
// AAD = magic || label  — binds the wrapped key to its label so a
// blob saved under one label can't be replayed under another.
//
// Argon2id input: passphrase, salt, time/memory/parallelism from
// constructor (defaulted via `with_default_params`).

/// Magic bytes prefixing every PassphraseCustodian wrap. Lets the
/// decoder detect "you handed me a different custodian's blob."
const PASSPHRASE_MAGIC: &[u8; 2] = b"P1";

/// `KeyCustodian` whose wrapping key is derived from a passphrase
/// via Argon2id (RFC 9106).
///
/// Drop wipes the held passphrase. Wrap/unwrap operations also
/// derive an ephemeral KEK held in `Zeroizing<[u8; 32]>` so it
/// wipes on the function's stack frame regardless of return path.
pub struct PassphraseCustodian {
    /// User passphrase. Wiped on drop.
    passphrase: Zeroizing<Vec<u8>>,
    /// Argon2id time cost.
    time_cost: u32,
    /// Argon2id memory cost (KiB).
    memory_cost: u32,
    /// Argon2id parallelism.
    parallelism: u32,
}

impl Drop for PassphraseCustodian {
    fn drop(&mut self) {
        // Zeroizing<Vec<u8>> wipes on its own drop, so explicit
        // call is belt-and-braces. Doesn't hurt.
        self.passphrase.zeroize();
    }
}

impl PassphraseCustodian {
    /// Build a `PassphraseCustodian` with the given passphrase and
    /// **default** Argon2id parameters (`time=3`, `memory=64 MiB`,
    /// `parallelism=4` — RFC 9106 recommended baseline).
    pub fn with_default_params(passphrase: impl Into<Vec<u8>>) -> Result<Self, CustodyError> {
        Self::with_params(
            passphrase,
            DEFAULT_ARGON_TIME_COST,
            DEFAULT_ARGON_MEMORY_KIB,
            DEFAULT_ARGON_PARALLELISM,
        )
    }

    /// Build a `PassphraseCustodian` with explicit Argon2id
    /// parameters. Useful for tests (low cost) and for callers that
    /// know their target hardware.
    pub fn with_params(
        passphrase: impl Into<Vec<u8>>,
        time_cost: u32,
        memory_cost: u32,
        parallelism: u32,
    ) -> Result<Self, CustodyError> {
        // Wrap the input passphrase in `Zeroizing` immediately, BEFORE
        // any validation that could fail. If validation rejects the
        // passphrase (empty, bad parameters), `pw` drops as a
        // `Zeroizing<Vec<u8>>` and is wiped — even on the
        // early-return error paths.
        let pw: Zeroizing<Vec<u8>> = Zeroizing::new(passphrase.into());
        if pw.is_empty() {
            return Err(CustodyError::InvalidParameter { what: "passphrase is empty" });
        }
        if time_cost == 0 || memory_cost < 8 || parallelism == 0 {
            return Err(CustodyError::InvalidParameter {
                what: "Argon2id parameters too small (need time>=1, memory>=8 KiB, parallelism>=1)",
            });
        }
        Ok(PassphraseCustodian {
            passphrase: pw,
            time_cost,
            memory_cost,
            parallelism,
        })
    }

    /// Build the AAD that binds a wrapped-key blob to its label.
    /// `magic || label` — the magic guards against custodian-mix-up,
    /// the label binds the blob to its intended slot.
    fn build_aad(label: &[u8]) -> Vec<u8> {
        let mut aad = Vec::with_capacity(PASSPHRASE_MAGIC.len() + label.len());
        aad.extend_from_slice(PASSPHRASE_MAGIC);
        aad.extend_from_slice(label);
        aad
    }

    /// Derive the 32-byte KEK from passphrase + salt via Argon2id.
    fn derive_kek(&self, salt: &[u8]) -> Result<Key, CustodyError> {
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
        .map_err(|_| CustodyError::Kdf)?;
        if tag.len() != KEY_LEN {
            return Err(CustodyError::Kdf);
        }
        let mut k = Zeroizing::new([0u8; KEY_LEN]);
        k.copy_from_slice(&tag);
        // tag itself is a Vec<u8> on the heap; zero it before drop.
        let mut tag_z = Zeroizing::new(tag);
        tag_z.zeroize();
        Ok(k)
    }
}

impl KeyCustodian for PassphraseCustodian {
    fn name(&self) -> &str {
        "passphrase"
    }

    fn capabilities(&self) -> CustodianCaps {
        CustodianCaps::SOFTWARE
    }

    fn wrap(&self, label: &Label, key: &Key) -> Result<WrappedKey, CustodyError> {
        // 1. Fresh salt + nonce.
        let mut salt = [0u8; SALT_LEN];
        fill_random(&mut salt)?;
        let mut nonce = [0u8; NONCE_LEN];
        fill_random(&mut nonce)?;

        // 2. Derive KEK from passphrase + salt.
        let kek = self.derive_kek(&salt)?;

        // 3. AEAD-encrypt the inner key under KEK with AAD = magic || label.
        let aad = Self::build_aad(label);
        let (ct, tag) = xchacha20_poly1305_aead_encrypt(&**key, &*kek, &nonce, &aad);
        // ct.len() should equal KEY_LEN.
        if ct.len() != KEY_LEN {
            return Err(CustodyError::Aead);
        }

        // 4. Compose blob: magic || salt || nonce || ct || tag.
        let mut blob = Vec::with_capacity(PASSPHRASE_MAGIC.len() + SALT_LEN + NONCE_LEN + KEY_LEN + TAG_LEN);
        blob.extend_from_slice(PASSPHRASE_MAGIC);
        blob.extend_from_slice(&salt);
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ct);
        blob.extend_from_slice(&tag);
        Ok(WrappedKey(blob))
    }

    fn unwrap(&self, label: &Label, wrapped: &WrappedKey) -> Result<Key, CustodyError> {
        let blob = &wrapped.0;
        let want_len = PASSPHRASE_MAGIC.len() + SALT_LEN + NONCE_LEN + KEY_LEN + TAG_LEN;
        if blob.len() != want_len {
            return Err(CustodyError::MalformedWrappedKey);
        }
        if &blob[..PASSPHRASE_MAGIC.len()] != PASSPHRASE_MAGIC {
            return Err(CustodyError::MalformedWrappedKey);
        }
        // Slice out the fields.
        let mut p = PASSPHRASE_MAGIC.len();
        let salt = &blob[p..p + SALT_LEN];
        p += SALT_LEN;
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&blob[p..p + NONCE_LEN]);
        p += NONCE_LEN;
        let ct = &blob[p..p + KEY_LEN];
        p += KEY_LEN;
        let mut tag = [0u8; TAG_LEN];
        tag.copy_from_slice(&blob[p..p + TAG_LEN]);

        // Re-derive KEK and AAD.
        let kek = self.derive_kek(salt)?;
        let aad = Self::build_aad(label);

        let pt = xchacha20_poly1305_aead_decrypt(ct, &*kek, &nonce, &aad, &tag)
            .ok_or(CustodyError::InvalidPassphrase)?;
        if pt.len() != KEY_LEN {
            return Err(CustodyError::Aead);
        }
        let mut k = Zeroizing::new([0u8; KEY_LEN]);
        k.copy_from_slice(&pt);
        // Wipe the heap pt buffer.
        let mut pt_z = Zeroizing::new(pt);
        pt_z.zeroize();
        Ok(k)
    }
}

// ─────────────────────────────────────────────────────────────────────
// 6. TpmCustodian — scaffold
// ─────────────────────────────────────────────────────────────────────
//
// The full TPM 2.0 backend (TBS on Windows, /dev/tpmrm0 on Linux,
// SystemKeyDirectory on macOS via Secure Enclave for the
// SecureEnclave variant) lands in a follow-up PR. This crate's job
// at v0.1 is to expose the right *capability shape* so downstream
// code (`select_custodian`) can already make TPM-first decisions.
// Wrap / unwrap return Unimplemented{ backend: "TPM 2.0" }.

/// Scaffold custodian for TPM 2.0 (and analogous hardware-bound
/// keystores). Reports `CustodianCaps::HARDWARE_LOCAL` so
/// [`select_custodian`] correctly prefers it over a passphrase
/// custodian; returns [`CustodyError::Unimplemented`] from `wrap` /
/// `unwrap` until the platform backend lands in a follow-up PR.
pub struct TpmCustodian {
    /// What the platform reports — used by `name()` and capability
    /// reporting in tests so we can simulate "TPM detected" without
    /// actually talking to silicon.
    detected_label: String,
}

impl TpmCustodian {
    /// Build a TPM custodian that pretends to have detected a
    /// device of the given label (e.g. `"tpm-2.0"`,
    /// `"secure-enclave"`).
    pub fn detected(label: impl Into<String>) -> Self {
        TpmCustodian { detected_label: label.into() }
    }
}

impl KeyCustodian for TpmCustodian {
    fn name(&self) -> &str {
        &self.detected_label
    }
    fn capabilities(&self) -> CustodianCaps {
        CustodianCaps::HARDWARE_LOCAL
    }
    fn wrap(&self, _label: &Label, _key: &Key) -> Result<WrappedKey, CustodyError> {
        Err(CustodyError::Unimplemented { backend: "TPM 2.0 / Secure Enclave" })
    }
    fn unwrap(&self, _label: &Label, _wrapped: &WrappedKey) -> Result<Key, CustodyError> {
        Err(CustodyError::Unimplemented { backend: "TPM 2.0 / Secure Enclave" })
    }
}

// ─────────────────────────────────────────────────────────────────────
// 7. select_custodian — the TPM-first policy helper
// ─────────────────────────────────────────────────────────────────────

/// Given a list of candidate custodians and a `force_software`
/// flag, pick the preferred custodian per the TPM-first / hardware-
/// preferred policy:
///
/// * If any candidate reports `hardware_bound = true`, it wins
///   (first such candidate in input order).
/// * Otherwise the first candidate wins.
/// * If `force_software` is `false` and at least one candidate is
///   hardware-bound but the caller passed only software ones,
///   that's a configuration error — return
///   `HardwareAvailableButSoftwareRequested`. (This is the
///   "refusal-to-fall-back" guarantee.)
///
/// Note: the "hardware available but software requested" detection
/// is delegated to the caller — they pass the full candidate list
/// and the function picks. If the caller wants to *exclude*
/// hardware (test / no-TPM path), they pass `force_software = true`
/// and a software-only candidate list, and we return that.
pub fn select_custodian<'a>(
    candidates: &'a [&'a dyn KeyCustodian],
    force_software: bool,
) -> Result<&'a dyn KeyCustodian, CustodyError> {
    if candidates.is_empty() {
        return Err(CustodyError::NoCandidates);
    }
    let any_hardware = candidates.iter().any(|c| c.capabilities().hardware_bound);

    if any_hardware {
        // Hardware available — pick the first hardware candidate
        // unless force_software is set, in which case the caller
        // is intentionally bypassing it (test / migration path).
        if force_software {
            // Pick the first software candidate, if any. If the
            // candidate list is hardware-only, FAIL CLOSED rather
            // than silently returning a hardware custodian — the
            // caller asked for software and giving them hardware
            // would be a policy/identity confusion.
            for c in candidates {
                if !c.capabilities().hardware_bound {
                    return Ok(*c);
                }
            }
            return Err(CustodyError::NoSoftwareCandidate);
        }
        for c in candidates {
            if c.capabilities().hardware_bound {
                return Ok(*c);
            }
        }
        // Unreachable given any_hardware == true, but fail closed
        // anyway rather than picking candidates[0].
        Err(CustodyError::HardwareAvailableButSoftwareRequested)
    } else {
        // No hardware available — software is the only option.
        Ok(candidates[0])
    }
}

/// Detect a "deceptive" configuration where the caller built a
/// software-only candidate list while the host machine probably has
/// a hardware custodian available. This is an *advisory* check
/// callers can run during boot: it returns
/// `HardwareAvailableButSoftwareRequested` when the candidate list
/// is software-only **and** the caller asserts (via `host_has_hw`)
/// that the host actually does have hardware. Use the OS / vendor-
/// specific detection your application has on hand to fill in
/// `host_has_hw`.
pub fn assert_no_hardware_bypass(
    candidates: &[&dyn KeyCustodian],
    host_has_hw: bool,
    force_software: bool,
) -> Result<(), CustodyError> {
    if !host_has_hw || force_software {
        return Ok(());
    }
    let any_hardware = candidates.iter().any(|c| c.capabilities().hardware_bound);
    if !any_hardware {
        return Err(CustodyError::HardwareAvailableButSoftwareRequested);
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────
// 8. Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_ct_compare::ct_eq;

    fn fast_passphrase(pw: &[u8]) -> PassphraseCustodian {
        // Use very low Argon2id parameters in tests; production
        // callers go through `with_default_params`.
        PassphraseCustodian::with_params(pw.to_vec(), 1, 8, 1).unwrap()
    }

    fn random_key() -> Key {
        fresh_random_key().unwrap()
    }

    // --- PassphraseCustodian round-trip ---

    #[test]
    fn passphrase_wrap_unwrap_roundtrip() {
        let cust = fast_passphrase(b"correct horse battery staple");
        let key = random_key();
        let label: Label = b"vault/master".to_vec();
        let wrapped = cust.wrap(&label, &key).unwrap();
        let unwrapped = cust.unwrap(&label, &wrapped).unwrap();
        assert!(ct_eq(&*key, &*unwrapped));
    }

    #[test]
    fn wrap_produces_distinct_blobs_each_time() {
        // Fresh random salt + nonce per wrap -> different blobs even
        // for the same key + label + passphrase.
        let cust = fast_passphrase(b"pw");
        let key = random_key();
        let label: Label = b"x".to_vec();
        let a = cust.wrap(&label, &key).unwrap();
        let b = cust.wrap(&label, &key).unwrap();
        assert_ne!(a.0, b.0);
        // Both unwrap to the same key.
        let ka = cust.unwrap(&label, &a).unwrap();
        let kb = cust.unwrap(&label, &b).unwrap();
        assert!(ct_eq(&*ka, &*kb));
    }

    // --- Wrong-passphrase rejection ---

    #[test]
    fn wrong_passphrase_fails_closed() {
        let good = fast_passphrase(b"good");
        let bad = fast_passphrase(b"bad");
        let key = random_key();
        let label: Label = b"slot1".to_vec();
        let wrapped = good.wrap(&label, &key).unwrap();
        match bad.unwrap(&label, &wrapped) {
            Err(CustodyError::InvalidPassphrase) => {}
            Err(other) => panic!("expected InvalidPassphrase, got {:?}", other),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    // --- Wrong-label rejection (AAD binding) ---

    #[test]
    fn wrong_label_fails_closed() {
        let cust = fast_passphrase(b"pw");
        let key = random_key();
        let label_a: Label = b"slot-a".to_vec();
        let label_b: Label = b"slot-b".to_vec();
        let wrapped = cust.wrap(&label_a, &key).unwrap();
        match cust.unwrap(&label_b, &wrapped) {
            Err(CustodyError::InvalidPassphrase) => {}
            Err(other) => panic!("expected InvalidPassphrase, got {:?}", other),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    // --- Tampering rejection ---

    #[test]
    fn body_tamper_fails_closed() {
        let cust = fast_passphrase(b"pw");
        let key = random_key();
        let label: Label = b"x".to_vec();
        let mut wrapped = cust.wrap(&label, &key).unwrap();
        // Flip a byte somewhere inside the ciphertext field. The
        // ct field starts at offset magic+salt+nonce = 2+16+24 = 42.
        wrapped.0[45] ^= 0x01;
        match cust.unwrap(&label, &wrapped) {
            Err(CustodyError::InvalidPassphrase) => {}
            Err(other) => panic!("expected InvalidPassphrase, got {:?}", other),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[test]
    fn magic_tamper_is_malformed() {
        let cust = fast_passphrase(b"pw");
        let key = random_key();
        let label: Label = b"x".to_vec();
        let mut wrapped = cust.wrap(&label, &key).unwrap();
        wrapped.0[0] ^= 0xFF;
        match cust.unwrap(&label, &wrapped) {
            Err(CustodyError::MalformedWrappedKey) => {}
            Err(other) => panic!("expected MalformedWrappedKey, got {:?}", other),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[test]
    fn truncated_blob_is_malformed() {
        let cust = fast_passphrase(b"pw");
        let key = random_key();
        let label: Label = b"x".to_vec();
        let mut wrapped = cust.wrap(&label, &key).unwrap();
        wrapped.0.truncate(20);
        match cust.unwrap(&label, &wrapped) {
            Err(CustodyError::MalformedWrappedKey) => {}
            Err(other) => panic!("expected MalformedWrappedKey, got {:?}", other),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    // --- Parameter validation ---

    #[test]
    fn empty_passphrase_rejected() {
        match PassphraseCustodian::with_default_params(Vec::new()) {
            Err(CustodyError::InvalidParameter { .. }) => {}
            Err(other) => panic!("expected InvalidParameter, got {:?}", other),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[test]
    fn zero_time_cost_rejected() {
        match PassphraseCustodian::with_params(b"pw".to_vec(), 0, 64, 1) {
            Err(CustodyError::InvalidParameter { .. }) => {}
            Err(other) => panic!("expected InvalidParameter, got {:?}", other),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    // --- Capabilities ---

    #[test]
    fn passphrase_reports_software_caps() {
        let cust = fast_passphrase(b"x");
        let caps = cust.capabilities();
        assert!(!caps.hardware_bound);
        assert!(caps.extractable);
        assert!(!caps.requires_user_presence);
        assert!(!caps.remote);
    }

    #[test]
    fn tpm_reports_hardware_caps() {
        let cust = TpmCustodian::detected("tpm-2.0");
        let caps = cust.capabilities();
        assert!(caps.hardware_bound);
        assert!(!caps.extractable);
    }

    #[test]
    fn tpm_wrap_returns_unimplemented() {
        let cust = TpmCustodian::detected("tpm-2.0");
        let key = random_key();
        match cust.wrap(&b"label".to_vec(), &key) {
            Err(CustodyError::Unimplemented { .. }) => {}
            Err(other) => panic!("expected Unimplemented, got {:?}", other),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    // --- TPM-first / select_custodian policy ---

    #[test]
    fn select_picks_hardware_when_available() {
        let pw = fast_passphrase(b"pw");
        let tpm = TpmCustodian::detected("tpm-2.0");
        let candidates: Vec<&dyn KeyCustodian> = vec![&pw, &tpm];
        let chosen = select_custodian(&candidates, false).unwrap();
        // The first hardware candidate is chosen; we placed the
        // TPM second in the list to verify the selector finds it.
        assert!(chosen.capabilities().hardware_bound);
        assert_eq!(chosen.name(), "tpm-2.0");
    }

    #[test]
    fn select_picks_software_when_only_software() {
        let pw = fast_passphrase(b"pw");
        let candidates: Vec<&dyn KeyCustodian> = vec![&pw];
        let chosen = select_custodian(&candidates, false).unwrap();
        assert!(!chosen.capabilities().hardware_bound);
    }

    #[test]
    fn select_with_force_software_picks_software_when_both_present() {
        let pw = fast_passphrase(b"pw");
        let tpm = TpmCustodian::detected("tpm-2.0");
        let candidates: Vec<&dyn KeyCustodian> = vec![&pw, &tpm];
        let chosen = select_custodian(&candidates, true).unwrap();
        assert!(!chosen.capabilities().hardware_bound);
        assert_eq!(chosen.name(), "passphrase");
    }

    #[test]
    fn select_with_force_software_on_hardware_only_list_fails_closed() {
        // Caller asks for software but only hardware is available.
        // Must NOT silently return hardware — must error out.
        let tpm = TpmCustodian::detected("tpm-2.0");
        let candidates: Vec<&dyn KeyCustodian> = vec![&tpm];
        match select_custodian(&candidates, /* force_software = */ true) {
            Err(CustodyError::NoSoftwareCandidate) => {}
            Err(other) => panic!("expected NoSoftwareCandidate, got {:?}", other),
            Ok(_) => panic!("expected error, got Ok (would have returned hardware silently)"),
        }
    }

    #[test]
    fn select_rejects_empty_candidates() {
        match select_custodian(&[], false) {
            Err(CustodyError::NoCandidates) => {}
            Err(other) => panic!("expected NoCandidates, got {:?}", other),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[test]
    fn assert_no_hardware_bypass_refuses_software_only_when_host_has_hw() {
        let pw = fast_passphrase(b"pw");
        let candidates: Vec<&dyn KeyCustodian> = vec![&pw];
        match assert_no_hardware_bypass(&candidates, /* host_has_hw = */ true, false) {
            Err(CustodyError::HardwareAvailableButSoftwareRequested) => {}
            other => panic!("expected HardwareAvailableButSoftwareRequested, got {:?}", other),
        }
    }

    #[test]
    fn assert_no_hardware_bypass_allows_software_only_when_no_hw() {
        let pw = fast_passphrase(b"pw");
        let candidates: Vec<&dyn KeyCustodian> = vec![&pw];
        assert!(assert_no_hardware_bypass(&candidates, false, false).is_ok());
    }

    #[test]
    fn assert_no_hardware_bypass_allows_software_only_with_force() {
        let pw = fast_passphrase(b"pw");
        let candidates: Vec<&dyn KeyCustodian> = vec![&pw];
        assert!(assert_no_hardware_bypass(&candidates, true, /* force = */ true).is_ok());
    }

    #[test]
    fn assert_no_hardware_bypass_allows_when_hardware_in_list() {
        let pw = fast_passphrase(b"pw");
        let tpm = TpmCustodian::detected("tpm-2.0");
        let candidates: Vec<&dyn KeyCustodian> = vec![&pw, &tpm];
        assert!(assert_no_hardware_bypass(&candidates, true, false).is_ok());
    }

    // --- Errors come from literals ---

    #[test]
    fn error_messages_are_static_literals() {
        let errs: Vec<CustodyError> = vec![
            CustodyError::InvalidPassphrase,
            CustodyError::MalformedWrappedKey,
            CustodyError::InvalidParameter { what: "x" },
            CustodyError::Csprng,
            CustodyError::Kdf,
            CustodyError::Aead,
            CustodyError::HardwareAvailableButSoftwareRequested,
            CustodyError::NoCandidates,
            CustodyError::NoSoftwareCandidate,
            CustodyError::Unimplemented { backend: "TPM 2.0" },
        ];
        for e in &errs {
            let s = e.to_string();
            assert!(s.starts_with("vault-key-custody:"));
        }
    }

    // --- Wrap output length is deterministic ---

    #[test]
    fn wrap_output_has_expected_length() {
        let cust = fast_passphrase(b"pw");
        let key = random_key();
        let wrapped = cust.wrap(&b"l".to_vec(), &key).unwrap();
        // 2 + 16 + 24 + 32 + 16 = 90.
        assert_eq!(wrapped.0.len(), 90);
    }

    // --- Drop wipes passphrase: smoke test ---
    //
    // We can't observe the heap directly post-drop in safe Rust
    // (and we don't want unsafe in this crate), but we can verify
    // that the custodian's Drop is wired to run zeroize by using a
    // wrapper type that exposes the passphrase via reference and
    // confirming it's not visible after drop. Since the actual
    // wiping is in Zeroizing<Vec<u8>> which is well-tested upstream,
    // we settle for a smoke test that the type compiles with Drop
    // and that wrap+drop runs without panicking.

    #[test]
    fn custodian_drop_is_safe() {
        let cust = fast_passphrase(b"top secret passphrase");
        let key = random_key();
        let _ = cust.wrap(&b"x".to_vec(), &key).unwrap();
        // Explicit drop — exercises the Drop impl.
        drop(cust);
    }
}
