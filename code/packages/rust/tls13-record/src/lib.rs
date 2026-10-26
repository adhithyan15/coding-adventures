//! # Allocation-free TLS 1.3 outer-record framing
//!
//! TLS places a five-byte envelope around every record.  The envelope looks
//! simple, but it sits directly on a hostile byte stream: its declared length
//! controls how much input the next protocol layer receives, while its outer
//! content type and legacy version are meaningful only within a TLS state
//! machine.  This crate owns the small structural part of that boundary.
//!
//! Plaintext and ciphertext deliberately have separate entry points.  A
//! plaintext caller supplies the record types its current state permits;
//! ciphertext always uses the TLS 1.3 outer `application_data` type.  Both
//! paths enforce RFC 8446's absolute length ceilings before borrowing the
//! fragment.  No function allocates, reads from a socket, or performs
//! cryptography.
//!
//! ```
//! use tls13_record::{
//!     decode_tls13_plaintext, encode_tls13_plaintext, Tls13PlaintextPolicy,
//!     TlsContentType,
//! };
//!
//! let policy = Tls13PlaintextPolicy::new(&[TlsContentType::Handshake], 1024)?;
//! let mut encoded = [0_u8; 32];
//! let used = encode_tls13_plaintext(
//!     &policy,
//!     TlsContentType::Handshake,
//!     &[1, 0, 0, 0],
//!     &mut encoded,
//! )?;
//! let record = decode_tls13_plaintext(&policy, &encoded[..used])?;
//! assert_eq!(record.content_type(), TlsContentType::Handshake);
//! assert_eq!(record.fragment(), &[1, 0, 0, 0]);
//! # Ok::<(), tls13_record::RecordError>(())
//! ```

#![forbid(unsafe_code)]

use core::fmt;

/// Bytes in the TLS record header.
pub const TLS_RECORD_HEADER_LEN: usize = 5;

/// The TLS 1.2 legacy version value required on TLS 1.3 records.
pub const TLS13_LEGACY_RECORD_VERSION: u16 = 0x0303;

/// RFC 8446's absolute `TLSPlaintext.fragment` ceiling.
pub const TLS13_MAX_PLAINTEXT_FRAGMENT_LEN: usize = 1 << 14;

/// RFC 8446's absolute `TLSCiphertext.encrypted_record` ceiling.
pub const TLS13_MAX_CIPHERTEXT_FRAGMENT_LEN: usize = (1 << 14) + 256;

/// A recognized TLS record content type.
///
/// Recognition is intentionally not authorization.  A plaintext record must
/// also be allowed by the caller's [`Tls13PlaintextPolicy`], and a future TLS
/// state machine must decide which policy is appropriate at each epoch.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum TlsContentType {
    /// Compatibility ChangeCipherSpec record.
    ChangeCipherSpec = 20,
    /// Alert protocol record.
    Alert = 21,
    /// Handshake protocol record.
    Handshake = 22,
    /// Application-data record and the required TLS 1.3 ciphertext outer type.
    ApplicationData = 23,
}

impl TlsContentType {
    /// Return the one-byte wire value.
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    const fn policy_bit(self) -> u8 {
        match self {
            Self::ChangeCipherSpec => 1 << 0,
            Self::Alert => 1 << 1,
            Self::Handshake => 1 << 2,
            Self::ApplicationData => 1 << 3,
        }
    }

    fn from_wire(value: u8) -> Result<Self, RecordError> {
        match value {
            20 => Ok(Self::ChangeCipherSpec),
            21 => Ok(Self::Alert),
            22 => Ok(Self::Handshake),
            23 => Ok(Self::ApplicationData),
            _ => Err(RecordError::new(RecordErrorKind::UnknownContentType)),
        }
    }
}

