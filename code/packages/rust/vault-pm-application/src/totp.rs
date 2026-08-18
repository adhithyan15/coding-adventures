//! Current-code computation for one stored `TOTP_SEED_V1` record.
//!
//! # Why this lives inside the application boundary
//!
//! VLT-PM45 §7. The obvious way to build `vault-pm totp code` would be to reuse
//! the existing seed reveal — hand the raw seed bytes to the CLI and let it
//! compute — and it would work. It would also mean that a command whose whole
//! purpose is to *avoid* showing anyone the seed nevertheless materializes the
//! seed in the process's outermost layer, where the terminal, the argument
//! parser, and every future command handler live.
//!
//! So the computation happens here, one module away from the decryption, and
//! what crosses the boundary is six digits and a countdown. That is the entire
//! design rationale for this file existing rather than being three lines in the
//! CLI.
//!
//! # The RFC in one paragraph
//!
//! RFC 6238 defines a time-based one-time password as RFC 4226's HOTP with the
//! counter supplied by the clock instead of by a button press:
//!
//! ```text
//! T    = floor((unix_seconds - T0) / period)      with T0 = 0
//! code = HOTP(seed, T) = truncate(HMAC-H(seed, T)) mod 10^digits
//! ```
//!
//! Both sides compute it independently and never exchange it, which is why a
//! disagreeing clock is indistinguishable from a wrong secret. The HMAC itself
//! is not implemented here — `coding_adventures_vault_auth` owns it, is tested
//! against the full RFC 6238 Appendix B table, and is where VLT-PM00 §6's reuse
//! map already put TOTP.
//!
//! # What this module refuses to guess
//!
//! Three parameters — algorithm, digit count, period — come from the stored
//! record and from nowhere else. There is no flag for them, no default applied
//! when one is missing, and no fallback when one is unrecognized. VLT-PM29
//! validates them at the CLI input boundary, but the *codec* does not, so a
//! record carrying `"SHA3-256"` or 12 digits can reach this function by way of
//! a portable import from some other product.
//!
//! Such a record gets [`ApplicationError::Unsupported`] and no code. The
//! alternative — compute under SHA-1 anyway, or clamp the digits — produces six
//! plausible digits that are simply wrong, and a wrong TOTP code is
//! indistinguishable from a right one until a login fails. Failing closed is
//! the only option that tells the truth.

use crate::ApplicationError;
use coding_adventures_vault_auth::{TotpAlgorithm, TotpAuthenticator};
use coding_adventures_vault_records::AnyRecord;
use coding_adventures_zeroize::{Zeroize, Zeroizing};

/// One computed TOTP code together with the window it stays valid in.
///
/// The two halves have deliberately different privacy: the code is a live
/// credential, and the countdown is a fact about the clock that anyone with a
/// watch can reproduce. So this type implements neither `Debug`, `Display`, nor
/// `Clone` — a `Debug` derive here would put a valid second factor into every
/// future `{:?}` of any structure that ever contains one — while the countdown
/// is readable through an ordinary accessor and is safe to print to standard
/// output.
///
/// The code's backing allocation is wiped on drop.
pub struct TotpCodeV1 {
    code: Zeroizing<String>,
    remaining_seconds: u32,
    period_seconds: u32,
}

impl TotpCodeV1 {
    /// Borrow the code only for the duration of the host disclosure action.
    ///
    /// The string is already zero-padded to the record's digit count, because
    /// `042311` and `42311` are different things to type and only one of them
    /// is the code.
    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    /// Seconds this code remains valid, in `1..=period`.
    ///
    /// Never `0`: a code with zero seconds left has already been replaced by
    /// its successor, so it is not the code this function's caller was handed.
    ///
    /// The figure is measured at computation time, which is before the audit
    /// event is published and before the terminal write, so it can be
    /// optimistic by however long those take — milliseconds. VLT-PM45 §5.3
    /// documents that rather than correcting it: correcting it would mean
    /// reading the clock again *after* the code was already released, and a
    /// clock failure at that point is a failure nothing can honestly report.
    pub const fn remaining_seconds(&self) -> u32 {
        self.remaining_seconds
    }

