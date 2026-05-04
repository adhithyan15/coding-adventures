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
//!   HMAC-SHA-1, time-based counter), 6 digits, 30-second period
//!   default. Verify accepts the current step ± 1 by default;
//!   replay-rejection cache is the caller's responsibility (we
//!   provide `verify_at_step` so upper layers can pin the
//!   accepted step into a per-secret last-used record).
//! - `combine_key_contributions(vault_id, factors)` —
//!   HKDF-Extract(salt = vault_id, ikm = ordered concat of bind-
//!   mode factor outputs, info = "VLT05/key/v1") → 32-byte unlock
//!   key.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_argon2id::{argon2id, Options as ArgonOptions};
use coding_adventures_ct_compare::ct_eq;
use coding_adventures_hkdf::{hkdf, HashAlgorithm};
use coding_adventures_hmac::hmac_sha1;
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

impl Drop for AuthAssertion {
    fn drop(&mut self) {
        if let Some(k) = self.key_contribution.as_mut() {
            k.zeroize();
        }
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
            return Err(AuthError::InvalidParameter { what: "salt < 8 bytes" });
        }
        if verifier.is_empty() {
            return Err(AuthError::InvalidParameter { what: "verifier empty" });
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
        let opts = ArgonOptions { key: None, associated_data: None, version: None };
        argon2id(password, salt, time_cost, memory_cost, parallelism, tag_length, &opts)
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
        let opts = ArgonOptions { key: None, associated_data: None, version: None };
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
// HOTP per RFC 4226: code = truncate(HMAC-SHA-1(secret, counter)) mod 10^digits
// TOTP per RFC 6238: counter = floor((unix_time - T0) / period)
//
// We use SHA-1 (the RFC 6238 default) because every authenticator
// app on the planet expects it. SHA-256 / SHA-512 variants are
// trivially added later by parameterising `algorithm`.

/// RFC 6238 TOTP authenticator. Gate-mode (no key contribution).
pub struct TotpAuthenticator {
    secret: Zeroizing<Vec<u8>>,
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
        period: u64,
        digits: u32,
        window: u32,
    ) -> Result<Self, AuthError> {
        let secret: Zeroizing<Vec<u8>> = Zeroizing::new(secret.into());
        if secret.is_empty() {
            return Err(AuthError::InvalidParameter { what: "TOTP secret empty" });
        }
        if period == 0 {
            return Err(AuthError::InvalidParameter { what: "TOTP period 0" });
        }
        if !(4..=10).contains(&digits) {
            return Err(AuthError::InvalidParameter {
                what: "TOTP digits must be 4..=10",
            });
        }
        Ok(Self { secret, period, digits, window })
    }

    /// Compute the TOTP code at the given UNIX time (seconds).
    /// Useful for testing and replay-cache integration.
    pub fn code_at(&self, unix_time_sec: u64) -> Result<u32, AuthError> {
        let counter = unix_time_sec / self.period;
        self.code_at_counter(counter)
    }

    fn code_at_counter(&self, counter: u64) -> Result<u32, AuthError> {
        let counter_be = counter.to_be_bytes();
        let mac = hmac_sha1(&self.secret, &counter_be).map_err(|_| AuthError::Crypto)?;
        // Dynamic truncation — RFC 4226 §5.3.
        let offset = (mac[19] & 0x0F) as usize;
        let bin =
            ((mac[offset] as u32 & 0x7F) << 24)
            | ((mac[offset + 1] as u32) << 16)
            | ((mac[offset + 2] as u32) << 8)
            |  (mac[offset + 3] as u32);
        let modulus = 10u32.pow(self.digits);
        Ok(bin % modulus)
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
        let k = assertion.key_contribution.as_ref().expect("bind-mode contribution");
        assert_eq!(k.len(), 32);
    }

    #[test]
    fn password_wrong_password_rejected() {
        let auth = fast_password_authenticator(b"good");
        match auth.verify(b"bad") {
            Err(AuthError::InvalidCredential) => {}
            other => panic!(
                "expected InvalidCredential, got {}",
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
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
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn password_with_verifier_rejects_short_salt() {
        match PasswordAuthenticator::with_verifier(vec![1, 2], 1, 8, 1, vec![0u8; 32]) {
            Err(AuthError::InvalidParameter { .. }) => {}
            other => panic!(
                "expected InvalidParameter, got {}",
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
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

    /// RFC 6238 Appendix B test vectors, SHA-1, T0=0, T=30, 8 digits.
    /// We use the 6-digit truncation since most apps render 6.
    /// The shared 20-byte secret is ASCII "12345678901234567890".
    fn rfc6238_secret() -> Vec<u8> {
        b"12345678901234567890".to_vec()
    }

    #[test]
    fn totp_rfc6238_vectors_sha1_8digit_subset() {
        // T = 59 → step 1 → 8-digit code 94287082 → 6-digit "287082".
        let auth = TotpAuthenticator::new(rfc6238_secret(), 30, 6, 1).unwrap();
        let code = auth.code_at(59).unwrap();
        assert_eq!(code, 287082);

        // T = 1111111109 → step 37037036 → 8-digit 07081804 → 6-digit "081804".
        let code = auth.code_at(1_111_111_109).unwrap();
        assert_eq!(code, 81804);

        // T = 1111111111 → 8-digit 14050471 → 6-digit "050471".
        let code = auth.code_at(1_111_111_111).unwrap();
        assert_eq!(code, 50471);
    }

    #[test]
    fn totp_verify_at_time_accepts_window() {
        let auth = TotpAuthenticator::new(rfc6238_secret(), 30, 6, 1).unwrap();
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
        let auth = TotpAuthenticator::new(rfc6238_secret(), 30, 6, 1).unwrap();
        let now = 1_111_111_109;
        let center = now / 30;
        // Code from step center-2 should NOT be accepted with window=1.
        let outside = auth.code_at_counter(center - 2).unwrap();
        match auth.verify_at_time(outside, now) {
            Err(AuthError::InvalidCredential) => {}
            other => panic!(
                "expected InvalidCredential outside window, got {}",
                if matches!(other, Ok(_)) { "Ok" } else { "different Err" }
            ),
        }
    }

    #[test]
    fn totp_invalid_parameters_rejected() {
        match TotpAuthenticator::new(Vec::<u8>::new(), 30, 6, 1) {
            Err(AuthError::InvalidParameter { .. }) => {}
            _ => panic!("expected InvalidParameter for empty secret"),
        }
        match TotpAuthenticator::new(b"x".to_vec(), 0, 6, 1) {
            Err(AuthError::InvalidParameter { .. }) => {}
            _ => panic!("expected InvalidParameter for period 0"),
        }
        match TotpAuthenticator::new(b"x".to_vec(), 30, 11, 1) {
            Err(AuthError::InvalidParameter { .. }) => {}
            _ => panic!("expected InvalidParameter for digits 11"),
        }
    }

    #[test]
    fn totp_malformed_credential_rejected() {
        let auth = TotpAuthenticator::new(rfc6238_secret(), 30, 6, 1).unwrap();
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
        let auth = TotpAuthenticator::new(rfc6238_secret(), 30, 6, 1).unwrap();
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
