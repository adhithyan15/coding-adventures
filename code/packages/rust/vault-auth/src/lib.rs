//! # coding_adventures_vault_auth — VLT05
//!
//! ## What this crate does
//!
//! The pluggable **authentication** layer of the Vault stack.
//! Defines an `Authenticator` trait with implementations for the
//! factors needed across the reference targets:
//!
//! - **End-user password manager** wants password + TOTP +
//!   WebAuthn + passkeys + recovery key.
//! - **Machine-secrets store** wants AppRole + OIDC/JWT + IAM
//!   signed-request + Kubernetes service-account + mTLS.
//!
//! Both reduce to "verify a credential, emit an authenticated
//! session." VLT05 is the trait host; this PR ships two factors
//! (password + TOTP) and the trait machinery so apps can compose
//! more in.
//!
//! ## Two operating modes per factor
//!
//! Every factor has a `mode()` of either:
//!
//! - **`Mode::Gate`** — pass / fail, no key material contributed.
//!   Used by 2FA-style factors that prove possession but don't
//!   widen the unlock-key derivation set: TOTP, WebAuthn (when not
//!   in PRF mode), SMS-OTP, Duo push.
//! - **`Mode::Bind`** — contributes key material to the unlock
//!   derivation. Used by primary factors and bind-mode hardware:
//!   `Password` (the `key_contribution` is the Argon2id-derived
//!   tag), 1Password's "Secret Key", FIDO2-PRF, Shamir shares.
//!
//! Higher layers combine bind-mode contributions through
//! `combine_key_contributions(...)` — an HKDF-extract over the
//! ordered concatenation of contributions, with the vault-id as
//! the salt and a fixed `info` so the derivation is deterministic
//! given the same factor set.
//!
//! ## What's in this crate (v0.1)
//!
//! - `Authenticator` trait + `Mode` + `AuthAssertion`.
//! - `PasswordAuthenticator` (bind-mode) — Argon2id-derived tag is
//!   the key contribution; verify() compares constant-time against
//!   a stored Argon2id-derived verifier.
//! - `TotpAuthenticator` (gate-mode) — RFC 6238 (HOTP under
//!   HMAC-SHA-1/SHA-256/SHA-512, time-based counter), 6 digits,
//!   30-second period default. Verify accepts the current step ± 1
//!   by default; replay-rejection cache is the caller's
//!   responsibility (we provide `verify_at_time` so upper layers can
//!   pin the accepted step into a per-secret last-used record).
//!   `code_at`/`formatted_code_at` also make it usable as a
//!   *generator*, which is what a password manager displaying a
//!   stored seed's current code needs.
//! - `WebAuthnPrfAuthenticator` (bind-mode), per
//!   `code/specs/VLT-PM51-hardware-security-keys.md`. Wires the
//!   registration-time shape (relying-party id, credential id,
//!   stored COSE public key) and, as of VLT-PM51's second slice, a
//!   real `Ctap2Transport` boundary: `verify()` performs a live
//!   CTAP2 `GetAssertion` with the `hmac-secret` extension against
//!   whatever transport it was built with, checks everything about
//!   the response that doesn't require an ECDSA P-256 signature
//!   verifier (rpId hash, credential id, user-presence flag,
//!   `hmac-secret` output present), and only then still refuses —
//!   via `AuthError::Unimplemented` — because that one remaining
//!   primitive doesn't exist anywhere in this workspace yet. See
//!   VLT-PM51 §6/§7 (slice 1) and its hardware-transport section
//!   (slice 2) for the full reasoning.
//! - `combine_key_contributions(vault_id, factors)` —
//!   HKDF-Extract(salt = vault_id, ikm = ordered concat of bind-
//!   mode factor outputs, info = "VLT05/key/v1") → 32-byte unlock
//!   key.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_argon2id::{argon2id, Options as ArgonOptions};
use coding_adventures_ct_compare::ct_eq;
use coding_adventures_hkdf::{hkdf, HashAlgorithm};
use coding_adventures_hmac::{hmac_sha1, hmac_sha256, hmac_sha512};
use coding_adventures_sha256::sha256;
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use std::time::Duration;

// ─────────────────────────────────────────────────────────────────────
// 1. Trait + supporting types
// ─────────────────────────────────────────────────────────────────────

/// Operating mode of an authenticator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Gate mode — pass/fail, no key material contributed.
    Gate,
    /// Bind mode — contributes key material to the unlock
    /// derivation.
    Bind,
}

/// Successful authentication assertion.
///
/// `key_contribution` is `Some(bytes)` only when `mode == Bind`.
pub struct AuthAssertion {
    /// The factor's `kind()` string, copied for logging.
    pub kind: &'static str,
    /// Mode the factor was operating in.
    pub mode: Mode,
    /// Key material contributed by this factor — only present in
    /// bind mode. Wrapped in `Zeroizing` so it wipes on drop.
    pub key_contribution: Option<Zeroizing<Vec<u8>>>,
}

impl AuthAssertion {
    /// Return a credential-safe summary of this assertion.
    ///
    /// The summary reports only factor identity, mode, and whether bind-mode
    /// key material exists. It never exposes the key-contribution bytes.
    pub fn summary(&self) -> AuthAssertionSummary {
        AuthAssertionSummary {
            kind: self.kind,
            mode: self.mode,
            has_key_contribution: self.key_contribution.is_some(),
            key_contribution_len: self
                .key_contribution
                .as_ref()
                .map_or(0, |contribution| contribution.len()),
        }
    }
}

impl Drop for AuthAssertion {
    fn drop(&mut self) {
        if let Some(k) = self.key_contribution.as_mut() {
            k.zeroize();
        }
    }
}

/// Credential-safe read model for a successful authentication assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthAssertionSummary {
    /// Stable factor kind, such as `"password"` or `"totp"`.
    pub kind: &'static str,
    /// Whether the factor was gate-only or contributed bind material.
    pub mode: Mode,
    /// Whether bind-mode key material is present.
    pub has_key_contribution: bool,
    /// Length of the key contribution, if present.
    pub key_contribution_len: usize,
}

impl AuthAssertionSummary {
    /// Return true when this summary represents a bind-mode factor with key material.
    pub fn contributes_key_material(&self) -> bool {
        self.mode == Mode::Bind && self.has_key_contribution && self.key_contribution_len > 0
    }
}

/// Aggregate read model for a set of successful authentication assertions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuthAssertionSetSummary {
    /// Number of assertions in the set.
    pub assertion_count: usize,
    /// Number of gate-mode factors.
    pub gate_count: usize,
    /// Number of bind-mode factors.
    pub bind_count: usize,
    /// Number of bind-mode factors that carry key contribution bytes.
    pub key_contribution_count: usize,
    /// Total key-contribution byte length across bind-mode factors.
    pub total_key_contribution_len: usize,
    /// Number of assertions from the built-in password factor.
    pub password_count: usize,
    /// Number of assertions from the built-in TOTP factor.
    pub totp_count: usize,
    /// Number of assertions from extension factors.
    pub extension_count: usize,
    /// Bind-mode factors that did not carry key contribution bytes.
    pub missing_bind_contribution_count: usize,
    /// Gate-mode factors that unexpectedly carried key contribution bytes.
    pub unexpected_gate_contribution_count: usize,
}

impl AuthAssertionSetSummary {
    /// Return true if the assertion set has at least one bind-mode contribution.
    pub fn can_derive_unlock_key(&self) -> bool {
        self.key_contribution_count > 0 && self.total_key_contribution_len > 0
    }

    /// Return true if the assertion set contains both bind and gate factors.
    pub fn is_multi_factor(&self) -> bool {
        self.bind_count > 0 && self.gate_count > 0
    }

    /// Return true if extension factors beyond password/TOTP participated.
    pub fn has_extension_factors(&self) -> bool {
        self.extension_count > 0
    }

    /// Return true if factors obey gate/bind contribution invariants.
    pub fn is_contribution_consistent(&self) -> bool {
        self.missing_bind_contribution_count == 0 && self.unexpected_gate_contribution_count == 0
    }
}

/// Errors from any [`Authenticator`].
///
/// `Display` strings are sourced exclusively from this crate's
/// literals — never from input.
#[derive(Debug)]
pub enum AuthError {
    /// Wrong password / wrong TOTP code / wrong identity. Always
    /// fail-closed; we never reveal *which* condition failed.
    InvalidCredential,
    /// The credential is structurally malformed (e.g. TOTP code
    /// not exactly N digits).
    MalformedCredential,
    /// Constructor parameter validation failed.
    InvalidParameter {
        /// Static description of the bad parameter.
        what: &'static str,
    },
    /// Underlying KDF / HMAC / HKDF / random failure.
    Crypto,
    /// `combine_key_contributions` got an empty factor list.
    NoBindFactors,
    /// A cryptographic primitive this backend needs doesn't exist in
    /// this workspace yet. Returned by [`WebAuthnPrfAuthenticator`]
    /// once a live CTAP2 assertion has already been obtained and
    /// checked in every way that doesn't require the missing
    /// primitive — see the module comment above
    /// [`WebAuthnPrfAuthenticator`] (`vault-key-custody::TpmCustodian`
    /// uses the identical pattern for a hardware-bound key custodian).
    Unimplemented {
        /// Static name of the backend that needs implementing.
        backend: &'static str,
    },
    /// No CTAP2 authenticator answered — none is plugged in, or more
    /// than one is and the transport can't disambiguate. Detected by
    /// cheap device enumeration, so a vault with no hardware key
    /// configured never blocks behind this on its normal unlock path.
    HardwareUnavailable,
    /// A CTAP2 authenticator was reached but did not confirm a
    /// physical touch (user presence) within the configured timeout.
    /// CTAP2-over-HID has no signal that reliably distinguishes an
    /// authenticator that was touched-and-declined from one that was
    /// simply never touched, so both surface here.
    HardwareTimeout,
    /// The transport reached a device but the CTAP2 exchange itself
    /// failed for a reason that is neither "no device" nor "no
    /// touch" — a HID I/O error, a malformed or unexpected CTAP2
    /// response, or a protocol-level refusal. `detail` is always a
    /// static classification, never bytes read from the device.
    HardwareTransport {
        /// Static classification of the transport failure.
        detail: &'static str,
    },
}

