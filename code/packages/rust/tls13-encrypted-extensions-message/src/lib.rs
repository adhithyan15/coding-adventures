//! Exact TLS 1.3 `EncryptedExtensions` message-body decoding.
//!
//! RFC 8446 section 4.3.1 defines this body as one extension vector. This
//! crate binds the exact borrowed body to the repository-owned structural
//! extension decoder under caller-selected limits. It deliberately applies no
//! extension registry or handshake-context policy.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_extension_block::{
    decode_extension_block, ExtensionBlock, ExtensionBlockError, ExtensionBlockLimits,
};

/// One exact, structurally decoded `EncryptedExtensions` handshake body.
///
/// The body and decoded extension entries share the same input lifetime. The
/// value is intentionally non-copyable so later policy can retain this exact
/// parse rather than silently substitute another body.
pub struct EncryptedExtensionsMessage<'input> {
    body: &'input [u8],
    extensions: ExtensionBlock<'input>,
}

impl<'input> EncryptedExtensionsMessage<'input> {
    /// Borrow the exact complete handshake body supplied to the decoder.
    #[must_use]
    pub const fn body(&self) -> &'input [u8] {
        self.body
    }

    /// Borrow the structurally decoded extension block in wire order.
    #[must_use]
    pub const fn extensions(&self) -> &ExtensionBlock<'input> {
        &self.extensions
    }
}

impl fmt::Debug for EncryptedExtensionsMessage<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncryptedExtensionsMessage")
            .field("extension_count", &self.extensions.len())
            .field(
                "extension_payload_length",
                &self.extensions.payload_length(),
            )
            .field("body", &"[OPAQUE]")
            .finish_non_exhaustive()
    }
}

/// Decode one exact RFC 8446 section 4.3.1 message body.
///
/// The protocol permits an empty extension vector. All structural and resource
/// failures remain the payload-free categories from `tls13-extension-block`.
/// This function does not decide whether a decoded extension is recognized or
/// legal in `EncryptedExtensions`.
pub fn decode_encrypted_extensions_message(
    body: &[u8],
    limits: ExtensionBlockLimits,
) -> Result<EncryptedExtensionsMessage<'_>, ExtensionBlockError> {
    let extensions = decode_extension_block(body, limits)?;
    Ok(EncryptedExtensionsMessage { body, extensions })
}
