//! # Bounded TLS 1.3 Certificate message decoding
//!
//! This crate decodes one already-delimited RFC 8446 `Certificate` handshake
//! body into borrowed request-context and certificate-entry slices. It applies
//! explicit caller-owned byte and entry-count ceilings before scanning.
//!
//! It does not parse X.509, interpret extensions, authenticate a certificate,
//! establish trust, process a handshake, or perform an external effect.

#![deny(unsafe_code)]

use core::fmt;

/// Caller-owned resource ceilings for one Certificate body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tls13CertificateMessageLimits {
    max_message_bytes: usize,
    max_request_context_bytes: usize,
    max_certificate_list_bytes: usize,
    max_certificate_count: usize,
    max_certificate_bytes: usize,
    max_extensions_bytes: usize,
}

impl Tls13CertificateMessageLimits {
    pub const fn new(
        max_message_bytes: usize,
        max_request_context_bytes: usize,
        max_certificate_list_bytes: usize,
        max_certificate_count: usize,
        max_certificate_bytes: usize,
        max_extensions_bytes: usize,
    ) -> Self {
        Self {
            max_message_bytes,
            max_request_context_bytes,
            max_certificate_list_bytes,
            max_certificate_count,
            max_certificate_bytes,
            max_extensions_bytes,
        }
    }
}

/// The field whose length or contents were truncated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tls13CertificateMessageField {
    RequestContext,
    CertificateListLength,
    CertificateList,
    CertificateLength,
    Certificate,
    ExtensionsLength,
    Extensions,
}

/// Stable, payload-free failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tls13CertificateMessageErrorKind {
    MessageTooLarge,
    RequestContextTooLarge,
    CertificateListTooLarge,
    TooManyCertificates,
    EmptyCertificate,
    CertificateTooLarge,
    ExtensionsTooLarge,
    Truncated(Tls13CertificateMessageField),
    TrailingData,
}

/// A redacted failure at an offset local to the Certificate body.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Tls13CertificateMessageError {
    kind: Tls13CertificateMessageErrorKind,
    offset: usize,
}

impl Tls13CertificateMessageError {
    const fn new(kind: Tls13CertificateMessageErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    pub const fn kind(self) -> Tls13CertificateMessageErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Debug for Tls13CertificateMessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tls13CertificateMessageError")
            .field("kind", &self.kind)
            .field("offset", &self.offset)
            .finish()
    }
}

impl fmt::Display for Tls13CertificateMessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS 1.3 Certificate message error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl std::error::Error for Tls13CertificateMessageError {}

/// One borrowed TLS 1.3 CertificateEntry.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Tls13CertificateEntry<'a> {
    certificate_data: &'a [u8],
    extensions: &'a [u8],
    index: usize,
}

impl fmt::Debug for Tls13CertificateEntry<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tls13CertificateEntry")
            .field("certificate_length", &self.certificate_data.len())
            .field("extensions_length", &self.extensions.len())
            .field("index", &self.index)
            .finish()
    }
}

impl<'a> Tls13CertificateEntry<'a> {
    pub const fn certificate_data(self) -> &'a [u8] {
        self.certificate_data
    }

    pub const fn extensions(self) -> &'a [u8] {
        self.extensions
    }

    pub const fn index(self) -> usize {
        self.index
    }
}

/// A fully framed, borrowed Certificate handshake body.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Tls13CertificateMessage<'a> {
    encoded: &'a [u8],
    request_context: &'a [u8],
    certificate_list: &'a [u8],
    certificate_count: usize,
}

impl fmt::Debug for Tls13CertificateMessage<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tls13CertificateMessage")
            .field("encoded_length", &self.encoded.len())
            .field("request_context_length", &self.request_context.len())
            .field("certificate_list_length", &self.certificate_list.len())
            .field("certificate_count", &self.certificate_count)
            .finish()
    }
}

