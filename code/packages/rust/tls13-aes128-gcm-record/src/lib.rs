//! Bounded TLS 1.3 AES-128-GCM record protection.
//!
//! This crate composes four already reviewed boundaries without widening any
//! of them: selected-suite-bound sequence reservations, TLS inner plaintext,
//! exact outer record framing, and fixed-memory AES-128-GCM. Public framing
//! and capacity failures occur before nonce consumption. Once a nonce is
//! reserved, every result consumes it, including authentication failure.
//!
//! This is not a TLS connector. It performs no I/O, handshake processing,
//! certificate validation, trust decision, revocation check, HTTP action, or
//! OAuth effect.

#![forbid(unsafe_code)]

use coding_adventures_aes_modes::{
    aes128_gcm_decrypt_fixed, aes128_gcm_encrypt_fixed, FixedAes128GcmError,
};
use coding_adventures_zeroize::Zeroize;
use core::fmt;
use tls13_cipher_suite_policy::{AeadAlgorithm, CipherSuite};
use tls13_inner_plaintext::{
    decode_tls13_inner_plaintext, encode_tls13_inner_plaintext, InnerPlaintextErrorKind,
    Tls13InnerPlaintextPolicy,
};
use tls13_record::{
    decode_tls13_ciphertext, RecordErrorKind, Tls13CiphertextPolicy, TlsContentType,
    TLS13_LEGACY_RECORD_VERSION, TLS_RECORD_HEADER_LEN,
};
use tls13_record_protection_state::{RecordProtectionState, RecordProtectionStateError};

const AES128_KEY_LENGTH: usize = 16;
const GCM_NONCE_LENGTH: usize = 12;
const GCM_TAG_LENGTH: usize = 16;

/// Stable, payload-free record-protection failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordProtectionErrorKind {
    /// The state selected a route other than TLS_AES_128_GCM_SHA256.
    UnsupportedCipherSuite,
    /// Bound selected-suite lengths disagreed with the reviewed route.
    InvariantViolation,
    /// Outer TLS record validation failed.
    Record(RecordErrorKind),
    /// Inner TLS plaintext policy or decoding failed.
    InnerPlaintext(InnerPlaintextErrorKind),
    /// Nonce reservation or its caller-authorized ceiling failed.
    State(RecordProtectionStateError),
    /// The ciphertext fragment could not contain cleartext plus a full tag.
    CiphertextTooShort,
    /// Caller-owned output could not hold the complete result.
    OutputTooSmall,
    /// Checked record-length arithmetic overflowed.
    LengthOverflow,
    /// The record tag did not authenticate.
    AuthenticationFailed,
    /// The fixed-memory AEAD rejected a bound invariant.
    AeadFailure,
}

/// One redacted record-protection failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecordProtectionError {
    kind: RecordProtectionErrorKind,
}

impl RecordProtectionError {
    const fn new(kind: RecordProtectionErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable category without peer or secret bytes.
    #[must_use]
    pub const fn kind(self) -> RecordProtectionErrorKind {
        self.kind
    }
}

impl fmt::Display for RecordProtectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS 1.3 AES-128-GCM record error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for RecordProtectionError {}

/// Borrowed content and protected framing requested for one sealed record.
///
/// This value deliberately does not implement `Clone` or `Copy`. Its debug
/// representation omits the borrowed content.
pub struct Tls13Plaintext<'content> {
    content_type: TlsContentType,
    content: &'content [u8],
    padding_length: usize,
}

impl<'content> Tls13Plaintext<'content> {
    /// Bind content, its protected type, and requested zero padding.
    #[must_use]
    pub const fn new(
        content_type: TlsContentType,
        content: &'content [u8],
        padding_length: usize,
    ) -> Self {
        Self {
            content_type,
            content,
            padding_length,
        }
    }
}

impl fmt::Debug for Tls13Plaintext<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tls13Plaintext")
            .field("content_type", &self.content_type)
            .field("content", &"[REDACTED]")
            .field("padding_length", &self.padding_length)
            .finish()
    }
}

/// Successful authenticated cleartext retained in caller-owned storage.
///
/// This view is deliberately neither `Clone` nor `Copy`. Dropping it clears
/// the exact prefix written by the authenticated opening operation.
pub struct OpenedTls13Record<'output> {
    cleartext: &'output mut [u8],
    content_type: TlsContentType,
    content_length: usize,
    padding_length: usize,
}

impl OpenedTls13Record<'_> {
    /// Return the authenticated protected content type.
    #[must_use]
    pub const fn content_type(&self) -> TlsContentType {
        self.content_type
    }

    /// Borrow the authenticated content, excluding type and padding.
    #[must_use]
    pub fn content(&self) -> &[u8] {
        &self.cleartext[..self.content_length]
    }

    /// Return the authenticated zero-padding length.
    #[must_use]
    pub const fn padding_length(&self) -> usize {
        self.padding_length
    }
}

