//! Exact TLS 1.3 ServerHello-to-ClientHello selection binding.
//!
//! A structurally valid [`Tls13ServerHello`] is not yet an authorized server
//! selection. RFC 8446 requires its legacy session ID to echo the client's
//! exact value and its cipher suite to come from the client's offer. This
//! crate makes those relations explicit without parsing either message again.
//!
//! The caller also supplies one policy-bearing [`ClientCipherSuiteOffer`].
//! Before trusting that detached evidence, [`bind_server_hello_selection`]
//! compares its complete ordered contents with the vector retained by the
//! exact decoded ClientHello. Only then can the offer's data-driven policy
//! authorize the server's selected suite. Success retains all three inputs so
//! later composition cannot substitute independently decoded messages or a
//! different offer after this check.
//!
//! This crate does not interpret extensions, process retry state, update a
//! transcript, derive keys, perform I/O, or provide TLS or HTTPS.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_cipher_suite_offer::{CipherSuiteOfferError, ClientCipherSuiteOffer};
use tls13_cipher_suite_policy::SelectedCipherSuite;
use tls13_client_hello_message::ClientHelloMessage;
use tls13_server_hello_message::{Tls13ServerHello, Tls13ServerHelloKind};

/// Stable binding failures that retain no handshake or policy payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerHelloSelectionError {
    /// The supplied policy offer was not the exact ordered ClientHello vector.
    ClientOfferMismatch,
    /// The server did not echo the exact ClientHello legacy session ID.
    LegacySessionIdEchoMismatch,
    /// The exact client offer or its policy rejected the server's selection.
    CipherSuiteSelection(CipherSuiteOfferError),
}

impl fmt::Display for ServerHelloSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ClientOfferMismatch => "cipher-suite offer does not match the exact ClientHello",
            Self::LegacySessionIdEchoMismatch => {
                "ServerHello legacy session ID does not match the ClientHello"
            }
            Self::CipherSuiteSelection(_) => {
                "ServerHello cipher-suite selection was not authorized"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ServerHelloSelectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CipherSuiteSelection(error) => Some(error),
            Self::ClientOfferMismatch | Self::LegacySessionIdEchoMismatch => None,
        }
    }
}

/// Public negotiated selection bound to one exact ClientHello.
pub struct BoundServerHelloSelection<'client, 'offer, 'server> {
    client_hello: ClientHelloMessage<'client>,
    client_offer: ClientCipherSuiteOffer<'offer>,
    server_hello: Tls13ServerHello<'server>,
    cipher_suite: SelectedCipherSuite,
}

impl<'client, 'offer, 'server> BoundServerHelloSelection<'client, 'offer, 'server> {
    /// Borrow the exact decoded ClientHello retained by this binding.
    #[must_use]
    pub const fn client_hello(&self) -> &ClientHelloMessage<'client> {
        &self.client_hello
    }

    /// Return the exact policy-bearing offer retained by this binding.
    #[must_use]
    pub const fn client_offer(&self) -> ClientCipherSuiteOffer<'offer> {
        self.client_offer
    }

    /// Borrow the exact decoded ServerHello retained by this binding.
    #[must_use]
    pub const fn server_hello(&self) -> &Tls13ServerHello<'server> {
        &self.server_hello
    }

    /// Return whether the decoded wire message was ordinary or a retry request.
    #[must_use]
    pub const fn kind(&self) -> Tls13ServerHelloKind {
        self.server_hello.kind()
    }

    /// Return the policy-authorized suite and its complete algorithm descriptor.
    #[must_use]
    pub const fn cipher_suite(&self) -> SelectedCipherSuite {
        self.cipher_suite
    }
}

impl fmt::Debug for BoundServerHelloSelection<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundServerHelloSelection")
            .field("client_hello", &"[RETAINED]")
            .field("client_offer", &"[POLICY-BOUND]")
            .field("server_hello", &"[RETAINED]")
            .field("kind", &self.kind())
            .field("cipher_suite", &"[POLICY-BOUND]")
            .finish()
    }
}

/// Bind one decoded ServerHello selection to the exact decoded ClientHello.
///
/// The comparison is deliberately ordered and complete. Merely proving that
/// the server's identifier occurs in both inputs would permit a caller to
/// substitute a different policy-bearing offer and silently change which
/// client advertisement the policy evidence describes.
pub fn bind_server_hello_selection<'client, 'offer, 'server>(
    client_hello: ClientHelloMessage<'client>,
    client_offer: ClientCipherSuiteOffer<'offer>,
    server_hello: Tls13ServerHello<'server>,
) -> Result<BoundServerHelloSelection<'client, 'offer, 'server>, ServerHelloSelectionError> {
    if client_offer.len() != client_hello.cipher_suite_count()
        || client_hello
            .cipher_suites()
            .enumerate()
            .any(|(index, wire_id)| client_offer.get(index) != Some(wire_id))
    {
        return Err(ServerHelloSelectionError::ClientOfferMismatch);
    }

    if server_hello.session_id_echo() != client_hello.legacy_session_id() {
        return Err(ServerHelloSelectionError::LegacySessionIdEchoMismatch);
    }

    let cipher_suite = client_offer
        .validate_server_selection(server_hello.cipher_suite().wire_value())
        .map_err(ServerHelloSelectionError::CipherSuiteSelection)?;

    Ok(BoundServerHelloSelection {
        client_hello,
        client_offer,
        server_hello,
        cipher_suite,
    })
}