impl core::fmt::Display for AuthError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            AuthError::InvalidCredential => "vault-auth: invalid credential",
            AuthError::MalformedCredential => "vault-auth: malformed credential",
            AuthError::InvalidParameter { what } => {
                return write!(f, "vault-auth: invalid parameter: {}", what);
            }
            AuthError::Crypto => "vault-auth: underlying cryptographic operation failed",
            AuthError::NoBindFactors => {
                "vault-auth: combine_key_contributions called with no bind-mode factors"
            }
            AuthError::Unimplemented { backend } => {
                return write!(
                    f,
                    "vault-auth: {} backend not yet implemented in this build",
                    backend
                );
            }
            AuthError::HardwareUnavailable => {
                "vault-auth: no hardware security key responded (none attached, or several are and the transport can't tell them apart)"
            }
            AuthError::HardwareTimeout => {
                "vault-auth: hardware security key did not confirm a touch within the timeout"
            }
            AuthError::HardwareTransport { detail } => {
                return write!(f, "vault-auth: hardware security key transport failed: {}", detail);
            }
        };
        write!(f, "{}", s)
    }
}

impl std::error::Error for AuthError {}

/// Pluggable authentication factor. Implementations: this crate
/// ships `PasswordAuthenticator` (bind) and `TotpAuthenticator`
/// (gate); follow-up PRs add WebAuthn / FIDO2-PRF / OPAQUE / OIDC
/// / mTLS / SMS / Duo / AppRole / AWS-STS / Kubernetes-SA / etc.
pub trait Authenticator {
    /// Stable string identifying the factor kind, e.g.
    /// `"password"`, `"totp"`, `"webauthn-prf"`.
    fn kind(&self) -> &'static str;

    /// Bind / gate.
    fn mode(&self) -> Mode;

    /// Verify the supplied `credential` against the factor's
    /// stored verifier. The credential is opaque bytes — for
    /// password it's the password text, for TOTP it's the 6-digit
    /// code as ASCII, for WebAuthn it's the assertion CBOR, etc.
    ///
    /// Returns an `AuthAssertion` on success. The shape of the
    /// assertion's `key_contribution` is factor-specific; bind-
    /// mode factors fill it, gate-mode leave it `None`.
    fn verify(&self, credential: &[u8]) -> Result<AuthAssertion, AuthError>;
}

/// Combine the key-material from bind-mode factors into a single
/// 32-byte unlock key.
///
/// `vault_id` is the per-vault salt — distinct vaults derive
/// distinct unlock keys from the same credential set. `factors`
/// is the ordered list of `AuthAssertion`s; only `Mode::Bind`
/// entries contribute. Empty list → [`AuthError::NoBindFactors`].
///
/// Returns `Zeroizing<[u8; 32]>` so the unlock key wipes on drop.
pub fn combine_key_contributions(
    vault_id: &[u8],
    factors: &[&AuthAssertion],
) -> Result<Zeroizing<[u8; 32]>, AuthError> {
    let mut ikm = Zeroizing::new(Vec::<u8>::new());
    for f in factors {
        if f.mode == Mode::Bind {
            if let Some(k) = f.key_contribution.as_ref() {
                ikm.extend_from_slice(k);
            }
        }
    }
    if ikm.is_empty() {
        return Err(AuthError::NoBindFactors);
    }
    let okm = hkdf(vault_id, &ikm, b"VLT05/key/v1", 32, HashAlgorithm::Sha256)
        .map_err(|_| AuthError::Crypto)?;
    if okm.len() != 32 {
        return Err(AuthError::Crypto);
    }
    let mut out = Zeroizing::new([0u8; 32]);
    out.copy_from_slice(&okm);
    let mut okm_z = Zeroizing::new(okm);
    okm_z.zeroize();
    Ok(out)
}

/// Summarize successful authentication assertions without exposing credentials.
pub fn summarize_auth_assertions(factors: &[&AuthAssertion]) -> AuthAssertionSetSummary {
    let mut summary = AuthAssertionSetSummary {
        assertion_count: factors.len(),
        ..AuthAssertionSetSummary::default()
    };
    for assertion in factors {
        let assertion_summary = assertion.summary();
        match assertion_summary.mode {
            Mode::Gate => {
                summary.gate_count += 1;
                if assertion_summary.has_key_contribution {
                    summary.unexpected_gate_contribution_count += 1;
                }
            }
            Mode::Bind => {
                summary.bind_count += 1;
                if !assertion_summary.has_key_contribution {
                    summary.missing_bind_contribution_count += 1;
                }
            }
        }
        match assertion_summary.kind {
            "password" => summary.password_count += 1,
            "totp" => summary.totp_count += 1,
            _ => summary.extension_count += 1,
        }
        if assertion_summary.contributes_key_material() {
            summary.key_contribution_count += 1;
            summary.total_key_contribution_len += assertion_summary.key_contribution_len;
        }
    }
    summary
}

// ─────────────────────────────────────────────────────────────────────
// 2. PasswordAuthenticator
// ─────────────────────────────────────────────────────────────────────

/// Argon2id-backed password authenticator. Bind-mode: the derived
/// 32-byte tag is the `key_contribution` for the unlock derivation.
///
/// Construct with `with_verifier(...)` — i.e. you already
/// computed the Argon2id verifier at registration time and stored
/// `(salt, params, verifier)`. `verify(password)` re-derives and
/// constant-time-compares.
pub struct PasswordAuthenticator {
    salt: Vec<u8>,
    time_cost: u32,
    memory_cost: u32,
    parallelism: u32,
    /// Stored Argon2id output (the verifier).
    verifier: Vec<u8>,
}

impl PasswordAuthenticator {
    /// Build with the four pieces persisted at registration time.
    pub fn with_verifier(
        salt: Vec<u8>,
        time_cost: u32,
        memory_cost: u32,
        parallelism: u32,
        verifier: Vec<u8>,
    ) -> Result<Self, AuthError> {
        if salt.len() < 8 {
            return Err(AuthError::InvalidParameter {
                what: "salt < 8 bytes",
            });
        }
        if verifier.is_empty() {
            return Err(AuthError::InvalidParameter {
                what: "verifier empty",
            });
        }
        if time_cost == 0 || memory_cost < 8 || parallelism == 0 {
            return Err(AuthError::InvalidParameter {
                what: "Argon2id parameters too small",
            });
        }
        Ok(Self {
            salt,
            time_cost,
            memory_cost,
            parallelism,
            verifier,
        })
    }

    /// Helper: derive a verifier at registration time. Caller
    /// stores `(salt, params, verifier)` and later passes them to
    /// `with_verifier`.
    pub fn derive_verifier(
        password: &[u8],
        salt: &[u8],
        time_cost: u32,
        memory_cost: u32,
        parallelism: u32,
        tag_length: u32,
    ) -> Result<Vec<u8>, AuthError> {
        let opts = ArgonOptions {
            key: None,
            associated_data: None,
            version: None,
        };
        argon2id(
            password,
            salt,
            time_cost,
            memory_cost,
            parallelism,
            tag_length,
            &opts,
        )
        .map_err(|_| AuthError::Crypto)
    }
}

impl Authenticator for PasswordAuthenticator {
    fn kind(&self) -> &'static str {
        "password"
    }
    fn mode(&self) -> Mode {
        Mode::Bind
    }
    fn verify(&self, credential: &[u8]) -> Result<AuthAssertion, AuthError> {
        if credential.is_empty() {
            return Err(AuthError::MalformedCredential);
        }
        let opts = ArgonOptions {
            key: None,
            associated_data: None,
            version: None,
        };
        let candidate = argon2id(
            credential,
            &self.salt,
            self.time_cost,
            self.memory_cost,
            self.parallelism,
            self.verifier.len() as u32,
            &opts,
        )
        .map_err(|_| AuthError::Crypto)?;
        if !ct_eq(&candidate, &self.verifier) {
            // Wipe the candidate before returning — even on failure.
            let mut c = Zeroizing::new(candidate);
            c.zeroize();
            return Err(AuthError::InvalidCredential);
        }
        // Success: the tag IS our key contribution. Move it into
        // the assertion wrapped in Zeroizing.
        let mut k = Zeroizing::new(candidate);
        // Defensive — make sure it's non-empty.
        if k.is_empty() {
            k.zeroize();
            return Err(AuthError::Crypto);
        }
        Ok(AuthAssertion {
            kind: "password",
            mode: Mode::Bind,
            key_contribution: Some(k),
        })
    }
}

// ─────────────────────────────────────────────────────────────────────
// 3. TotpAuthenticator (RFC 6238)
// ─────────────────────────────────────────────────────────────────────
//
// HOTP per RFC 4226: code = truncate(HMAC-H(secret, counter)) mod 10^digits
// TOTP per RFC 6238: counter = floor((unix_time - T0) / period)
//
// RFC 6238 §1.2 names three HMAC variants — SHA-1, SHA-256, and
// SHA-512 — and its Appendix B publishes test vectors for all
// three. SHA-1 is what every authenticator app on the planet
// defaults to, but "default" is not "only": a stored seed carries
// its algorithm with it, so a verifier or generator that assumed
// SHA-1 would silently produce plausible, wrong digits for the
// other two. `TotpAlgorithm` therefore has no `Default` impl and
// the constructor takes it explicitly.

/// HMAC hash underlying one TOTP secret, per RFC 6238 §1.2.
///
/// There is deliberately no `Default`. Six wrong digits look exactly
/// like six right ones, so the one parameter that decides which is
/// which is never chosen on a caller's behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotpAlgorithm {
    /// HMAC-SHA-1. The RFC 6238 default and the near-universal choice.
    Sha1,
    /// HMAC-SHA-256.
    Sha256,
    /// HMAC-SHA-512.
    Sha512,
}

