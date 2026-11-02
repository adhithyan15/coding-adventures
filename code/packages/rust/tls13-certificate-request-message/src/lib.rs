//! Bounded TLS 1.3 `CertificateRequest` body decoding.
//!
//! A `CertificateRequest` body contains one length-prefixed opaque request
//! context followed by one generic TLS extension block. This crate validates
//! that exact structural framing under caller-owned limits and retains both
//! components as borrowed data for later handshake policy.
//!
//! It deliberately does not decide whether a context is valid in the current
//! handshake state, require or decode any extension, authenticate a peer,
//! access credentials, sign, update a transcript, emit alerts, or provide TLS
//! or HTTPS.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_extension_block::{
    decode_extension_block, ExtensionBlock, ExtensionBlockError, ExtensionBlockLimits,
};

/// Largest request context representable by TLS's one-byte vector length.
pub const MAX_CERTIFICATE_REQUEST_CONTEXT_LENGTH: usize = u8::MAX as usize;

/// Caller-owned resource limits for one `CertificateRequest` body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CertificateRequestLimits {
    max_context_length: u8,
    extension_limits: ExtensionBlockLimits,
}

impl CertificateRequestLimits {
    /// Validate a context-byte ceiling and retain reviewed extension limits.
    pub fn new(
        max_context_length: usize,
        extension_limits: ExtensionBlockLimits,
    ) -> Result<Self, CertificateRequestError> {
        if max_context_length > MAX_CERTIFICATE_REQUEST_CONTEXT_LENGTH {
            return Err(CertificateRequestError::new(
                CertificateRequestErrorKind::InvalidLimits,
            ));
        }

        Ok(Self {
            max_context_length: max_context_length as u8,
            extension_limits,
        })
    }

    /// Return the maximum accepted request-context length.
    #[must_use]
    pub const fn max_context_length(self) -> usize {
        self.max_context_length as usize
    }

    /// Return the limits delegated to generic extension-block decoding.
    #[must_use]
    pub const fn extension_limits(self) -> ExtensionBlockLimits {
        self.extension_limits
    }
}

/// Stable payload-free failure categories for request-body decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificateRequestErrorKind {
    /// A caller-owned context limit exceeded the one-byte wire width.
    InvalidLimits,
    /// The one-byte request-context length was absent.
    MissingContextLength,
    /// The declared request context exceeded the caller-owned byte ceiling.
    ContextTooLong,
    /// The declared request context was not present in full.
    TruncatedContext,
    /// The structurally required extension vector contained no entries.
    EmptyExtensions,
    /// The exact remaining bytes failed generic extension-block decoding.
    Extensions(ExtensionBlockError),
}

/// One payload-free `CertificateRequest` decoding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CertificateRequestError {
    kind: CertificateRequestErrorKind,
}

impl CertificateRequestError {
    const fn new(kind: CertificateRequestErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category without peer-supplied values.
    #[must_use]
    pub const fn kind(self) -> CertificateRequestErrorKind {
        self.kind
    }
}

impl fmt::Display for CertificateRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS 1.3 CertificateRequest error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for CertificateRequestError {}

/// One structurally decoded TLS 1.3 `CertificateRequest` body.
pub struct CertificateRequest<'input> {
    encoded_body: &'input [u8],
    context: &'input [u8],
    extensions: ExtensionBlock<'input>,
}

impl<'input> CertificateRequest<'input> {
    /// Borrow the complete exact body supplied to the decoder.
    #[must_use]
    pub const fn encoded_body(&self) -> &'input [u8] {
        self.encoded_body
    }

    /// Borrow the complete opaque certificate-request context.
    #[must_use]
    pub const fn context(&self) -> &'input [u8] {
        self.context
    }

    /// Borrow the ordered, duplicate-free generic extension block.
    #[must_use]
    pub const fn extensions(&self) -> &ExtensionBlock<'input> {
        &self.extensions
    }
}

impl fmt::Debug for CertificateRequest<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CertificateRequest")
            .field("body_length", &self.encoded_body.len())
            .field("context_length", &self.context.len())
            .field("context", &"[OPAQUE]")
            .field("extensions", &self.extensions)
            .finish()
    }
}

/// Decode one exact TLS 1.3 `CertificateRequest` body.
///
/// The first byte declares the opaque request-context length. The decoder
/// checks that value against the caller ceiling before borrowing the context,
/// then delegates every remaining byte to [`decode_extension_block`]. Context
/// semantics and extension-specific rules remain the caller's responsibility.
pub fn decode_certificate_request(
    input: &[u8],
    limits: CertificateRequestLimits,
) -> Result<CertificateRequest<'_>, CertificateRequestError> {
    let Some(&declared_context_length) = input.first() else {
        return Err(CertificateRequestError::new(
            CertificateRequestErrorKind::MissingContextLength,
        ));
    };
    let context_length = declared_context_length as usize;
    if context_length > limits.max_context_length() {
        return Err(CertificateRequestError::new(
            CertificateRequestErrorKind::ContextTooLong,
        ));
    }

    let context_end = 1 + context_length;
    if input.len() < context_end {
        return Err(CertificateRequestError::new(
            CertificateRequestErrorKind::TruncatedContext,
        ));
    }

    let context = &input[1..context_end];
    let extensions = decode_extension_block(&input[context_end..], limits.extension_limits())
        .map_err(|error| {
            CertificateRequestError::new(CertificateRequestErrorKind::Extensions(error))
        })?;
    if extensions.is_empty() {
        return Err(CertificateRequestError::new(
            CertificateRequestErrorKind::EmptyExtensions,
        ));
    }

    Ok(CertificateRequest {
        encoded_body: input,
        context,
        extensions,
    })
}