    /// The record's configured time step, in seconds.
    pub const fn period_seconds(&self) -> u32 {
        self.period_seconds
    }
}

impl Zeroize for TotpCodeV1 {
    fn zeroize(&mut self) {
        self.code.zeroize();
        self.remaining_seconds = 0;
        self.period_seconds = 0;
    }
}

impl Drop for TotpCodeV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Map the record's stored algorithm name onto the engine's selector.
///
/// The comparison is exact and case-sensitive against the three spellings
/// VLT-PM29 §2 accepts. Accepting `"sha1"` here would mean two spellings of one
/// algorithm exist in stored records, which is how a canonical encoding stops
/// being canonical.
fn algorithm(name: &str) -> Result<TotpAlgorithm, ApplicationError> {
    match name {
        "SHA1" => Ok(TotpAlgorithm::Sha1),
        "SHA256" => Ok(TotpAlgorithm::Sha256),
        "SHA512" => Ok(TotpAlgorithm::Sha512),
        _ => Err(ApplicationError::Unsupported),
    }
}

/// Compute the current code for one record at one caller-supplied instant.
///
/// `code_time_ms` is Unix milliseconds and must be a *fresh* reading. It is not
/// the audit event's advisory timestamp: VLT-PM45 §4.1 requires two separate
/// readings because an Argon2id unlock and a human typing `yes` sit between the
/// pre-authentication reservation and this call, and a whole period is easily
/// reachable in that gap. A code derived from the reserved timestamp would
/// routinely be the *previous* code — six digits, correct-looking, rejected.
///
/// Milliseconds truncate to seconds by flooring, which is what every TOTP
/// client does and keeps the step boundary on a whole second.
///
/// | Record | Result |
/// |---|---|
/// | a TOTP seed with recognized parameters | the code and its window |
/// | any other record kind | `InvalidInput` |
/// | a TOTP seed whose algorithm/digits/period this build cannot compute | `Unsupported` |
pub(crate) fn current_code(
    record: &AnyRecord,
    code_time_ms: u64,
) -> Result<TotpCodeV1, ApplicationError> {
    let AnyRecord::TotpSeed(seed) = record else {
        return Err(ApplicationError::InvalidInput);
    };
    let algorithm = algorithm(&seed.algorithm)?;
    // `TotpAuthenticator::new` owns the remaining validation — a non-zero
    // period, a digit count it can render, a non-empty secret. Its refusal
    // becomes `Unsupported` rather than `InvalidInput` because nothing about
    // the *caller's request* was malformed; the stored record simply names
    // parameters this build has no way to honour. The window is 0 because
    // generation has exactly one current step (VLT-PM45 §4.4); a window is a
    // verifier's tolerance.
    let authenticator = TotpAuthenticator::new(
        seed.secret.clone(),
        algorithm,
        u64::from(seed.period),
        u32::from(seed.digits),
        0,
    )
    .map_err(|_| ApplicationError::Unsupported)?;
    let unix_seconds = code_time_ms / 1_000;
    let code = authenticator
        .formatted_code_at(unix_seconds)
        .map_err(|_| ApplicationError::Unsupported)?;
    let remaining = authenticator.remaining_seconds(unix_seconds);
    // `remaining` is bounded by `period`, which is a u32 in the record, so the
    // narrowing cannot lose information. It is written as a checked conversion
    // rather than an `as` cast so that a future widening of `period` turns into
    // a compile-time or test-time failure instead of a silent truncation of the
    // number a person reads off the screen.
    let remaining_seconds =
        u32::try_from(remaining).map_err(|_| ApplicationError::InternalInvariant)?;
    Ok(TotpCodeV1 {
        code,
        remaining_seconds,
        period_seconds: seed.period,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_vault_records::{Login, TotpSeed};

    /// The RFC 6238 Appendix B SHA-1 seed, as a stored record.
    fn record(algorithm: &str, digits: u8, period: u32) -> AnyRecord {
        AnyRecord::TotpSeed(TotpSeed {
            label: "GitHub ada@example.com".into(),
            issuer: Some("GitHub".into()),
            secret: b"12345678901234567890".to_vec(),
            algorithm: algorithm.into(),
            digits,
            period,
        })
    }

    /// The published Appendix B answers, reached through the record rather
    /// than through the engine directly — this is the test that the stored
    /// parameters are actually the ones used.
    #[test]
    fn computes_the_published_rfc6238_vectors_from_a_stored_record() {
        for (algorithm, secret, expected) in [
            ("SHA1", b"12345678901234567890".to_vec(), "94287082"),
            (
                "SHA256",
                b"12345678901234567890123456789012".to_vec(),
                "46119246",
            ),
            (
                "SHA512",
                b"1234567890123456789012345678901234567890123456789012345678901234".to_vec(),
                "90693936",
            ),
        ] {
            let record = AnyRecord::TotpSeed(TotpSeed {
                label: "label".into(),
                issuer: None,
                secret,
                algorithm: algorithm.into(),
                digits: 8,
                period: 30,
            });
            // T = 59 seconds, expressed in milliseconds.
            let code = current_code(&record, 59_000).unwrap();
            assert_eq!(code.code(), expected);
        }
    }

    #[test]
    fn six_digit_records_render_six_padded_digits() {
        let code = current_code(&record("SHA1", 6, 30), 1_111_111_109_000).unwrap();
        // Appendix B's eight-digit answer here is 07081804.
        assert_eq!(code.code(), "081804");
        assert_eq!(code.code().len(), 6);
    }

    /// Sub-second precision must not move the step. Every millisecond inside
    /// one second belongs to that second.
    #[test]
    fn milliseconds_floor_to_the_containing_second() {
        for milliseconds in [59_000_u64, 59_001, 59_500, 59_999] {
            let code = current_code(&record("SHA1", 8, 30), milliseconds).unwrap();
            assert_eq!(code.code(), "94287082");
        }
        // T=59 is the *last* second of step 1 (59/30 = 1), so the very next
        // second starts step 2 and must produce a different code. Flooring is
        // what puts the boundary there; rounding would have put it at 59.5 and
        // made half of every second belong to the wrong step.
        let code = current_code(&record("SHA1", 8, 30), 60_000).unwrap();
        assert_ne!(code.code(), "94287082");
        // And it is stable for the whole of the second it just entered.
        assert_eq!(
            current_code(&record("SHA1", 8, 30), 60_999).unwrap().code(),
            code.code()
        );
        // The countdown agrees: one second left at T=59, a full period at T=60.
        assert_eq!(
            current_code(&record("SHA1", 8, 30), 59_999)
                .unwrap()
                .remaining_seconds(),
            1
        );
        assert_eq!(code.remaining_seconds(), 30);
    }

    /// The code is constant across a step and changes exactly at the boundary.
    #[test]
    fn the_code_turns_over_exactly_at_the_period_boundary() {
        let boundary_ms = 1_111_111_110_000_u64; // 37_037_037 * 30 seconds
        let before = current_code(&record("SHA1", 6, 30), boundary_ms - 1_000).unwrap();
        let at = current_code(&record("SHA1", 6, 30), boundary_ms).unwrap();
        let inside = current_code(&record("SHA1", 6, 30), boundary_ms + 29_000).unwrap();
        let after = current_code(&record("SHA1", 6, 30), boundary_ms + 30_000).unwrap();
        assert_ne!(before.code(), at.code());
        assert_eq!(at.code(), inside.code());
        assert_ne!(at.code(), after.code());
    }

    /// The countdown walks the whole period and never reaches zero.
    #[test]
    fn remaining_seconds_covers_the_period_without_reaching_zero() {
        let boundary_ms = 1_111_111_110_000_u64;
        for offset in 0..30_u64 {
            let code = current_code(&record("SHA1", 6, 30), boundary_ms + offset * 1_000).unwrap();
            assert_eq!(
                code.remaining_seconds(),
                30 - u32::try_from(offset).unwrap()
            );
            assert!((1..=30).contains(&code.remaining_seconds()));
            assert_eq!(code.period_seconds(), 30);
        }
        // Wrapping into the next step restores the full window.
        let code = current_code(&record("SHA1", 6, 30), boundary_ms + 30_000).unwrap();
        assert_eq!(code.remaining_seconds(), 30);
    }

    /// A non-default period is honoured rather than replaced by 30.
    #[test]
    fn a_non_default_period_is_taken_from_the_record() {
        let code = current_code(&record("SHA1", 6, 60), 1_111_111_109_000).unwrap();
        assert_eq!(code.period_seconds(), 60);
        assert_eq!(code.remaining_seconds(), 60 - (1_111_111_109 % 60));
        // 60-second steps land on a different counter than 30-second ones, so
        // the code must differ from the 30-second answer at the same instant.
        let thirty = current_code(&record("SHA1", 6, 30), 1_111_111_109_000).unwrap();
        assert_ne!(code.code(), thirty.code());
    }

    #[test]
    fn a_non_totp_record_is_invalid_input() {
        let login = AnyRecord::Login(Login {
            title: "login".into(),
            username: "user".into(),
            password: "secret".into(),
            urls: vec![],
            notes: None,
        });
        // `TotpCodeV1` has no `Debug`, deliberately, so these assertions
        // pattern-match rather than reaching for `unwrap_err`.
        assert!(matches!(
            current_code(&login, 59_000),
            Err(ApplicationError::InvalidInput)
        ));
        let opaque = AnyRecord::Opaque {
            content_type: "future/secret/v1".into(),
            payload_bytes: vec![1, 2, 3],
        };
        assert!(matches!(
            current_code(&opaque, 59_000),
            Err(ApplicationError::InvalidInput)
        ));
    }

    /// Stored parameters this build cannot compute fail closed, and every one
    /// of them fails as `Unsupported` rather than as a silent substitution.
    #[test]
    fn uncomputable_stored_parameters_fail_closed() {
        for record in [
            record("SHA3-256", 6, 30),
            record("sha1", 6, 30),
            record("SHA-1", 6, 30),
            record("", 6, 30),
            record("SHA1", 6, 0),
            record("SHA1", 0, 30),
            record("SHA1", 3, 30),
            record("SHA1", 11, 30),
            AnyRecord::TotpSeed(TotpSeed {
                label: "label".into(),
                issuer: None,
                secret: Vec::new(),
                algorithm: "SHA1".into(),
                digits: 6,
                period: 30,
            }),
        ] {
            assert!(
                matches!(
                    current_code(&record, 59_000),
                    Err(ApplicationError::Unsupported)
                ),
                "expected a closed refusal, not a substituted parameter"
            );
        }
    }

    /// Every recognized algorithm spelling reaches a different hash.
    #[test]
    fn each_stored_algorithm_selects_a_different_hash() {
        let sha1 = current_code(&record("SHA1", 8, 30), 59_000).unwrap();
        let sha256 = current_code(&record("SHA256", 8, 30), 59_000).unwrap();
        let sha512 = current_code(&record("SHA512", 8, 30), 59_000).unwrap();
        assert_ne!(sha1.code(), sha256.code());
        assert_ne!(sha256.code(), sha512.code());
        assert_ne!(sha1.code(), sha512.code());
    }

    #[test]
    fn the_code_is_wiped_on_request() {
        let mut code = current_code(&record("SHA1", 6, 30), 59_000).unwrap();
        assert!(!code.code().is_empty());
        code.zeroize();
        assert!(code.code().is_empty());
        assert_eq!(code.remaining_seconds(), 0);
        assert_eq!(code.period_seconds(), 0);
    }
}
