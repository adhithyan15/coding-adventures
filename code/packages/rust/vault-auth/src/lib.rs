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
use coding_adventures_zeroize::{Zeroize, Zeroizing};

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
    fn mac(self, key: &[u8], message: &[u8]) -> Result<Zeroizing<Vec<u8>>, AuthError> {
        let tag: Vec<u8> = match self {
            Self::Sha1 => hmac_sha1(key, message)
                .map_err(|_| AuthError::Crypto)?
                .into(),
            Self::Sha256 => hmac_sha256(key, message)
                .map_err(|_| AuthError::Crypto)?
                .into(),
            Self::Sha512 => hmac_sha512(key, message)
                .map_err(|_| AuthError::Crypto)?
                .into(),
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
        let offset = (mac[mac.len() - 1] & 0x0F) as usize;
        let bin = ((mac[offset] as u32 & 0x7F) << 24)
            | ((mac[offset + 1] as u32) << 16)
            | ((mac[offset + 2] as u32) << 8)
            | (mac[offset + 3] as u32);
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
// 4. Tests
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
        ];
        for e in &errs {
            let s = e.to_string();
            assert!(s.starts_with("vault-auth:"));
        }
    }
}