impl TotpAlgorithm {
    /// Compute the HMAC of `message` under `key` for this algorithm.
    ///
    /// The three hashes have different output widths (20, 32, and 64
    /// bytes), so the tag is returned as a `Vec` and the dynamic
    /// truncation below indexes it by its actual length rather than
    /// by a hard-wired 20.
    ///
    /// Each arm copies the fixed-size array into the returned
    /// `Zeroizing` buffer and then wipes the array. Writing this as
    /// `hmac_sha1(...)?.into()` would be shorter and would leave a
    /// live authentication tag on the stack: `into()` *copies* into a
    /// fresh allocation, so the `Zeroizing` wrapper would own the
    /// second copy while the first went out of scope untouched. From
    /// a TOTP tag an observer reads the code directly, so the copy
    /// that nobody wipes is the one that matters.
    fn mac(self, key: &[u8], message: &[u8]) -> Result<Zeroizing<Vec<u8>>, AuthError> {
        fn take<const N: usize>(tag: Result<[u8; N], impl Sized>) -> Result<Vec<u8>, AuthError> {
            let mut tag = tag.map_err(|_| AuthError::Crypto)?;
            let owned = tag.to_vec();
            tag.zeroize();
            Ok(owned)
        }
        let tag = match self {
            Self::Sha1 => take(hmac_sha1(key, message))?,
            Self::Sha256 => take(hmac_sha256(key, message))?,
            Self::Sha512 => take(hmac_sha512(key, message))?,
        };
        Ok(Zeroizing::new(tag))
    }
}

/// RFC 6238 TOTP authenticator. Gate-mode (no key contribution).
pub struct TotpAuthenticator {
    secret: Zeroizing<Vec<u8>>,
    /// HMAC hash the seed was provisioned under.
    algorithm: TotpAlgorithm,
    /// Time step in seconds (RFC 6238 default 30).
    period: u64,
    /// Number of digits in the code (typically 6 or 8).
    digits: u32,
    /// Number of time-step skew the verifier accepts on each side
    /// (default 1 — accept current ± 1 step).
    window: u32,
}

impl TotpAuthenticator {
    /// Build a TOTP authenticator. `digits` must be in 4..=10.
    pub fn new(
        secret: impl Into<Vec<u8>>,
        algorithm: TotpAlgorithm,
        period: u64,
        digits: u32,
        window: u32,
    ) -> Result<Self, AuthError> {
        let secret: Zeroizing<Vec<u8>> = Zeroizing::new(secret.into());
        if secret.is_empty() {
            return Err(AuthError::InvalidParameter {
                what: "TOTP secret empty",
            });
        }
        if period == 0 {
            return Err(AuthError::InvalidParameter {
                what: "TOTP period 0",
            });
        }
        if !(4..=10).contains(&digits) {
            return Err(AuthError::InvalidParameter {
                what: "TOTP digits must be 4..=10",
            });
        }
        Ok(Self {
            secret,
            algorithm,
            period,
            digits,
            window,
        })
    }

    /// Number of digits this authenticator's codes are rendered in.
    pub const fn digits(&self) -> u32 {
        self.digits
    }

    /// Compute the TOTP code at the given UNIX time (seconds).
    /// Useful for testing and replay-cache integration.
    pub fn code_at(&self, unix_time_sec: u64) -> Result<u32, AuthError> {
        let counter = unix_time_sec / self.period;
        self.code_at_counter(counter)
    }

    /// Render the TOTP code at the given UNIX time as the decimal
    /// string a person actually types, zero-padded to `digits`.
    ///
    /// Padding is not cosmetic. Roughly one code in ten has a leading
    /// zero, and `042311` and `42311` are different strings to paste;
    /// only one of them is the code. Returning the integer and
    /// letting each caller remember to pad is an invitation for one
    /// of them to forget, so the padding lives here.
    ///
    /// The result is wipe-on-drop because it is a live credential.
    pub fn formatted_code_at(&self, unix_time_sec: u64) -> Result<Zeroizing<String>, AuthError> {
        let code = self.code_at(unix_time_sec)?;
        Ok(Zeroizing::new(format!(
            "{code:0width$}",
            width = self.digits as usize
        )))
    }

    /// Seconds until the step containing `unix_time_sec` ends.
    ///
    /// Always in `1..=period`: at the first second of a step the
    /// answer is the whole period, and it is never `0`, because a
    /// code with zero seconds left has already been replaced by the
    /// next one — "0 seconds remaining" would describe a code this
    /// function's caller was never given.
    pub const fn remaining_seconds(&self, unix_time_sec: u64) -> u64 {
        self.period - (unix_time_sec % self.period)
    }

    fn code_at_counter(&self, counter: u64) -> Result<u32, AuthError> {
        let counter_be = counter.to_be_bytes();
        let mac = self.algorithm.mac(&self.secret, &counter_be)?;
        // Dynamic truncation — RFC 4226 §5.3. The offset comes from
        // the low nibble of the *last* byte, which is byte 19 for
        // SHA-1 and 31/63 for the wider hashes; RFC 6238's reference
        // implementation indexes from the end for exactly this
        // reason, so the four bytes it selects always exist.
        //
        // Both lookups are checked rather than indexed. A nibble is
        // at most 15, and the narrowest tag here is 20 bytes, so a
        // panic is unreachable today — but "unreachable" rests on a
        // fact about three hash functions declared in another crate,
        // and this function cannot see it. A `Crypto` error costs
        // nothing and turns a future 4-byte digest from a panic in a
        // password manager into a refusal.
        let last = *mac.last().ok_or(AuthError::Crypto)?;
        let offset = (last & 0x0F) as usize;
        let window: [u8; 4] = mac
            .get(offset..offset + 4)
            .ok_or(AuthError::Crypto)?
            .try_into()
            .map_err(|_| AuthError::Crypto)?;
        let bin = ((window[0] as u32 & 0x7F) << 24)
            | ((window[1] as u32) << 16)
            | ((window[2] as u32) << 8)
            | (window[3] as u32);
        // The modulus is computed in u64, not u32. `digits` is
        // permitted up to 10 and 10^10 is 10_000_000_000, which
        // exceeds u32::MAX — `10u32.pow(10)` panics in a debug build
        // and wraps in a release one. `bin` is only 31 bits, so for
        // digits 10 the modulus is a no-op, which is the correct
        // answer and now also a reachable one.
        let modulus = 10u64.pow(self.digits);
        Ok((u64::from(bin) % modulus) as u32)
    }

    /// Verify a code at a specific UNIX time, applying the
    /// configured `window` (accept current ± window steps). Returns
    /// the matched step counter on success (so the caller can
    /// store-and-reject-replays at a higher layer); returns
    /// `InvalidCredential` if no step in the window matches.
    pub fn verify_at_time(&self, code: u32, unix_time_sec: u64) -> Result<u64, AuthError> {
        let center = unix_time_sec / self.period;
        let w = self.window as i64;
        for d in -w..=w {
            let counter = match (center as i64).checked_add(d) {
                Some(c) if c >= 0 => c as u64,
                _ => continue,
            };
            let cand = self.code_at_counter(counter)?;
            // Constant-time compare of the digits-as-bytes
            // representations to avoid timing leaks across
            // off-by-one matches.
            let cand_bytes = cand.to_be_bytes();
            let code_bytes = code.to_be_bytes();
            if ct_eq(&cand_bytes, &code_bytes) {
                return Ok(counter);
            }
        }
        Err(AuthError::InvalidCredential)
    }
}

