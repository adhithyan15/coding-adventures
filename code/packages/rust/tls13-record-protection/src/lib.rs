//! Data-driven routing for reviewed TLS 1.3 record protection.
//!
//! The retained selected suite is the only routing input. The dispatcher
//! admits the two repository-owned production record routes and rejects the
//! recognized AES-256-GCM suite before parsing, encoding, output mutation, or
//! nonce consumption:
//!
//! | Selected AEAD | Route |
//! | --- | --- |
//! | AES-128-GCM | repository-owned AES-128-GCM record composition |
//! | ChaCha20-Poly1305 | repository-owned ChaCha20-Poly1305 record composition |
//! | AES-256-GCM | fail closed; no reviewed fixed-memory route exists |
//!
//! This crate is not a TLS connector. It performs no I/O, handshake,
//! certificate, trust, revocation, HTTP, OAuth, credential, audit, or provider
//! action.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_aes128_gcm_record as aes128;
use tls13_chacha20_poly1305_record as chacha20;
use tls13_cipher_suite_policy::CipherSuite;
use tls13_inner_plaintext::Tls13InnerPlaintextPolicy;
use tls13_record::{Tls13CiphertextPolicy, TlsContentType};
use tls13_record_protection_state::RecordProtectionState;

/// Stable, payload-free dispatcher failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordProtectionErrorKind {
    /// The selected suite has no reviewed repository-owned record route.
    UnsupportedCipherSuite,
    /// The AES-128-GCM route rejected the operation.
    Aes128Gcm(aes128::RecordProtectionErrorKind),
    /// The ChaCha20-Poly1305 route rejected the operation.
    Chacha20Poly1305(chacha20::RecordProtectionErrorKind),
}

/// One redacted dispatcher failure.
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
        write!(formatter, "TLS 1.3 record routing error {:?}", self.kind)
    }
}

impl std::error::Error for RecordProtectionError {}

/// Borrowed content and protected framing requested for one sealed record.
///
/// This value deliberately does not implement `Clone` or `Copy`. Debug output
/// redacts the borrowed content.
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

enum OpenedRoute<'output> {
    Aes128Gcm(aes128::OpenedTls13Record<'output>),
    Chacha20Poly1305(chacha20::OpenedTls13Record<'output>),
}

#[derive(Clone, Copy)]
enum SelectedRoute {
    Aes128Gcm,
    Chacha20Poly1305,
}

/// Algorithm-erased authenticated cleartext retained in caller-owned storage.
///
/// This view is deliberately neither `Clone` nor `Copy`. The selected route's
/// private opened-record owner clears the exact authenticated prefix when this
/// wrapper is dropped.
pub struct OpenedTls13Record<'output> {
    route: OpenedRoute<'output>,
}

impl OpenedTls13Record<'_> {
    /// Return the authenticated protected content type.
    #[must_use]
    pub const fn content_type(&self) -> TlsContentType {
        match &self.route {
            OpenedRoute::Aes128Gcm(opened) => opened.content_type(),
            OpenedRoute::Chacha20Poly1305(opened) => opened.content_type(),
        }
    }

    /// Borrow the authenticated content, excluding type and padding.
    #[must_use]
    pub fn content(&self) -> &[u8] {
        match &self.route {
            OpenedRoute::Aes128Gcm(opened) => opened.content(),
            OpenedRoute::Chacha20Poly1305(opened) => opened.content(),
        }
    }

    /// Return the authenticated zero-padding length.
    #[must_use]
    pub const fn padding_length(&self) -> usize {
        match &self.route {
            OpenedRoute::Aes128Gcm(opened) => opened.padding_length(),
            OpenedRoute::Chacha20Poly1305(opened) => opened.padding_length(),
        }
    }
}

impl fmt::Debug for OpenedTls13Record<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenedTls13Record")
            .field("content_type", &self.content_type())
            .field("content", &"[REDACTED]")
            .field("padding_length", &self.padding_length())
            .finish()
    }
}

fn selected_route(
    state: &RecordProtectionState<'_>,
) -> Result<SelectedRoute, RecordProtectionError> {
    match state.selected_suite().parameters().suite() {
        CipherSuite::TlsAes128GcmSha256 => Ok(SelectedRoute::Aes128Gcm),
        CipherSuite::TlsChacha20Poly1305Sha256 => Ok(SelectedRoute::Chacha20Poly1305),
        CipherSuite::TlsAes256GcmSha384 => Err(RecordProtectionError::new(
            RecordProtectionErrorKind::UnsupportedCipherSuite,
        )),
    }
}

/// Seal one protected content value through the selected reviewed route.
///
/// AES-256-GCM fails before either concrete route sees caller scratch or
/// output. Admitted routes retain their reviewed preflight and nonce-
/// consumption semantics.
pub fn seal_tls13_record(
    state: &mut RecordProtectionState<'_>,
    ciphertext_policy: &Tls13CiphertextPolicy,
    inner_policy: &Tls13InnerPlaintextPolicy,
    plaintext: Tls13Plaintext<'_>,
    cleartext_scratch: &mut [u8],
    output: &mut [u8],
) -> Result<usize, RecordProtectionError> {
    match selected_route(state)? {
        SelectedRoute::Aes128Gcm => aes128::seal_tls13_aes128_gcm_record(
            state,
            ciphertext_policy,
            inner_policy,
            aes128::Tls13Plaintext::new(
                plaintext.content_type,
                plaintext.content,
                plaintext.padding_length,
            ),
            cleartext_scratch,
            output,
        )
        .map_err(|error| {
            RecordProtectionError::new(RecordProtectionErrorKind::Aes128Gcm(error.kind()))
        }),
        SelectedRoute::Chacha20Poly1305 => chacha20::seal_tls13_chacha20_record(
            state,
            ciphertext_policy,
            inner_policy,
            chacha20::Tls13Plaintext::new(
                plaintext.content_type,
                plaintext.content,
                plaintext.padding_length,
            ),
            cleartext_scratch,
            output,
        )
        .map_err(|error| {
            RecordProtectionError::new(RecordProtectionErrorKind::Chacha20Poly1305(error.kind()))
        }),
    }
}

/// Authenticate and open one record through the selected reviewed route.
///
/// AES-256-GCM fails before outer-record parsing or caller-output mutation.
/// Admitted routes preserve authentication-before-release and wipe-on-drop.
pub fn open_tls13_record<'output>(
    state: &mut RecordProtectionState<'_>,
    ciphertext_policy: &Tls13CiphertextPolicy,
    inner_policy: &Tls13InnerPlaintextPolicy,
    input: &[u8],
    output: &'output mut [u8],
) -> Result<OpenedTls13Record<'output>, RecordProtectionError> {
    let route = match selected_route(state)? {
        SelectedRoute::Aes128Gcm => OpenedRoute::Aes128Gcm(
            aes128::open_tls13_aes128_gcm_record(
                state,
                ciphertext_policy,
                inner_policy,
                input,
                output,
            )
            .map_err(|error| {
                RecordProtectionError::new(RecordProtectionErrorKind::Aes128Gcm(error.kind()))
            })?,
        ),
        SelectedRoute::Chacha20Poly1305 => OpenedRoute::Chacha20Poly1305(
            chacha20::open_tls13_chacha20_record(
                state,
                ciphertext_policy,
                inner_policy,
                input,
                output,
            )
            .map_err(|error| {
                RecordProtectionError::new(RecordProtectionErrorKind::Chacha20Poly1305(
                    error.kind(),
                ))
            })?,
        ),
    };

    Ok(OpenedTls13Record { route })
}