/// Stable, payload-free failure categories for policy, decoding, and encoding.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RecordErrorKind {
    /// A plaintext policy allowed no content type.
    EmptyContentTypePolicy,
    /// A plaintext policy repeated a content type.
    DuplicateContentType,
    /// A caller fragment ceiling was zero or exceeded the RFC 8446 maximum.
    FragmentLimitOutOfRange,
    /// Fewer than five bytes were available for a record header.
    TruncatedHeader,
    /// The header carried an unrecognized content type.
    UnknownContentType,
    /// A recognized plaintext content type was not allowed by caller policy.
    DisallowedContentType,
    /// A ciphertext record did not use outer `application_data`.
    InvalidCiphertextContentType,
    /// A ciphertext record did not carry the TLS 1.3 legacy version `0x0303`.
    InvalidLegacyRecordVersion,
    /// The declared fragment exceeded caller policy.
    FragmentTooLong,
    /// The input ended before the complete declared fragment.
    TruncatedFragment,
    /// Exact decoding found bytes after the declared record.
    TrailingData,
    /// A control-plane plaintext record declared an empty fragment.
    EmptyControlFragment,
    /// A ciphertext record declared an empty encrypted fragment.
    EmptyCiphertextFragment,
    /// Caller-provided encoding storage could not hold the complete record.
    OutputTooSmall,
}

/// One redacted TLS record failure.
///
/// The error stores only a stable category.  It never retains the record,
/// fragment, declared length, content-type byte, or caller output buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecordError {
    kind: RecordErrorKind,
}

impl RecordError {
    const fn new(kind: RecordErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> RecordErrorKind {
        self.kind
    }
}

impl fmt::Display for RecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TLS 1.3 record error {:?}", self.kind)
    }
}

impl std::error::Error for RecordError {}

/// Caller-owned plaintext record policy.
///
/// Construction turns a slice into a compact set only after rejecting an
/// empty set, duplicate entries, and an invalid size ceiling.  The fields stay
/// private so all policies preserve those invariants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tls13PlaintextPolicy {
    allowed_content_types: u8,
    maximum_fragment_length: usize,
}

impl Tls13PlaintextPolicy {
    /// Validate one explicit plaintext policy.
    pub fn new(
        allowed_content_types: &[TlsContentType],
        maximum_fragment_length: usize,
    ) -> Result<Self, RecordError> {
        if allowed_content_types.is_empty() {
            return Err(RecordError::new(RecordErrorKind::EmptyContentTypePolicy));
        }
        validate_fragment_limit(maximum_fragment_length, TLS13_MAX_PLAINTEXT_FRAGMENT_LEN)?;

        let mut allowed = 0_u8;
        for &content_type in allowed_content_types {
            let bit = content_type.policy_bit();
            if allowed & bit != 0 {
                return Err(RecordError::new(RecordErrorKind::DuplicateContentType));
            }
            allowed |= bit;
        }

        Ok(Self {
            allowed_content_types: allowed,
            maximum_fragment_length,
        })
    }

    /// Return whether this policy allows one recognized content type.
    #[must_use]
    pub const fn allows(&self, content_type: TlsContentType) -> bool {
        self.allowed_content_types & content_type.policy_bit() != 0
    }

    /// Return the caller's validated fragment ceiling.
    #[must_use]
    pub const fn maximum_fragment_length(&self) -> usize {
        self.maximum_fragment_length
    }
}

/// Caller-owned ciphertext record policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tls13CiphertextPolicy {
    maximum_fragment_length: usize,
}

impl Tls13CiphertextPolicy {
    /// Validate a ciphertext fragment ceiling.
    pub fn new(maximum_fragment_length: usize) -> Result<Self, RecordError> {
        validate_fragment_limit(maximum_fragment_length, TLS13_MAX_CIPHERTEXT_FRAGMENT_LEN)?;
        Ok(Self {
            maximum_fragment_length,
        })
    }

    /// Return the caller's validated fragment ceiling.
    #[must_use]
    pub const fn maximum_fragment_length(&self) -> usize {
        self.maximum_fragment_length
    }
}

/// One validated borrowed `TLSPlaintext` record.
///
/// Private fields make the value constructible only by this crate's checked
/// decoder.  The borrowed fragment gives downstream parsers the exact bytes
/// covered by the validated outer length without copying them.
#[derive(Debug, Eq, PartialEq)]
pub struct Tls13PlaintextRecord<'a> {
    content_type: TlsContentType,
    legacy_record_version: u16,
    fragment: &'a [u8],
}

impl<'a> Tls13PlaintextRecord<'a> {
    /// Return the validated outer content type.
    #[must_use]
    pub const fn content_type(&self) -> TlsContentType {
        self.content_type
    }

    /// Return the retained, uninterpreted plaintext legacy record version.
    ///
    /// RFC 8446 requires receivers to ignore this deprecated field.  A later
    /// handshake layer may retain it for diagnostics but must not use it for
    /// version negotiation or record acceptance.
    #[must_use]
    pub const fn legacy_record_version(&self) -> u16 {
        self.legacy_record_version
    }