impl Authenticator for TotpAuthenticator {
    fn kind(&self) -> &'static str {
        "totp"
    }
    fn mode(&self) -> Mode {
        Mode::Gate
    }
    /// Verifies a TOTP code against the *current* UNIX time
    /// (`SystemTime::now()`). Caller-supplied time is available
    /// via `verify_at_time`.
    ///
    /// `credential` is the ASCII-decimal code, e.g. `b"123456"`.
    fn verify(&self, credential: &[u8]) -> Result<AuthAssertion, AuthError> {
        let s = core::str::from_utf8(credential).map_err(|_| AuthError::MalformedCredential)?;
        if s.len() != self.digits as usize {
            return Err(AuthError::MalformedCredential);
        }
        let code: u32 = s.parse().map_err(|_| AuthError::MalformedCredential)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| AuthError::Crypto)?
            .as_secs();
        let _step = self.verify_at_time(code, now)?;
        Ok(AuthAssertion {
            kind: "totp",
            mode: Mode::Gate,
            key_contribution: None,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────
// 4. Ctap2Transport — the hardware I/O boundary (VLT-PM51 slice 2)
// ─────────────────────────────────────────────────────────────────────
//
// `WebAuthnPrfAuthenticator::verify()` needs to obtain a live CTAP2
// `GetAssertion` (with the `hmac-secret` extension) from a physical
// FIDO2 authenticator. This crate does not talk to USB HID devices
// itself — `vault-auth` is the trust-sensitive KDF/authentication
// crate every other factor in this file lives in, and giving it a
// native, hardware-touching dependency (`ctap-hid-fido2` + `hidapi`)
// would mean every consumer of `PasswordAuthenticator`/
// `TotpAuthenticator` also inherits that dependency's build and
// runtime footprint whether or not it ever plugs in a hardware key.
//
// So the boundary is a trait, not a concrete transport:
// `Ctap2Transport` is "however we talk to a physical authenticator,"
// and the real `hidapi`-backed implementation lives in the sibling
// crate `coding_adventures_vault_webauthn_ctap2_hid` — the same split
// `VLT-PM48` already uses for the local agent (a protocol crate plus
// a separate transport/host crate). Tests in this crate use a small
// in-process fake instead of real hardware.

/// A request for a live CTAP2 `GetAssertion` with the `hmac-secret`
/// extension, addressed to one specific registered credential.
pub struct Ctap2AssertionRequest<'a> {
    /// Relying-party id the credential was registered under.
    pub relying_party_id: &'a str,
    /// The credential id from registration — tells the authenticator
    /// which credential to assert with.
    pub credential_id: &'a [u8],
    /// Raw challenge bytes for this specific attempt — analogous to
    /// WebAuthn's `challenge`. The transport hashes this itself to
    /// form the assertion's signed clientDataHash-equivalent (this
    /// crate has no browser and therefore no `clientDataJSON` to
    /// build; a native CTAP2 caller signs over a hash of the
    /// challenge directly, the same simplification `ctap-hid-fido2`
    /// itself makes). Freshness here protects the eventual signature
    /// check; it deliberately does **not** feed the
    /// `hmac_secret_salt` below, which must stay fixed across
    /// attempts (see [`WebAuthnPrfAuthenticator::verify`]).
    pub challenge: [u8; 32],
    /// The `hmac-secret` salt. Fixed per registered credential so the
    /// authenticator derives the *same* secret on every unlock —
    /// `PasswordAuthenticator`'s Argon2id tag has the identical
    /// same-input-same-output property, and `combine_key_
    /// contributions`'s stability depends on it here too.
    pub hmac_secret_salt: [u8; 32],
    /// Upper bound on how long to wait for the physical touch / user
    /// presence check before giving up.
    pub touch_timeout: Duration,
}

/// A live CTAP2 `GetAssertion` response, transport-agnostic.
pub struct Ctap2AssertionResponse {
    /// SHA-256(relying_party_id) as reported inside the
    /// authenticator's signed `authData`.
    pub rpid_hash: [u8; 32],
    /// The credential id the authenticator actually asserted with.
    pub credential_id: Vec<u8>,
    /// Whether `authData`'s flags report user presence — a physical
    /// touch happened for this specific assertion.
    pub user_present: bool,
    /// Raw ECDSA signature over `authData || SHA-256(challenge)`.
    /// Kept so a future ECDSA verifier has something to check;
    /// unused until then.
    pub signature: Vec<u8>,
    /// Raw `authData`, needed alongside the request's `challenge` to
    /// reconstruct the signed message once ECDSA lands.
    pub auth_data: Vec<u8>,
    /// The `hmac-secret` extension output when the authenticator
    /// honored the extension for this credential; `None` if it
    /// doesn't support `hmac-secret` at all.
    pub hmac_secret_output: Option<Zeroizing<Vec<u8>>>,
}

/// Errors from the CTAP2 transport boundary. Every variant is a
/// static classification — never bytes read from the device — for
/// the same reason every other `AuthError` avoids echoing input.
#[derive(Debug, Clone, Copy)]
pub enum Ctap2TransportError {
    /// No CTAP2 authenticator was found, or more than one was and the
    /// transport can't disambiguate. Detected via cheap enumeration —
    /// implementations must return this fast, without ever entering
    /// a blocking wait, so the software-only unlock path is never
    /// slowed down by "is there hardware plugged in?".
    NoDeviceAvailable,
    /// A device was reached but did not confirm user presence inside
    /// the request's `touch_timeout`.
    TouchTimedOut,
    /// The exchange reached a device but failed for a reason that
    /// isn't "no device" or "no touch" — a HID I/O error, a malformed
    /// or unexpected CTAP2 response, or a protocol-level refusal.
    Failed {
        /// Static description of what went wrong.
        detail: &'static str,
    },
}

/// The narrow boundary between [`WebAuthnPrfAuthenticator`]'s
/// verification logic and physical FIDO2 hardware I/O. See the
/// section comment above for why this is a trait rather than a
/// direct `ctap-hid-fido2` dependency of this crate.
pub trait Ctap2Transport {
    /// Perform a live CTAP2 `GetAssertion` with the `hmac-secret`
    /// extension for the credential named in `request`. May block —
    /// up to `request.touch_timeout` — while waiting for the
    /// physical touch; enumeration failing outright (no device found)
    /// must return fast rather than entering that wait.
    fn get_hmac_secret_assertion(
        &self,
        request: &Ctap2AssertionRequest<'_>,
    ) -> Result<Ctap2AssertionResponse, Ctap2TransportError>;
}

// ─────────────────────────────────────────────────────────────────────
// 5. WebAuthnPrfAuthenticator (VLT-PM51)
// ─────────────────────────────────────────────────────────────────────
//
// A FIDO2 hardware security key (YubiKey and any other CTAP2-compliant
// authenticator — this is a standards-based factor, not a YubiKey-only
// one) can contribute key material to the unlock derivation through the
// CTAP2 `hmac-secret` extension, surfaced to browsers as the WebAuthn
// `prf` extension. `code/specs/VLT-PM51-hardware-security-keys.md`
// covers the full design and the reasoning behind every choice below;
// the short version:
//
// * This is a **bind-mode, additive, second factor** — exactly the same
//   shape `PasswordAuthenticator` already has, composed the same way
//   through `combine_key_contributions`. It never replaces the
//   passphrase path; VLT-PM51 §4 works through why an unlock-time
//   hardware requirement without a software fallback would violate
//   this product's existing "the CLI always works without the agent /
//   without a keychain" design language.
// * `verify()` now performs real CTAP2 hardware I/O through whatever
//   `Ctap2Transport` this instance was built with, and checks
//   everything about the response that doesn't need an ECDSA P-256
//   signature verifier: the rpId hash, the credential id, the
//   user-presence flag, and that the `hmac-secret` extension actually
//   fired. No primitive for ECDSA P-256 exists anywhere in this
//   workspace yet, so `verify()` still refuses at the very last step
//   — via `AuthError::Unimplemented` — rather than trust
//   `hmac_secret_output` as key material without being able to prove
//   the assertion was signed by the registered credential. Answering
//   "is this shaped like a valid credential, and did real hardware
//   answer for it" is a different question from "is this a valid
//   credential," and this type refuses to conflate them under the
//   same `Ok(...)` — `vault-key-custody::TpmCustodian` makes the
//   identical call for the identical reason.

/// FIDO2/WebAuthn hardware security key authenticator using the CTAP2
/// `hmac-secret` extension (WebAuthn's `prf` extension) as a bind-mode
/// unlock factor. Works with any CTAP2-compliant authenticator that
/// implements `hmac-secret` — YubiKey 5-series and many others, not a
/// vendor-specific integration.
///
/// See the module-level comment above and
/// `code/specs/VLT-PM51-hardware-security-keys.md`. `verify()`
/// performs real hardware I/O through its `Ctap2Transport` but still
/// always returns [`AuthError::Unimplemented`] as its final answer,
/// because ECDSA P-256 assertion-signature verification doesn't exist
/// in this workspace yet.
pub struct WebAuthnPrfAuthenticator {
    /// Relying-party id this credential was registered under (e.g.
    /// `"vault-pm"`). Bound into every real assertion's `authData` as
    /// `SHA-256(rpId)`; a mismatch there is what stops a credential
    /// registered for one vault from authenticating a different one.
    relying_party_id: String,
    /// Opaque credential id returned by the authenticator at
    /// registration time. Sent back to the authenticator to select
    /// which resident/non-resident credential to assert with.
    credential_id: Vec<u8>,
    /// COSE-encoded public key (`canonical-cbor`-shaped, RFC 8949
    /// §4.2.3) recorded at registration. Retained now; consumed once
    /// signature verification lands, so the wrapping type stops taking
    /// registration data it can't yet fully validate.
    public_key_cose: Vec<u8>,
    /// However this instance talks to a physical authenticator. Real
    /// callers pass a `coding_adventures_vault_webauthn_ctap2_hid`
    /// transport; tests pass a small in-process fake.
    transport: Box<dyn Ctap2Transport + Send + Sync>,
    /// Upper bound on how long `verify()` waits for a physical touch.
    touch_timeout: Duration,
}

/// Default upper bound on how long [`WebAuthnPrfAuthenticator::verify`]
/// waits for a physical touch before giving up. FIDO2 authenticators
/// commonly enforce their own internal timeout in this neighborhood
/// for a `GetAssertion` request; this doesn't invent a new number, it
/// matches that ceiling.
pub const DEFAULT_TOUCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Smallest touch timeout [`WebAuthnPrfAuthenticator::with_touch_timeout`]
/// accepts. Below one second there is no realistic window for a human
/// to react to a blinking device.
pub const MIN_TOUCH_TIMEOUT: Duration = Duration::from_secs(1);

/// Largest touch timeout [`WebAuthnPrfAuthenticator::with_touch_timeout`]
/// accepts. Above two minutes a caller almost certainly meant a
/// background job, not an interactive unlock waiting on a touch — and
/// `verify()`'s "clear, fast, non-hanging" failure mode for hardware
/// problems depends on this staying a bounded, human-scale wait.
pub const MAX_TOUCH_TIMEOUT: Duration = Duration::from_secs(120);

impl WebAuthnPrfAuthenticator {
    /// Build from the pieces a registration ceremony records
    /// (relying-party id, credential id, the credential's COSE public
    /// key) plus the transport this instance uses to reach hardware.
    /// Uses [`DEFAULT_TOUCH_TIMEOUT`]; use
    /// [`with_touch_timeout`](Self::with_touch_timeout) to override.
    pub fn new(
        relying_party_id: impl Into<String>,
        credential_id: impl Into<Vec<u8>>,
        public_key_cose: impl Into<Vec<u8>>,
        transport: impl Ctap2Transport + Send + Sync + 'static,
    ) -> Result<Self, AuthError> {
        Self::with_touch_timeout(
            relying_party_id,
            credential_id,
            public_key_cose,
            transport,
            DEFAULT_TOUCH_TIMEOUT,
        )
    }

    /// As [`new`](Self::new), with an explicit touch timeout in
    /// [`MIN_TOUCH_TIMEOUT`]..=[`MAX_TOUCH_TIMEOUT`].
    pub fn with_touch_timeout(
        relying_party_id: impl Into<String>,
        credential_id: impl Into<Vec<u8>>,
        public_key_cose: impl Into<Vec<u8>>,
        transport: impl Ctap2Transport + Send + Sync + 'static,
        touch_timeout: Duration,
    ) -> Result<Self, AuthError> {
        let relying_party_id = relying_party_id.into();
        let credential_id = credential_id.into();
        let public_key_cose = public_key_cose.into();
        if relying_party_id.is_empty() {
            return Err(AuthError::InvalidParameter {
                what: "relying_party_id is empty",
            });
        }
        if credential_id.is_empty() {
            return Err(AuthError::InvalidParameter {
                what: "credential_id is empty",
            });
        }
        if public_key_cose.is_empty() {
            return Err(AuthError::InvalidParameter {
                what: "public_key_cose is empty",
            });
        }
        if touch_timeout < MIN_TOUCH_TIMEOUT || touch_timeout > MAX_TOUCH_TIMEOUT {
            return Err(AuthError::InvalidParameter {
                what: "touch_timeout outside MIN_TOUCH_TIMEOUT..=MAX_TOUCH_TIMEOUT",
            });
        }
        Ok(Self {
            relying_party_id,
            credential_id,
            public_key_cose,
            transport: Box::new(transport),
            touch_timeout,
        })
    }

    /// The relying-party id this credential was registered under.
    pub fn relying_party_id(&self) -> &str {
        &self.relying_party_id
    }

    /// The opaque credential id recorded at registration.
    pub fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }

    /// The COSE-encoded public key recorded at registration.
    pub fn public_key_cose(&self) -> &[u8] {
        &self.public_key_cose
    }

    /// The touch timeout this instance was built with.
    pub const fn touch_timeout(&self) -> Duration {
        self.touch_timeout
    }

    /// Domain-separated `hmac-secret` salt, derived from registration-
    /// time data only (rpId + credential id) — never from the
    /// per-attempt `credential` bytes `verify()` receives. Stable
    /// across every unlock attempt for the same registered hardware
    /// key, which is what makes the eventual `key_contribution`
    /// reproducible the same way `PasswordAuthenticator`'s Argon2id
    /// tag is reproducible for the same password.
    fn hmac_secret_salt(&self) -> [u8; 32] {
        let mut buf =
            Vec::with_capacity(self.relying_party_id.len() + self.credential_id.len() + 40);
        buf.extend_from_slice(b"VLT05/webauthn-prf/hmac-secret-salt/v1");
        buf.extend_from_slice(self.relying_party_id.as_bytes());
        buf.extend_from_slice(&self.credential_id);
        sha256(&buf)
    }
}