impl fmt::Debug for OpenedTls13Record<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenedTls13Record")
            .field("content_type", &self.content_type)
            .field("content", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl Drop for OpenedTls13Record<'_> {
    fn drop(&mut self) {
        self.cleartext.zeroize();
    }
}

struct ClearOnDrop<'a>(&'a mut [u8]);

impl Drop for ClearOnDrop<'_> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

fn ensure_aes128_gcm_route(state: &RecordProtectionState<'_>) -> Result<(), RecordProtectionError> {
    let selected = state.selected_suite();
    if selected.parameters().suite() != CipherSuite::TlsAes128GcmSha256
        || selected.parameters().aead() != AeadAlgorithm::Aes128Gcm
    {
        return Err(RecordProtectionError::new(
            RecordProtectionErrorKind::UnsupportedCipherSuite,
        ));
    }
    Ok(())
}

fn ciphertext_header(fragment_length: usize) -> Result<[u8; 5], RecordProtectionError> {
    let fragment_length = u16::try_from(fragment_length)
        .map_err(|_| RecordProtectionError::new(RecordProtectionErrorKind::LengthOverflow))?;
    let [version_high, version_low] = TLS13_LEGACY_RECORD_VERSION.to_be_bytes();
    let [length_high, length_low] = fragment_length.to_be_bytes();
    Ok([
        TlsContentType::ApplicationData.wire_value(),
        version_high,
        version_low,
        length_high,
        length_low,
    ])
}

fn map_aead_error(error: FixedAes128GcmError) -> RecordProtectionError {
    let kind = match error {
        FixedAes128GcmError::AuthenticationFailed => {
            RecordProtectionErrorKind::AuthenticationFailed
        }
        FixedAes128GcmError::OutputLengthMismatch
        | FixedAes128GcmError::LengthOverflow
        | FixedAes128GcmError::CounterExhausted => RecordProtectionErrorKind::AeadFailure,
    };
    RecordProtectionError::new(kind)
}

/// Seal one protected content value into one complete TLS 1.3 record.
///
/// Public route, inner-policy, length, and output-capacity failures precede
/// nonce consumption. After successful inner encoding, the used scratch prefix
/// is cleared on every return. A successful reservation is never rolled back.
pub fn seal_tls13_aes128_gcm_record(
    state: &mut RecordProtectionState<'_>,
    ciphertext_policy: &Tls13CiphertextPolicy,
    inner_policy: &Tls13InnerPlaintextPolicy,
    plaintext: Tls13Plaintext<'_>,
    cleartext_scratch: &mut [u8],
    output: &mut [u8],
) -> Result<usize, RecordProtectionError> {
    ensure_aes128_gcm_route(state)?;

    let cleartext_length = encode_tls13_inner_plaintext(
        inner_policy,
        plaintext.content_type,
        plaintext.content,
        plaintext.padding_length,
        cleartext_scratch,
    )
    .map_err(|error| {
        RecordProtectionError::new(RecordProtectionErrorKind::InnerPlaintext(error.kind()))
    })?;
    let cleartext = ClearOnDrop(&mut cleartext_scratch[..cleartext_length]);

    let fragment_length = cleartext_length
        .checked_add(GCM_TAG_LENGTH)
        .ok_or_else(|| RecordProtectionError::new(RecordProtectionErrorKind::LengthOverflow))?;
    if fragment_length > ciphertext_policy.maximum_fragment_length() {
        return Err(RecordProtectionError::new(
            RecordProtectionErrorKind::Record(RecordErrorKind::FragmentTooLong),
        ));
    }
    let record_length = TLS_RECORD_HEADER_LEN
        .checked_add(fragment_length)
        .ok_or_else(|| RecordProtectionError::new(RecordProtectionErrorKind::LengthOverflow))?;
    if output.len() < record_length {
        return Err(RecordProtectionError::new(
            RecordProtectionErrorKind::OutputTooSmall,
        ));
    }
    let header = ciphertext_header(fragment_length)?;

    let reservation = state
        .reserve_next()
        .map_err(|error| RecordProtectionError::new(RecordProtectionErrorKind::State(error)))?;
    if reservation.aead() != AeadAlgorithm::Aes128Gcm
        || reservation.key().len() != AES128_KEY_LENGTH
        || reservation.nonce().len() != GCM_NONCE_LENGTH
        || reservation.tag_length() != GCM_TAG_LENGTH
    {
        return Err(RecordProtectionError::new(
            RecordProtectionErrorKind::InvariantViolation,
        ));
    }
    let key: &[u8; AES128_KEY_LENGTH] = reservation
        .key()
        .try_into()
        .map_err(|_| RecordProtectionError::new(RecordProtectionErrorKind::InvariantViolation))?;
    let nonce: &[u8; GCM_NONCE_LENGTH] = reservation
        .nonce()
        .try_into()
        .map_err(|_| RecordProtectionError::new(RecordProtectionErrorKind::InvariantViolation))?;

    let record_output = &mut output[..record_length];
    let (header_output, fragment_output) = record_output.split_at_mut(TLS_RECORD_HEADER_LEN);
    let (ciphertext_output, tag_output) = fragment_output.split_at_mut(cleartext_length);
    let tag_output: &mut [u8; GCM_TAG_LENGTH] = tag_output
        .try_into()
        .map_err(|_| RecordProtectionError::new(RecordProtectionErrorKind::InvariantViolation))?;
    aes128_gcm_encrypt_fixed(
        &*cleartext.0,
        key,
        nonce,
        &header,
        ciphertext_output,
        tag_output,
    )
    .map_err(map_aead_error)?;
    header_output.copy_from_slice(&header);
    Ok(record_length)
}