    /// Borrow the exact declared plaintext fragment.
    #[must_use]
    pub const fn fragment(&self) -> &'a [u8] {
        self.fragment
    }
}

/// One validated borrowed `TLSCiphertext` record.
#[derive(Debug, Eq, PartialEq)]
pub struct Tls13CiphertextRecord<'a> {
    fragment: &'a [u8],
}

impl<'a> Tls13CiphertextRecord<'a> {
    /// Return the fixed TLS 1.3 ciphertext outer content type.
    #[must_use]
    pub const fn outer_content_type(&self) -> TlsContentType {
        TlsContentType::ApplicationData
    }

    /// Return the validated TLS 1.3 legacy record version.
    #[must_use]
    pub const fn legacy_record_version(&self) -> u16 {
        TLS13_LEGACY_RECORD_VERSION
    }

    /// Borrow the exact declared encrypted fragment.
    #[must_use]
    pub const fn fragment(&self) -> &'a [u8] {
        self.fragment
    }
}

struct FramedRecord<'a> {
    raw_content_type: u8,
    legacy_record_version: u16,
    fragment: &'a [u8],
    remainder: &'a [u8],
}

fn validate_fragment_limit(maximum: usize, protocol_maximum: usize) -> Result<(), RecordError> {
    if maximum == 0 || maximum > protocol_maximum {
        return Err(RecordError::new(RecordErrorKind::FragmentLimitOutOfRange));
    }
    Ok(())
}

fn decode_record_prefix(
    input: &[u8],
    maximum_fragment_length: usize,
) -> Result<FramedRecord<'_>, RecordError> {
    let Some(header) = input.get(..TLS_RECORD_HEADER_LEN) else {
        return Err(RecordError::new(RecordErrorKind::TruncatedHeader));
    };
    let legacy_record_version = u16::from_be_bytes([header[1], header[2]]);

    let fragment_length = usize::from(u16::from_be_bytes([header[3], header[4]]));
    if fragment_length > maximum_fragment_length {
        return Err(RecordError::new(RecordErrorKind::FragmentTooLong));
    }
    let record_length = TLS_RECORD_HEADER_LEN + fragment_length;
    if input.len() < record_length {
        return Err(RecordError::new(RecordErrorKind::TruncatedFragment));
    }

    Ok(FramedRecord {
        raw_content_type: header[0],
        legacy_record_version,
        fragment: &input[TLS_RECORD_HEADER_LEN..record_length],
        remainder: &input[record_length..],
    })
}

fn require_exact(remainder: &[u8]) -> Result<(), RecordError> {
    if remainder.is_empty() {
        Ok(())
    } else {
        Err(RecordError::new(RecordErrorKind::TrailingData))
    }
}

fn validate_plaintext_fragment(
    policy: &Tls13PlaintextPolicy,
    content_type: TlsContentType,
    fragment: &[u8],
) -> Result<(), RecordError> {
    if !policy.allows(content_type) {
        return Err(RecordError::new(RecordErrorKind::DisallowedContentType));
    }
    if fragment.len() > policy.maximum_fragment_length {
        return Err(RecordError::new(RecordErrorKind::FragmentTooLong));
    }
    if fragment.is_empty() && content_type != TlsContentType::ApplicationData {
        return Err(RecordError::new(RecordErrorKind::EmptyControlFragment));
    }
    Ok(())
}

fn validate_ciphertext_fragment(
    policy: &Tls13CiphertextPolicy,
    fragment: &[u8],
) -> Result<(), RecordError> {
    if fragment.is_empty() {
        return Err(RecordError::new(RecordErrorKind::EmptyCiphertextFragment));
    }
    if fragment.len() > policy.maximum_fragment_length {
        return Err(RecordError::new(RecordErrorKind::FragmentTooLong));
    }
    Ok(())
}