impl Authenticator for WebAuthnPrfAuthenticator {
    fn kind(&self) -> &'static str {
        "webauthn-prf"
    }
    fn mode(&self) -> Mode {
        Mode::Bind
    }

    /// Performs a live CTAP2 `GetAssertion` (with `hmac-secret`) via
    /// this instance's transport, using `credential` as caller-chosen
    /// challenge/context bytes — typically a fresh random nonce
    /// generated per unlock attempt, analogous to WebAuthn's
    /// `challenge`. This is the one authenticator in this crate where
    /// `credential` is *not* a password/code the caller already had;
    /// it's freshness material for the hardware round-trip `verify()`
    /// performs itself.
    ///
    /// Checks (in order): the credential bytes are non-empty; the
    /// transport reaches a device and gets an assertion within the
    /// configured touch timeout; the response's rpId hash matches
    /// `SHA-256(relying_party_id)`; the response's credential id
    /// matches the registered one; the response reports user
    /// presence; the response carries an `hmac-secret` output. Only
    /// after every one of those passes does `verify()` reach the
    /// final, still-unconditional refusal — seeing real hardware
    /// answer correctly for the registered credential is not the same
    /// as cryptographically proving it, and this type does not treat
    /// them as the same thing.
    fn verify(&self, credential: &[u8]) -> Result<AuthAssertion, AuthError> {
        if credential.is_empty() {
            return Err(AuthError::MalformedCredential);
        }
        let request = Ctap2AssertionRequest {
            relying_party_id: &self.relying_party_id,
            credential_id: &self.credential_id,
            // `credential` may be any length; collapse it to the
            // fixed 32 bytes `Ctap2AssertionRequest::challenge`
            // expects. The transport hashes this once more itself —
            // see that field's doc for why hashing it here too isn't
            // a correctness problem, only a naming one to get right.
            challenge: sha256(credential),
            hmac_secret_salt: self.hmac_secret_salt(),
            touch_timeout: self.touch_timeout,
        };
        let response =
            self.transport
                .get_hmac_secret_assertion(&request)
                .map_err(|err| match err {
                    Ctap2TransportError::NoDeviceAvailable => AuthError::HardwareUnavailable,
                    Ctap2TransportError::TouchTimedOut => AuthError::HardwareTimeout,
                    Ctap2TransportError::Failed { detail } => {
                        AuthError::HardwareTransport { detail }
                    }
                })?;

        let expected_rpid_hash = sha256(self.relying_party_id.as_bytes());
        if !ct_eq(&response.rpid_hash, &expected_rpid_hash) {
            return Err(AuthError::InvalidCredential);
        }
        if response.credential_id != self.credential_id {
            return Err(AuthError::InvalidCredential);
        }
        if !response.user_present {
            return Err(AuthError::InvalidCredential);
        }
        let hmac_secret_output = response
            .hmac_secret_output
            .ok_or(AuthError::InvalidCredential)?;

        // Every structural check above passed against a real hardware
        // response, and `hmac_secret_output` (about to be dropped —
        // `Zeroizing` wipes it) is exactly the bytes a real
        // `key_contribution` would use. What's still missing is the
        // one piece that turns "a device answered for this
        // credential id" into "the registered credential's private
        // key produced this": ECDSA P-256 verification of
        // `response.signature` over `response.auth_data ||
        // SHA-256(request.challenge)`. Refuse rather than trust it
        // anyway.
        drop(hmac_secret_output);
        Err(AuthError::Unimplemented {
            backend: "ECDSA P-256 assertion-signature verification (WebAuthn PRF)",
        })
    }
}

