//! Bounded TLS 1.3 no-PSK server-flight sequencing.
//!
//! After a caller has validated a no-PSK `ServerHello`, the TLS 1.3 server
//! authentication flight is `EncryptedExtensions`, an optional
//! `CertificateRequest`, `Certificate`, `CertificateVerify`, and `Finished`.
//! This crate advances only that public message-type sequence and retains
//! whether the server requested client authentication.
//!
//! It does not establish that a `ServerHello` selected no PSK, decode a body,
//! update a transcript, authenticate a message, process certificates, access
//! credentials, invoke cryptography, send an alert, protect records, or perform
//! I/O. Completion is ordering evidence only, not handshake authentication.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_handshake::Tls13HandshakeType;

/// The next expected position in a no-PSK server authentication flight.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ServerFlightState {
    /// No server-flight message has been accepted yet.
    ExpectEncryptedExtensions,
    /// Encrypted extensions were accepted; client auth may be requested once.
    ExpectCertificateRequestOrCertificate,
    /// A certificate request was accepted; the server certificate is next.
    ExpectCertificate,
    /// The server certificate was accepted; its proof is next.
    ExpectCertificateVerify,
    /// The server certificate proof was accepted; Finished is next.
    ExpectFinished,
    /// Every required server-flight message was accepted exactly once.
    Complete,
}

/// Stable payload-free no-PSK server-flight failure categories.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ServerFlightErrorKind {
    /// The received recognized type was not valid in the current state.
    UnexpectedMessage,
    /// A message was presented after the server flight completed.
    AlreadyComplete,
}

/// One failure that retains no received message type or peer bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServerFlightError {
    kind: ServerFlightErrorKind,
}

impl ServerFlightError {
    const fn new(kind: ServerFlightErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(self) -> ServerFlightErrorKind {
        self.kind
    }
}

impl fmt::Display for ServerFlightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS 1.3 no-PSK server-flight error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for ServerFlightError {}

/// Mutable ordering policy for one no-PSK server authentication flight.
#[derive(Debug, Eq, PartialEq)]
pub struct NoPskServerFlight {
    state: ServerFlightState,
    client_authentication_requested: bool,
}

impl NoPskServerFlight {
    /// Start immediately after a separately validated no-PSK `ServerHello`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: ServerFlightState::ExpectEncryptedExtensions,
            client_authentication_requested: false,
        }
    }

    /// Return the next expected server-flight position.
    #[must_use]
    pub const fn state(&self) -> ServerFlightState {
        self.state
    }

    /// Return whether one accepted `CertificateRequest` requires a client reply.
    #[must_use]
    pub const fn client_authentication_requested(&self) -> bool {
        self.client_authentication_requested
    }

    /// Return whether all required server-flight types were accepted in order.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self.state, ServerFlightState::Complete)
    }

    /// Accept one already decoded core handshake-message type.
    ///
    /// Unexpected input returns a payload-free error without changing the
    /// current state or the retained client-authentication requirement.
    pub fn accept(
        &mut self,
        received: Tls13HandshakeType,
    ) -> Result<ServerFlightState, ServerFlightError> {
        if self.is_complete() {
            return Err(ServerFlightError::new(
                ServerFlightErrorKind::AlreadyComplete,
            ));
        }

        let (next_state, client_authentication_requested) = match (self.state, received) {
            (
                ServerFlightState::ExpectEncryptedExtensions,
                Tls13HandshakeType::EncryptedExtensions,
            ) => (
                ServerFlightState::ExpectCertificateRequestOrCertificate,
                false,
            ),
            (
                ServerFlightState::ExpectCertificateRequestOrCertificate,
                Tls13HandshakeType::CertificateRequest,
            ) => (ServerFlightState::ExpectCertificate, true),
            (
                ServerFlightState::ExpectCertificateRequestOrCertificate,
                Tls13HandshakeType::Certificate,
            ) => (ServerFlightState::ExpectCertificateVerify, false),
            (ServerFlightState::ExpectCertificate, Tls13HandshakeType::Certificate) => (
                ServerFlightState::ExpectCertificateVerify,
                self.client_authentication_requested,
            ),
            (ServerFlightState::ExpectCertificateVerify, Tls13HandshakeType::CertificateVerify) => {
                (
                    ServerFlightState::ExpectFinished,
                    self.client_authentication_requested,
                )
            }
            (ServerFlightState::ExpectFinished, Tls13HandshakeType::Finished) => (
                ServerFlightState::Complete,
                self.client_authentication_requested,
            ),
            _ => {
                return Err(ServerFlightError::new(
                    ServerFlightErrorKind::UnexpectedMessage,
                ));
            }
        };

        self.state = next_state;
        self.client_authentication_requested = client_authentication_requested;
        Ok(next_state)
    }
}

impl Default for NoPskServerFlight {
    fn default() -> Self {
        Self::new()
    }
}