/// Authenticate and open one exact TLS 1.3 ciphertext record.
///
/// Exact outer framing and output capacity are validated before reserving a
/// nonce. Authentication failure consumes the reservation while leaving output
/// unchanged. Successful plaintext remains borrowed and is cleared on drop.
pub fn open_tls13_aes128_gcm_record<'output>(
    state: &mut RecordProtectionState<'_>,
    ciphertext_policy: &Tls13CiphertextPolicy,
    inner_policy: &Tls13InnerPlaintextPolicy,
    input: &[u8],
    output: &'output mut [u8],
) -> Result<OpenedTls13Record<'output>, RecordProtectionError> {
    ensure_aes128_gcm_route(state)?;
    let record = decode_tls13_ciphertext(ciphertext_policy, input).map_err(|error| {
        RecordProtectionError::new(RecordProtectionErrorKind::Record(error.kind()))
    })?;
    let fragment = record.fragment();
    if fragment.len() <= GCM_TAG_LENGTH {
        return Err(RecordProtectionError::new(
            RecordProtectionErrorKind::CiphertextTooShort,
        ));
    }
    let cleartext_length = fragment.len() - GCM_TAG_LENGTH;
    if output.len() < cleartext_length {
        return Err(RecordProtectionError::new(
            RecordProtectionErrorKind::OutputTooSmall,
        ));
    }

    let reservation = state
        .reserve_next()
        .map_err(|error| RecordProtectionError::new(RecordProtectionErrorKind::State(error)))?;
    if reservation.aead() != AeadAlgorithm::Aes128Gcm
        || reservation.key().len() != AES128_KEY_LENGTH
        || reservation.nonce().len() != GCM_NONCE_LENGTH
        || reservation.tag_length() != GCM_TAG_LENGTH
    {
        return Err(RecordProtectionError::new(
            RecordProtectionErrorKind::InvariantViolation,
        ));
    }
    let key: &[u8; AES128_KEY_LENGTH] = reservation
        .key()
        .try_into()
        .map_err(|_| RecordProtectionError::new(RecordProtectionErrorKind::InvariantViolation))?;
    let nonce: &[u8; GCM_NONCE_LENGTH] = reservation
        .nonce()
        .try_into()
        .map_err(|_| RecordProtectionError::new(RecordProtectionErrorKind::InvariantViolation))?;

    let (ciphertext, tag) = fragment.split_at(cleartext_length);
    let tag: &[u8; GCM_TAG_LENGTH] = tag
        .try_into()
        .map_err(|_| RecordProtectionError::new(RecordProtectionErrorKind::InvariantViolation))?;
    let cleartext = &mut output[..cleartext_length];
    aes128_gcm_decrypt_fixed(
        ciphertext,
        key,
        nonce,
        &input[..TLS_RECORD_HEADER_LEN],
        tag,
        cleartext,
    )
    .map_err(map_aead_error)?;

    let decoded = match decode_tls13_inner_plaintext(inner_policy, cleartext) {
        Ok(decoded) => decoded,
        Err(error) => {
            cleartext.zeroize();
            return Err(RecordProtectionError::new(
                RecordProtectionErrorKind::InnerPlaintext(error.kind()),
            ));
        }
    };
    let content_type = decoded.content_type();
    let content_length = decoded.content().len();
    let padding_length = decoded.padding_length();

    Ok(OpenedTls13Record {
        cleartext,
        content_type,
        content_length,
        padding_length,
    })
}