impl<'a> Tls13CertificateMessage<'a> {
    pub const fn encoded(self) -> &'a [u8] {
        self.encoded
    }

    pub const fn request_context(self) -> &'a [u8] {
        self.request_context
    }

    pub const fn certificate_count(self) -> usize {
        self.certificate_count
    }

    pub const fn entries(self) -> Tls13CertificateEntries<'a> {
        Tls13CertificateEntries {
            remaining: self.certificate_list,
            index: 0,
        }
    }
}

/// Iterator over entries already validated by the decoder.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Tls13CertificateEntries<'a> {
    remaining: &'a [u8],
    index: usize,
}

impl fmt::Debug for Tls13CertificateEntries<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tls13CertificateEntries")
            .field("remaining_length", &self.remaining.len())
            .field("next_index", &self.index)
            .finish()
    }
}

impl<'a> Iterator for Tls13CertificateEntries<'a> {
    type Item = Tls13CertificateEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }
        let certificate_length = read_u24(self.remaining.get(..3)?)?;
        let certificate_start = 3usize;
        let certificate_end = certificate_start.checked_add(certificate_length)?;
        let certificate_data = self.remaining.get(certificate_start..certificate_end)?;
        let extensions_length_end = certificate_end.checked_add(2)?;
        let extensions_length =
            read_u16(self.remaining.get(certificate_end..extensions_length_end)?)?;
        let extensions_end = extensions_length_end.checked_add(extensions_length)?;
        let extensions = self.remaining.get(extensions_length_end..extensions_end)?;
        self.remaining = self.remaining.get(extensions_end..)?;
        let entry = Tls13CertificateEntry {
            certificate_data,
            extensions,
            index: self.index,
        };
        self.index += 1;
        Some(entry)
    }
}

/// Decode one exact TLS 1.3 Certificate handshake body.
///
/// Empty certificate lists and non-empty request contexts are structurally
/// valid. Handshake-role policy must decide whether either is permitted at the
/// caller's exact protocol state.
pub fn decode_tls13_certificate_message<'a>(
    encoded: &'a [u8],
    limits: Tls13CertificateMessageLimits,
) -> Result<Tls13CertificateMessage<'a>, Tls13CertificateMessageError> {
    if encoded.len() > limits.max_message_bytes {
        return Err(Tls13CertificateMessageError::new(
            Tls13CertificateMessageErrorKind::MessageTooLarge,
            0,
        ));
    }
    let context_length = *encoded.first().ok_or_else(|| {
        Tls13CertificateMessageError::new(
            Tls13CertificateMessageErrorKind::Truncated(
                Tls13CertificateMessageField::RequestContext,
            ),
            0,
        )
    })? as usize;
    if context_length > limits.max_request_context_bytes {
        return Err(Tls13CertificateMessageError::new(
            Tls13CertificateMessageErrorKind::RequestContextTooLarge,
            0,
        ));
    }
    // The one-byte wire length bounds this sum to 256.
    let context_end = 1 + context_length;
    let request_context = encoded.get(1..context_end).ok_or_else(|| {
        Tls13CertificateMessageError::new(
            Tls13CertificateMessageErrorKind::Truncated(
                Tls13CertificateMessageField::RequestContext,
            ),
            encoded.len(),
        )
    })?;
    let list_length_end = context_end + 3;
    let certificate_list_length =
        read_u24(encoded.get(context_end..list_length_end).ok_or_else(|| {
            Tls13CertificateMessageError::new(
                Tls13CertificateMessageErrorKind::Truncated(
                    Tls13CertificateMessageField::CertificateListLength,
                ),
                context_end,
            )
        })?)
        .expect("the exact three-byte slice was checked");
    if certificate_list_length > limits.max_certificate_list_bytes {
        return Err(Tls13CertificateMessageError::new(
            Tls13CertificateMessageErrorKind::CertificateListTooLarge,
            context_end,
        ));
    }
    // The three-byte vector length bounds this sum below 2^24 + 259.
    let list_end = list_length_end + certificate_list_length;
    let certificate_list = encoded.get(list_length_end..list_end).ok_or_else(|| {
        Tls13CertificateMessageError::new(
            Tls13CertificateMessageErrorKind::Truncated(
                Tls13CertificateMessageField::CertificateList,
            ),
            encoded.len(),
        )
    })?;
    if list_end != encoded.len() {
        return Err(Tls13CertificateMessageError::new(
            Tls13CertificateMessageErrorKind::TrailingData,
            list_end,
        ));
    }

    let certificate_count = validate_entries(certificate_list, list_length_end, limits)?;
    Ok(Tls13CertificateMessage {
        encoded,
        request_context,
        certificate_list,
        certificate_count,
    })
}