/// Decode one `TLSPlaintext` record and return the unconsumed stream tail.
pub fn decode_tls13_plaintext_prefix<'a>(
    policy: &Tls13PlaintextPolicy,
    input: &'a [u8],
) -> Result<(Tls13PlaintextRecord<'a>, &'a [u8]), RecordError> {
    let framed = decode_record_prefix(input, policy.maximum_fragment_length)?;
    let content_type = TlsContentType::from_wire(framed.raw_content_type)?;
    validate_plaintext_fragment(policy, content_type, framed.fragment)?;
    Ok((
        Tls13PlaintextRecord {
            content_type,
            legacy_record_version: framed.legacy_record_version,
            fragment: framed.fragment,
        },
        framed.remainder,
    ))
}

/// Decode exactly one `TLSPlaintext` record.
pub fn decode_tls13_plaintext<'a>(
    policy: &Tls13PlaintextPolicy,
    input: &'a [u8],
) -> Result<Tls13PlaintextRecord<'a>, RecordError> {
    let (record, remainder) = decode_tls13_plaintext_prefix(policy, input)?;
    require_exact(remainder)?;
    Ok(record)
}

/// Decode one `TLSCiphertext` record and return the unconsumed stream tail.
pub fn decode_tls13_ciphertext_prefix<'a>(
    policy: &Tls13CiphertextPolicy,
    input: &'a [u8],
) -> Result<(Tls13CiphertextRecord<'a>, &'a [u8]), RecordError> {
    let framed = decode_record_prefix(input, policy.maximum_fragment_length)?;
    if framed.raw_content_type != TlsContentType::ApplicationData.wire_value() {
        return Err(RecordError::new(
            RecordErrorKind::InvalidCiphertextContentType,
        ));
    }
    if framed.legacy_record_version != TLS13_LEGACY_RECORD_VERSION {
        return Err(RecordError::new(
            RecordErrorKind::InvalidLegacyRecordVersion,
        ));
    }
    validate_ciphertext_fragment(policy, framed.fragment)?;
    Ok((
        Tls13CiphertextRecord {
            fragment: framed.fragment,
        },
        framed.remainder,
    ))
}

/// Decode exactly one `TLSCiphertext` record.
pub fn decode_tls13_ciphertext<'a>(
    policy: &Tls13CiphertextPolicy,
    input: &'a [u8],
) -> Result<Tls13CiphertextRecord<'a>, RecordError> {
    let (record, remainder) = decode_tls13_ciphertext_prefix(policy, input)?;
    require_exact(remainder)?;
    Ok(record)
}

fn encode_record(
    content_type: TlsContentType,
    fragment: &[u8],
    output: &mut [u8],
) -> Result<usize, RecordError> {
    let record_length = TLS_RECORD_HEADER_LEN + fragment.len();
    if output.len() < record_length {
        return Err(RecordError::new(RecordErrorKind::OutputTooSmall));
    }

    // Fragment lengths have already passed a protocol ceiling below `u16::MAX`.
    // Keeping the checked conversion makes that dependency explicit if a future
    // protocol limit changes.
    let fragment_length = u16::try_from(fragment.len())
        .map_err(|_| RecordError::new(RecordErrorKind::FragmentTooLong))?;
    let [version_high, version_low] = TLS13_LEGACY_RECORD_VERSION.to_be_bytes();
    let [length_high, length_low] = fragment_length.to_be_bytes();

    output[0] = content_type.wire_value();
    output[1] = version_high;
    output[2] = version_low;
    output[3] = length_high;
    output[4] = length_low;
    output[TLS_RECORD_HEADER_LEN..record_length].copy_from_slice(fragment);
    Ok(record_length)
}

/// Encode one validated `TLSPlaintext` record into caller-owned storage.
///
/// The output is not modified unless content type, fragment length, emptiness,
/// and complete output capacity have all passed validation.
pub fn encode_tls13_plaintext(
    policy: &Tls13PlaintextPolicy,
    content_type: TlsContentType,
    fragment: &[u8],
    output: &mut [u8],
) -> Result<usize, RecordError> {
    validate_plaintext_fragment(policy, content_type, fragment)?;
    encode_record(content_type, fragment, output)
}

/// Encode one validated `TLSCiphertext` record into caller-owned storage.
///
/// The output is not modified unless fragment length, non-emptiness, and
/// complete output capacity have all passed validation.
pub fn encode_tls13_ciphertext(
    policy: &Tls13CiphertextPolicy,
    fragment: &[u8],
    output: &mut [u8],
) -> Result<usize, RecordError> {
    validate_ciphertext_fragment(policy, fragment)?;
    encode_record(TlsContentType::ApplicationData, fragment, output)
}