// ─────────────────────────────────────────────────────────────────────
// 6. Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fast_password_authenticator(password: &[u8]) -> PasswordAuthenticator {
        // Low Argon2id parameters so the test is fast.
        let salt: Vec<u8> = b"saltsaltsaltsalt".to_vec();
        let verifier =
            PasswordAuthenticator::derive_verifier(password, &salt, 1, 8, 1, 32).unwrap();
        PasswordAuthenticator::with_verifier(salt, 1, 8, 1, verifier).unwrap()
    }

    // --- Password ---

    #[test]
    fn password_correct_verify_succeeds_with_bind_contribution() {
        let auth = fast_password_authenticator(b"correct horse battery staple");
        let assertion = auth.verify(b"correct horse battery staple").unwrap();
        assert_eq!(assertion.kind, "password");
        assert_eq!(assertion.mode, Mode::Bind);
        let k = assertion
            .key_contribution
            .as_ref()
            .expect("bind-mode contribution");
        assert_eq!(k.len(), 32);
    }

    #[test]
    fn password_assertion_summary_hides_key_bytes() {
        let auth = fast_password_authenticator(b"correct horse battery staple");
        let assertion = auth.verify(b"correct horse battery staple").unwrap();

        let summary = assertion.summary();

        assert_eq!(summary.kind, "password");
        assert_eq!(summary.mode, Mode::Bind);
        assert!(summary.has_key_contribution);
        assert_eq!(summary.key_contribution_len, 32);
        assert!(summary.contributes_key_material());
    }

    #[test]
    fn password_wrong_password_rejected() {
        let auth = fast_password_authenticator(b"good");
        match auth.verify(b"bad") {
            Err(AuthError::InvalidCredential) => {}
            other => panic!(
                "expected InvalidCredential, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn password_empty_credential_is_malformed() {
        let auth = fast_password_authenticator(b"good");
        match auth.verify(b"") {
            Err(AuthError::MalformedCredential) => {}
            other => panic!(
                "expected MalformedCredential, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn password_with_verifier_rejects_short_salt() {
        match PasswordAuthenticator::with_verifier(vec![1, 2], 1, 8, 1, vec![0u8; 32]) {
            Err(AuthError::InvalidParameter { .. }) => {}
            other => panic!(
                "expected InvalidParameter, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn password_key_contribution_is_deterministic() {
        // Same password → same key_contribution bytes (so
        // combine_key_contributions is reproducible).
        let auth1 = fast_password_authenticator(b"pw");
        let auth2 = fast_password_authenticator(b"pw");
        let a1 = auth1.verify(b"pw").unwrap();
        let a2 = auth2.verify(b"pw").unwrap();
        let k1 = a1.key_contribution.as_ref().unwrap();
        let k2 = a2.key_contribution.as_ref().unwrap();
        assert_eq!(&k1[..], &k2[..]);
    }

    // --- TOTP — RFC 6238 known vectors ---

    /// The three RFC 6238 Appendix B seeds.
    ///
    /// The RFC's table is often quoted as if one 20-byte secret
    /// produced all eighteen codes. It does not: the reference
    /// implementation in Appendix A defines `seed`, `seed32`, and
    /// `seed64`, and each is the ASCII string "1234567890" repeated
    /// until it fills the hash's block-friendly width. Testing all
    /// three against one secret is the classic way to "prove" a
    /// SHA-256 implementation that is quietly still SHA-1.
    fn rfc6238_secret() -> Vec<u8> {
        b"12345678901234567890".to_vec()
    }

    fn rfc6238_secret_sha256() -> Vec<u8> {
        b"12345678901234567890123456789012".to_vec()
    }

    fn rfc6238_secret_sha512() -> Vec<u8> {
        b"1234567890123456789012345678901234567890123456789012345678901234".to_vec()
    }

    /// RFC 6238 Appendix B, in full: every published timestamp
    /// against every published algorithm, at the published 8-digit
    /// width, with T0 = 0 and X = 30.
    ///
    /// | T (sec) | SHA-1 | SHA-256 | SHA-512 |
    /// |---:|---:|---:|---:|
    /// | 59 | 94287082 | 46119246 | 90693936 |
    /// | 1111111109 | 07081804 | 68084774 | 25091201 |
    /// | 1111111111 | 14050471 | 67062674 | 99943326 |
    /// | 1234567890 | 89005924 | 91819424 | 93441116 |
    /// | 2000000000 | 69279037 | 90698825 | 38618901 |
    /// | 20000000000 | 65353130 | 77737706 | 47863826 |
    ///
    /// Note the leading zero in the SHA-1 row for T=1111111109. That
    /// row is the reason `formatted_code_at` exists and the reason
    /// this table is compared as *strings*: as an integer the code is
    /// 7081804, which is not what a person types.
    #[test]
    fn totp_reproduces_every_rfc6238_appendix_b_vector() {
        let vectors: [(u64, [&str; 3]); 6] = [
            (59, ["94287082", "46119246", "90693936"]),
            (1_111_111_109, ["07081804", "68084774", "25091201"]),
            (1_111_111_111, ["14050471", "67062674", "99943326"]),
            (1_234_567_890, ["89005924", "91819424", "93441116"]),
            (2_000_000_000, ["69279037", "90698825", "38618901"]),
            (20_000_000_000, ["65353130", "77737706", "47863826"]),
        ];
        let algorithms = [
            (TotpAlgorithm::Sha1, rfc6238_secret()),
            (TotpAlgorithm::Sha256, rfc6238_secret_sha256()),
            (TotpAlgorithm::Sha512, rfc6238_secret_sha512()),
        ];
        for (index, (algorithm, secret)) in algorithms.into_iter().enumerate() {
            let auth = TotpAuthenticator::new(secret, algorithm, 30, 8, 1).unwrap();
            for (unix_time_sec, expected) in vectors {
                assert_eq!(
                    auth.formatted_code_at(unix_time_sec).unwrap().as_str(),
                    expected[index],
                    "RFC 6238 Appendix B, {algorithm:?} at T={unix_time_sec}"
                );
            }
        }
    }

    /// The same vectors truncated to six digits, which is what a
    /// password manager actually renders.
    ///
    /// Six digits is the last six of the eight, because the modulus
    /// is applied to one 31-bit integer rather than to a decimal
    /// string — so this test also pins the fact that narrowing the
    /// width does not re-derive anything.
    #[test]
    fn totp_six_digit_rendering_is_the_low_six_of_the_published_eight() {
        let auth = TotpAuthenticator::new(rfc6238_secret(), TotpAlgorithm::Sha1, 30, 6, 1).unwrap();
        for (unix_time_sec, eight) in [
            (59_u64, "94287082"),
            (1_111_111_109, "07081804"),
            (1_111_111_111, "14050471"),
            (1_234_567_890, "89005924"),
            (2_000_000_000, "69279037"),
            (20_000_000_000, "65353130"),
        ] {
            assert_eq!(
                auth.formatted_code_at(unix_time_sec).unwrap().as_str(),
                &eight[2..]
            );
        }
    }

    /// One algorithm's seed under another algorithm must not produce
    /// that algorithm's published answer.
    ///
    /// Without this, an implementation that ignored the selector and
    /// always called SHA-1 would still pass the table above for its
    /// SHA-1 row and fail loudly for the others — but an
    /// implementation that mixed *seeds* up could pass by accident.
    #[test]
    fn totp_algorithm_selector_actually_changes_the_hash() {
        let secret = rfc6238_secret();
        let sha1 = TotpAuthenticator::new(secret.clone(), TotpAlgorithm::Sha1, 30, 8, 1).unwrap();
        let sha256 =
            TotpAuthenticator::new(secret.clone(), TotpAlgorithm::Sha256, 30, 8, 1).unwrap();
        let sha512 = TotpAuthenticator::new(secret, TotpAlgorithm::Sha512, 30, 8, 1).unwrap();
        let a = sha1.code_at(59).unwrap();
        let b = sha256.code_at(59).unwrap();
        let c = sha512.code_at(59).unwrap();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    /// The code is constant across a step and changes exactly at the
    /// boundary — not one second early, not one second late.
    #[test]
    fn totp_changes_exactly_at_the_period_boundary() {
        let auth = TotpAuthenticator::new(rfc6238_secret(), TotpAlgorithm::Sha1, 30, 6, 1).unwrap();
        // 1111111110 is 37037037 * 30 exactly: the first second of a
        // step. Its predecessor belongs to the previous step.
        let boundary = 1_111_111_110_u64;
        assert_eq!(boundary % 30, 0);
        let before = auth.code_at(boundary - 1).unwrap();
        let at = auth.code_at(boundary).unwrap();
        assert_ne!(before, at, "code must change at the boundary");
        assert_eq!(auth.code_at(boundary - 2).unwrap(), before);
        assert_eq!(auth.code_at(boundary + 1).unwrap(), at);
        // Constant for the whole step, and only for the whole step.
        for offset in 0..30 {
            assert_eq!(auth.code_at(boundary + offset).unwrap(), at);
        }
        assert_ne!(auth.code_at(boundary + 30).unwrap(), at);
    }

    /// Remaining validity is `period - (t mod period)`: never 0,
    /// never more than the period, and stepping down by one each
    /// second.
    #[test]
    fn totp_remaining_seconds_walks_the_whole_period() {
        let auth = TotpAuthenticator::new(rfc6238_secret(), TotpAlgorithm::Sha1, 30, 6, 1).unwrap();
        let boundary = 1_111_111_110_u64;
        for offset in 0..30 {
            let remaining = auth.remaining_seconds(boundary + offset);
            assert_eq!(remaining, 30 - offset);
            assert!((1..=30).contains(&remaining));
        }
        assert_eq!(auth.remaining_seconds(boundary + 30), 30);
    }

    /// A ten-digit authenticator used to panic in debug builds and
    /// wrap in release ones, because the modulus was `10u32.pow(10)`
    /// and 10^10 exceeds `u32::MAX`. Ten digits is legal per this
    /// constructor, so it must compute.
    #[test]
    fn totp_ten_digits_does_not_overflow_the_modulus() {
        let auth =
            TotpAuthenticator::new(rfc6238_secret(), TotpAlgorithm::Sha1, 30, 10, 1).unwrap();
        // RFC 4226 dynamic truncation yields a 31-bit value, so at
        // ten digits the modulus cannot bite and the code is the
        // truncation itself, left-padded to width ten.
        let rendered = auth.formatted_code_at(59).unwrap();
        assert_eq!(rendered.len(), 10);
        assert_eq!(rendered.parse::<u32>().unwrap(), auth.code_at(59).unwrap());
        assert!(auth.code_at(59).unwrap() <= 0x7FFF_FFFF);
    }

    /// A step whose truncation is short must still render at full
    /// width. Nine digits over a 31-bit truncation reaches this
    /// often enough to pin it without searching.
    #[test]
    fn totp_rendering_is_zero_padded_to_the_configured_width() {
        let auth = TotpAuthenticator::new(rfc6238_secret(), TotpAlgorithm::Sha1, 30, 8, 1).unwrap();
        // The published SHA-1 vector at T=1111111109 is 07081804.
        let rendered = auth.formatted_code_at(1_111_111_109).unwrap();
        assert_eq!(rendered.as_str(), "07081804");
        assert_eq!(rendered.len(), 8);
        assert_eq!(auth.code_at(1_111_111_109).unwrap(), 7_081_804);
    }

    #[test]
    fn totp_verify_at_time_accepts_window() {
        let auth = TotpAuthenticator::new(rfc6238_secret(), TotpAlgorithm::Sha1, 30, 6, 1).unwrap();
        // Code at step k must be accepted at step k, k-1, k+1.
        let now = 1_111_111_109;
        let center = now / 30;
        let code = auth.code_at(now).unwrap();
        let prev = auth.code_at_counter(center - 1).unwrap();
        let next = auth.code_at_counter(center + 1).unwrap();
        assert!(auth.verify_at_time(code, now).is_ok());
        assert!(auth.verify_at_time(prev, now).is_ok());
        assert!(auth.verify_at_time(next, now).is_ok());
    }

    #[test]
    fn totp_verify_at_time_rejects_outside_window() {
        let auth = TotpAuthenticator::new(rfc6238_secret(), TotpAlgorithm::Sha1, 30, 6, 1).unwrap();
        let now = 1_111_111_109;
        let center = now / 30;
        // Code from step center-2 should NOT be accepted with window=1.
        let outside = auth.code_at_counter(center - 2).unwrap();
        match auth.verify_at_time(outside, now) {
            Err(AuthError::InvalidCredential) => {}
            other => panic!(
                "expected InvalidCredential outside window, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn totp_invalid_parameters_rejected() {
        match TotpAuthenticator::new(Vec::<u8>::new(), TotpAlgorithm::Sha1, 30, 6, 1) {
            Err(AuthError::InvalidParameter { .. }) => {}
            _ => panic!("expected InvalidParameter for empty secret"),
        }
        match TotpAuthenticator::new(b"x".to_vec(), TotpAlgorithm::Sha1, 0, 6, 1) {
            Err(AuthError::InvalidParameter { .. }) => {}
            _ => panic!("expected InvalidParameter for period 0"),
        }
        match TotpAuthenticator::new(b"x".to_vec(), TotpAlgorithm::Sha1, 30, 11, 1) {
            Err(AuthError::InvalidParameter { .. }) => {}
            _ => panic!("expected InvalidParameter for digits 11"),
        }
    }

    #[test]
    fn totp_malformed_credential_rejected() {
        let auth = TotpAuthenticator::new(rfc6238_secret(), TotpAlgorithm::Sha1, 30, 6, 1).unwrap();
        // Wrong digit count.
        match auth.verify(b"1234") {
            Err(AuthError::MalformedCredential) => {}
            _ => panic!("expected MalformedCredential for short code"),
        }
        // Non-decimal.
        match auth.verify(b"abcdef") {
            Err(AuthError::MalformedCredential) => {}
            _ => panic!("expected MalformedCredential for non-decimal"),
        }
    }

    #[test]
    fn totp_assertion_has_no_key_contribution() {
        // Build a small verify_at_time path so we don't depend on
        // wall clock matching a specific code.
        let auth = TotpAuthenticator::new(rfc6238_secret(), TotpAlgorithm::Sha1, 30, 6, 1).unwrap();
        let code = auth.code_at(1_111_111_109).unwrap();
        let step = auth.verify_at_time(code, 1_111_111_109).unwrap();
        assert_eq!(step, 1_111_111_109 / 30);
        // The trait-level verify uses SystemTime::now() so we can't
        // control it; just assert the API shape on the lower-level
        // path (gate-mode authenticators contribute no key).
        let assertion = AuthAssertion {
            kind: "totp",
            mode: Mode::Gate,
            key_contribution: None,
        };
        assert_eq!(assertion.mode, Mode::Gate);
        assert!(assertion.key_contribution.is_none());
    }

    // --- combine_key_contributions ---

    #[test]
    fn combine_yields_deterministic_unlock_key() {
        let auth = fast_password_authenticator(b"pw");
        let a = auth.verify(b"pw").unwrap();
        let b = auth.verify(b"pw").unwrap();
        let k1 = combine_key_contributions(b"vault-1", &[&a]).unwrap();
        let k2 = combine_key_contributions(b"vault-1", &[&b]).unwrap();
        assert_eq!(&k1[..], &k2[..]);
    }

    #[test]
    fn combine_distinct_vault_ids_yield_distinct_keys() {
        let auth = fast_password_authenticator(b"pw");
        let a = auth.verify(b"pw").unwrap();
        let k_a = combine_key_contributions(b"vault-A", &[&a]).unwrap();
        let k_b = combine_key_contributions(b"vault-B", &[&a]).unwrap();
        assert_ne!(&k_a[..], &k_b[..]);
    }

    #[test]
    fn combine_skips_gate_mode_factors() {
        // Gate-only factor list yields NoBindFactors.
        let gate = AuthAssertion {
            kind: "totp",
            mode: Mode::Gate,
            key_contribution: None,
        };
        match combine_key_contributions(b"vault", &[&gate]) {
            Err(AuthError::NoBindFactors) => {}
            _ => panic!("expected NoBindFactors when only gate factors are present"),
        }
    }

    #[test]
    fn assertion_set_summary_counts_gate_and_bind_factors() {
        let auth = fast_password_authenticator(b"pw");
        let bind = auth.verify(b"pw").unwrap();
        let gate = AuthAssertion {
            kind: "totp",
            mode: Mode::Gate,
            key_contribution: None,
        };

        let summary = summarize_auth_assertions(&[&bind, &gate]);

        assert_eq!(summary.assertion_count, 2);
        assert_eq!(summary.bind_count, 1);
        assert_eq!(summary.gate_count, 1);
        assert_eq!(summary.key_contribution_count, 1);
        assert_eq!(summary.total_key_contribution_len, 32);
        assert_eq!(summary.password_count, 1);
        assert_eq!(summary.totp_count, 1);
        assert_eq!(summary.extension_count, 0);
        assert_eq!(summary.missing_bind_contribution_count, 0);
        assert_eq!(summary.unexpected_gate_contribution_count, 0);
        assert!(summary.can_derive_unlock_key());
        assert!(summary.is_multi_factor());
        assert!(!summary.has_extension_factors());
        assert!(summary.is_contribution_consistent());
    }

    #[test]
    fn assertion_set_summary_marks_gate_only_sets_as_not_derivable() {
        let gate = AuthAssertion {
            kind: "totp",
            mode: Mode::Gate,
            key_contribution: None,
        };

        let summary = summarize_auth_assertions(&[&gate]);

        assert_eq!(summary.assertion_count, 1);
        assert_eq!(summary.bind_count, 0);
        assert_eq!(summary.gate_count, 1);
        assert_eq!(summary.key_contribution_count, 0);
        assert_eq!(summary.total_key_contribution_len, 0);
        assert_eq!(summary.password_count, 0);
        assert_eq!(summary.totp_count, 1);
        assert_eq!(summary.extension_count, 0);
        assert!(!summary.can_derive_unlock_key());
        assert!(!summary.is_multi_factor());
        assert!(summary.is_contribution_consistent());
    }

    #[test]
    fn assertion_set_summary_flags_extension_and_contribution_shape() {
        let bind_without_key = AuthAssertion {
            kind: "webauthn-prf",
            mode: Mode::Bind,
            key_contribution: None,
        };
        let gate_with_key = AuthAssertion {
            kind: "push",
            mode: Mode::Gate,
            key_contribution: Some(Zeroizing::new(vec![1, 2, 3])),
        };

        let summary = summarize_auth_assertions(&[&bind_without_key, &gate_with_key]);

        assert_eq!(summary.assertion_count, 2);
        assert_eq!(summary.bind_count, 1);
        assert_eq!(summary.gate_count, 1);
        assert_eq!(summary.password_count, 0);
        assert_eq!(summary.totp_count, 0);
        assert_eq!(summary.extension_count, 2);
        assert_eq!(summary.key_contribution_count, 0);
        assert_eq!(summary.total_key_contribution_len, 0);
        assert_eq!(summary.missing_bind_contribution_count, 1);
        assert_eq!(summary.unexpected_gate_contribution_count, 1);
        assert!(summary.is_multi_factor());
        assert!(summary.has_extension_factors());
        assert!(!summary.can_derive_unlock_key());
        assert!(!summary.is_contribution_consistent());
    }

    #[test]
    fn combine_no_factors_rejected() {
        match combine_key_contributions(b"vault", &[]) {
            Err(AuthError::NoBindFactors) => {}
            _ => panic!("expected NoBindFactors on empty input"),
        }
    }

    // --- Errors ---

    #[test]
    fn error_messages_are_static_literals() {
        let errs: Vec<AuthError> = vec![
            AuthError::InvalidCredential,
            AuthError::MalformedCredential,
            AuthError::InvalidParameter { what: "x" },
            AuthError::Crypto,
            AuthError::NoBindFactors,
            AuthError::Unimplemented {
                backend: "ECDSA P-256 assertion-signature verification (WebAuthn PRF)",
            },
            AuthError::HardwareUnavailable,
            AuthError::HardwareTimeout,
            AuthError::HardwareTransport {
                detail: "HID read error",
            },
        ];
        for e in &errs {
            let s = e.to_string();
            assert!(s.starts_with("vault-auth:"));
        }
    }

    // --- WebAuthnPrfAuthenticator ---

    const RP_ID: &str = "vault-pm";
    const CRED_ID: &[u8] = b"credential-id-bytes";
    const PUBKEY: &[u8] = b"cose-public-key-bytes";

    /// What the fake transport should hand back for one test.
    #[derive(Clone, Copy)]
    enum FakeOutcome {
        /// A structurally-correct response for `RP_ID`/`CRED_ID`, with
        /// `hmac-secret` output present and user-presence set.
        Correct,
        /// Same, but the rpId hash doesn't match `RP_ID`.
        WrongRelyingParty,
        /// Same, but the credential id doesn't match `CRED_ID`.
        WrongCredentialId,
        /// Same, but the `user_present` flag is unset.
        NoUserPresence,
        /// Same, but the authenticator didn't return an `hmac-secret`
        /// output at all (device doesn't support the extension).
        NoHmacSecretExtension,
        /// The transport itself failed before/without reaching a
        /// device successfully.
        TransportError(Ctap2TransportError),
    }

    /// In-process stand-in for real CTAP2 hardware I/O. No USB, no
    /// HID, no `ctap-hid-fido2` — just enough of `Ctap2Transport`'s
    /// contract to exercise `WebAuthnPrfAuthenticator::verify`'s own
    /// logic (request shape, response validation, error mapping)
    /// deterministically and instantly. Panics if `verify()` ever
    /// calls the transport when it shouldn't (e.g. on an empty
    /// credential, which must short-circuit before any I/O).
    ///
    /// What one fake transport call saw: the relying-party id and
    /// credential id it was asked to assert with.
    type SeenRequest = (String, Vec<u8>);
    /// Shared handle onto a `FakeTransport`'s last-seen request.
    /// `Arc<Mutex<..>>` (not `Rc<RefCell<..>>`) because
    /// `WebAuthnPrfAuthenticator::new` requires `Send + Sync`
    /// transports.
    type SeenHandle = std::sync::Arc<std::sync::Mutex<Option<SeenRequest>>>;

    /// In-process stand-in for real CTAP2 hardware I/O. No USB, no
    /// HID, no `ctap-hid-fido2` — just enough of `Ctap2Transport`'s
    /// contract to exercise `WebAuthnPrfAuthenticator::verify`'s own
    /// logic (request shape, response validation, error mapping)
    /// deterministically and instantly. Panics if `verify()` ever
    /// calls the transport when it shouldn't (e.g. on an empty
    /// credential, which must short-circuit before any I/O).
    struct FakeTransport {
        outcome: FakeOutcome,
        seen: SeenHandle,
    }

    impl FakeTransport {
        fn new(outcome: FakeOutcome) -> Self {
            Self {
                outcome,
                seen: std::sync::Arc::new(std::sync::Mutex::new(None)),
            }
        }

        /// Build a fake plus a cloned handle onto its `seen` cell, for
        /// tests that need to inspect calls after handing the fake's
        /// ownership away — a test can clone this handle *before*
        /// moving the transport into `WebAuthnPrfAuthenticator::new`
        /// (which takes ownership via `Box<dyn Ctap2Transport>`) and
        /// still inspect what request `verify()` actually sent.
        fn new_with_handle(outcome: FakeOutcome) -> (Self, SeenHandle) {
            let fake = Self::new(outcome);
            let handle = fake.seen.clone();
            (fake, handle)
        }
    }

    impl Ctap2Transport for FakeTransport {
        fn get_hmac_secret_assertion(
            &self,
            request: &Ctap2AssertionRequest<'_>,
        ) -> Result<Ctap2AssertionResponse, Ctap2TransportError> {
            *self.seen.lock().unwrap() = Some((
                request.relying_party_id.to_string(),
                request.credential_id.to_vec(),
            ));
            match self.outcome {
                FakeOutcome::TransportError(e) => Err(e),
                FakeOutcome::Correct
                | FakeOutcome::WrongRelyingParty
                | FakeOutcome::WrongCredentialId
                | FakeOutcome::NoUserPresence
                | FakeOutcome::NoHmacSecretExtension => {
                    let rpid_hash = if matches!(self.outcome, FakeOutcome::WrongRelyingParty) {
                        sha256(b"attacker.example")
                    } else {
                        sha256(RP_ID.as_bytes())
                    };
                    let credential_id = if matches!(self.outcome, FakeOutcome::WrongCredentialId) {
                        b"some-other-credential".to_vec()
                    } else {
                        CRED_ID.to_vec()
                    };
                    let hmac_secret_output =
                        if matches!(self.outcome, FakeOutcome::NoHmacSecretExtension) {
                            None
                        } else {
                            Some(Zeroizing::new(vec![0x42; 32]))
                        };
                    Ok(Ctap2AssertionResponse {
                        rpid_hash,
                        credential_id,
                        user_present: !matches!(self.outcome, FakeOutcome::NoUserPresence),
                        signature: vec![0xAB; 64],
                        auth_data: vec![0xCD; 37],
                        hmac_secret_output,
                    })
                }
            }
        }
    }

    /// A transport that panics if it's ever invoked — for tests that
    /// must prove `verify()` short-circuits before any hardware I/O.
    struct UnreachableTransport;
    impl Ctap2Transport for UnreachableTransport {
        fn get_hmac_secret_assertion(
            &self,
            _request: &Ctap2AssertionRequest<'_>,
        ) -> Result<Ctap2AssertionResponse, Ctap2TransportError> {
            panic!("transport must not be called for this input");
        }
    }

    fn webauthn_prf_with(outcome: FakeOutcome) -> WebAuthnPrfAuthenticator {
        WebAuthnPrfAuthenticator::new(
            RP_ID,
            CRED_ID.to_vec(),
            PUBKEY.to_vec(),
            FakeTransport::new(outcome),
        )
        .unwrap()
    }

    #[test]
    fn webauthn_prf_reports_bind_mode_and_kind() {
        let auth = webauthn_prf_with(FakeOutcome::Correct);
        assert_eq!(auth.kind(), "webauthn-prf");
        assert_eq!(auth.mode(), Mode::Bind);
    }

    #[test]
    fn webauthn_prf_accessors_round_trip_construction_inputs() {
        let auth = webauthn_prf_with(FakeOutcome::Correct);
        assert_eq!(auth.relying_party_id(), "vault-pm");
        assert_eq!(auth.credential_id(), CRED_ID);
        assert_eq!(auth.public_key_cose(), PUBKEY);
        assert_eq!(auth.touch_timeout(), DEFAULT_TOUCH_TIMEOUT);
    }

    #[test]
    fn webauthn_prf_verify_still_refuses_after_a_correct_hardware_round_trip() {
        // Real (fake) hardware answered correctly for the registered
        // credential — every structural check passes — and `verify()`
        // still refuses, because ECDSA P-256 verification doesn't
        // exist in this workspace yet. This is the load-bearing test:
        // it proves the refusal is scoped to the missing primitive,
        // not to "the transport never got called."
        let auth = webauthn_prf_with(FakeOutcome::Correct);
        match auth.verify(b"a fresh per-attempt challenge nonce") {
            Err(AuthError::Unimplemented { backend }) => {
                assert!(backend.contains("ECDSA"));
            }
            other => panic!(
                "expected Unimplemented, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_verify_sends_the_registered_rp_and_credential_id() {
        let (transport, seen) = FakeTransport::new_with_handle(FakeOutcome::Correct);
        let auth =
            WebAuthnPrfAuthenticator::new(RP_ID, CRED_ID.to_vec(), PUBKEY.to_vec(), transport)
                .unwrap();
        assert!(seen.lock().unwrap().is_none());
        let _ = auth.verify(b"a fresh per-attempt challenge");
        let (seen_rp, seen_cred) = seen.lock().unwrap().clone().expect("transport was called");
        assert_eq!(seen_rp, RP_ID);
        assert_eq!(seen_cred, CRED_ID);
    }

    #[test]
    fn webauthn_prf_verify_rejects_wrong_relying_party_hash() {
        let auth = webauthn_prf_with(FakeOutcome::WrongRelyingParty);
        match auth.verify(b"challenge") {
            Err(AuthError::InvalidCredential) => {}
            other => panic!(
                "expected InvalidCredential, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_verify_rejects_credential_id_mismatch() {
        let auth = webauthn_prf_with(FakeOutcome::WrongCredentialId);
        match auth.verify(b"challenge") {
            Err(AuthError::InvalidCredential) => {}
            other => panic!(
                "expected InvalidCredential, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_verify_rejects_missing_user_presence() {
        let auth = webauthn_prf_with(FakeOutcome::NoUserPresence);
        match auth.verify(b"challenge") {
            Err(AuthError::InvalidCredential) => {}
            other => panic!(
                "expected InvalidCredential, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_verify_rejects_missing_hmac_secret_extension() {
        let auth = webauthn_prf_with(FakeOutcome::NoHmacSecretExtension);
        match auth.verify(b"challenge") {
            Err(AuthError::InvalidCredential) => {}
            other => panic!(
                "expected InvalidCredential, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_verify_maps_no_device_to_hardware_unavailable() {
        let auth = webauthn_prf_with(FakeOutcome::TransportError(
            Ctap2TransportError::NoDeviceAvailable,
        ));
        match auth.verify(b"challenge") {
            Err(AuthError::HardwareUnavailable) => {}
            other => panic!(
                "expected HardwareUnavailable, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_verify_maps_touch_timeout() {
        let auth = webauthn_prf_with(FakeOutcome::TransportError(
            Ctap2TransportError::TouchTimedOut,
        ));
        match auth.verify(b"challenge") {
            Err(AuthError::HardwareTimeout) => {}
            other => panic!(
                "expected HardwareTimeout, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_verify_maps_generic_transport_failure() {
        let auth = webauthn_prf_with(FakeOutcome::TransportError(Ctap2TransportError::Failed {
            detail: "HID read error",
        }));
        match auth.verify(b"challenge") {
            Err(AuthError::HardwareTransport { detail }) => {
                assert_eq!(detail, "HID read error");
            }
            other => panic!(
                "expected HardwareTransport, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_verify_empty_credential_short_circuits_before_any_hardware_io() {
        // Empty per-attempt challenge bytes must be rejected before
        // the transport is ever invoked — `UnreachableTransport`
        // panics if `get_hmac_secret_assertion` is called at all, so
        // this test also proves the short-circuit, not just the
        // error variant.
        let auth = WebAuthnPrfAuthenticator::new(
            RP_ID,
            CRED_ID.to_vec(),
            PUBKEY.to_vec(),
            UnreachableTransport,
        )
        .unwrap();
        match auth.verify(b"") {
            Err(AuthError::MalformedCredential) => {}
            other => panic!(
                "expected MalformedCredential, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_hmac_secret_salt_is_stable_across_attempts_and_independent_of_credential_bytes()
    {
        // The salt that feeds `hmac-secret` must be the same on every
        // unlock attempt for the same registered credential, even
        // though the per-attempt `credential` challenge bytes differ
        // every time — otherwise the derived secret (and therefore
        // `key_contribution`) would change on every unlock and could
        // never decrypt data encrypted under a previous unlock key.
        let a = webauthn_prf_with(FakeOutcome::Correct);
        let b = webauthn_prf_with(FakeOutcome::Correct);
        assert_eq!(a.hmac_secret_salt(), b.hmac_secret_salt());

        // Different registered credentials must NOT share a salt.
        let other = WebAuthnPrfAuthenticator::new(
            RP_ID,
            b"a-completely-different-credential-id".to_vec(),
            PUBKEY.to_vec(),
            FakeTransport::new(FakeOutcome::Correct),
        )
        .unwrap();
        assert_ne!(a.hmac_secret_salt(), other.hmac_secret_salt());
    }

    #[test]
    fn webauthn_prf_rejects_empty_relying_party_id() {
        match WebAuthnPrfAuthenticator::new(
            "",
            b"cred".to_vec(),
            b"key".to_vec(),
            FakeTransport::new(FakeOutcome::Correct),
        ) {
            Err(AuthError::InvalidParameter { .. }) => {}
            other => panic!(
                "expected InvalidParameter, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_rejects_empty_credential_id() {
        match WebAuthnPrfAuthenticator::new(
            "vault-pm",
            Vec::<u8>::new(),
            b"key".to_vec(),
            FakeTransport::new(FakeOutcome::Correct),
        ) {
            Err(AuthError::InvalidParameter { .. }) => {}
            other => panic!(
                "expected InvalidParameter, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_rejects_empty_public_key() {
        match WebAuthnPrfAuthenticator::new(
            "vault-pm",
            b"cred".to_vec(),
            Vec::<u8>::new(),
            FakeTransport::new(FakeOutcome::Correct),
        ) {
            Err(AuthError::InvalidParameter { .. }) => {}
            other => panic!(
                "expected InvalidParameter, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_rejects_touch_timeout_below_minimum() {
        match WebAuthnPrfAuthenticator::with_touch_timeout(
            RP_ID,
            CRED_ID.to_vec(),
            PUBKEY.to_vec(),
            FakeTransport::new(FakeOutcome::Correct),
            Duration::from_millis(500),
        ) {
            Err(AuthError::InvalidParameter { .. }) => {}
            other => panic!(
                "expected InvalidParameter, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_rejects_touch_timeout_above_maximum() {
        match WebAuthnPrfAuthenticator::with_touch_timeout(
            RP_ID,
            CRED_ID.to_vec(),
            PUBKEY.to_vec(),
            FakeTransport::new(FakeOutcome::Correct),
            Duration::from_secs(600),
        ) {
            Err(AuthError::InvalidParameter { .. }) => {}
            other => panic!(
                "expected InvalidParameter, got {}",
                if other.is_ok() { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn webauthn_prf_with_touch_timeout_accepts_bounds_inclusive() {
        assert!(WebAuthnPrfAuthenticator::with_touch_timeout(
            RP_ID,
            CRED_ID.to_vec(),
            PUBKEY.to_vec(),
            FakeTransport::new(FakeOutcome::Correct),
            MIN_TOUCH_TIMEOUT,
        )
        .is_ok());
        assert!(WebAuthnPrfAuthenticator::with_touch_timeout(
            RP_ID,
            CRED_ID.to_vec(),
            PUBKEY.to_vec(),
            FakeTransport::new(FakeOutcome::Correct),
            MAX_TOUCH_TIMEOUT,
        )
        .is_ok());
    }

    #[test]
    fn webauthn_prf_kind_is_counted_as_extension_factor_in_summaries() {
        // vault-pm's own summarize_auth_assertions bucket for kinds
        // other than "password"/"totp" is "extension" — pin that a
        // successful webauthn-prf assertion (once verify() is real)
        // would land there, matching the existing extension-factor
        // test in this module.
        let assertion = AuthAssertion {
            kind: "webauthn-prf",
            mode: Mode::Bind,
            key_contribution: Some(Zeroizing::new(vec![1, 2, 3, 4])),
        };
        let summary = summarize_auth_assertions(&[&assertion]);
        assert_eq!(summary.extension_count, 1);
        assert_eq!(summary.password_count, 0);
        assert_eq!(summary.totp_count, 0);
        assert!(summary.can_derive_unlock_key());
    }
}