fn validate_entries(
    mut remaining: &[u8],
    base_offset: usize,
    limits: Tls13CertificateMessageLimits,
) -> Result<usize, Tls13CertificateMessageError> {
    let mut count = 0usize;
    let mut consumed = 0usize;
    while !remaining.is_empty() {
        let entry_offset = base_offset + consumed;
        if count >= limits.max_certificate_count {
            return Err(Tls13CertificateMessageError::new(
                Tls13CertificateMessageErrorKind::TooManyCertificates,
                entry_offset,
            ));
        }
        let certificate_length = read_u24(remaining.get(..3).ok_or_else(|| {
            Tls13CertificateMessageError::new(
                Tls13CertificateMessageErrorKind::Truncated(
                    Tls13CertificateMessageField::CertificateLength,
                ),
                entry_offset,
            )
        })?)
        .expect("the exact three-byte slice was checked");
        if certificate_length == 0 {
            return Err(Tls13CertificateMessageError::new(
                Tls13CertificateMessageErrorKind::EmptyCertificate,
                entry_offset,
            ));
        }
        if certificate_length > limits.max_certificate_bytes {
            return Err(Tls13CertificateMessageError::new(
                Tls13CertificateMessageErrorKind::CertificateTooLarge,
                entry_offset,
            ));
        }
        // Certificate and extension lengths are bounded by their wire widths.
        let certificate_end = 3 + certificate_length;
        remaining.get(3..certificate_end).ok_or_else(|| {
            Tls13CertificateMessageError::new(
                Tls13CertificateMessageErrorKind::Truncated(
                    Tls13CertificateMessageField::Certificate,
                ),
                entry_offset + 3,
            )
        })?;
        let extensions_length_end = certificate_end + 2;
        let extensions_length = read_u16(
            remaining
                .get(certificate_end..extensions_length_end)
                .ok_or_else(|| {
                    Tls13CertificateMessageError::new(
                        Tls13CertificateMessageErrorKind::Truncated(
                            Tls13CertificateMessageField::ExtensionsLength,
                        ),
                        entry_offset + certificate_end,
                    )
                })?,
        )
        .expect("the exact two-byte slice was checked");
        if extensions_length > limits.max_extensions_bytes {
            return Err(Tls13CertificateMessageError::new(
                Tls13CertificateMessageErrorKind::ExtensionsTooLarge,
                entry_offset + certificate_end,
            ));
        }
        let entry_end = extensions_length_end + extensions_length;
        remaining
            .get(extensions_length_end..entry_end)
            .ok_or_else(|| {
                Tls13CertificateMessageError::new(
                    Tls13CertificateMessageErrorKind::Truncated(
                        Tls13CertificateMessageField::Extensions,
                    ),
                    entry_offset + extensions_length_end,
                )
            })?;
        remaining = remaining
            .get(entry_end..)
            .expect("the entry end was checked");
        consumed += entry_end;
        count += 1;
    }
    Ok(count)
}

fn read_u24(bytes: &[u8]) -> Option<usize> {
    let bytes: [u8; 3] = bytes.try_into().ok()?;
    Some((usize::from(bytes[0]) << 16) | (usize::from(bytes[1]) << 8) | usize::from(bytes[2]))
}

fn read_u16(bytes: &[u8]) -> Option<usize> {
    let bytes: [u8; 2] = bytes.try_into().ok()?;
    Some((usize::from(bytes[0]) << 8) | usize::from(bytes[1]))
}
