//! Exact TLS 1.3 HelloRetryRequest-to-ServerHello selection continuity.
//!
//! A [`BoundServerHelloVersionSelection`] proves one server message in
//! isolation. A retry handshake needs a second relation: the retry and final
//! selections must describe one continuous negotiation. This crate retains
//! both non-copyable proofs and compares the public fields that must remain
//! stable before later state and transcript composition may consume them.
//!
//! This boundary intentionally does not validate every permitted ClientHello2
//! change. It does not interpret retry extensions, enforce retry progress,
//! rewrite a transcript, advance connection state, perform I/O, or provide TLS
//! or HTTPS.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_server_hello_message::Tls13ServerHelloKind;
use tls13_server_hello_version_selection::BoundServerHelloVersionSelection;

/// Stable continuity failures that retain no handshake or policy payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerHelloRetrySelectionError {
    /// The first proof did not retain a HelloRetryRequest.
    ExpectedHelloRetryRequest,
    /// The second proof did not retain an ordinary ServerHello.
    ExpectedServerHello,
    /// The two retained ClientHello random values differed.
    ClientRandomChanged,
    /// The two retained ClientHello legacy session IDs differed.
    LegacySessionIdChanged,
    /// The exact cipher-suite offer or its data-driven policy differed.
    CipherSuiteOfferChanged,
    /// The exact supported-version offer differed.
    VersionOfferChanged,
    /// The retained extension-context rule table differed.
    ExtensionPolicyChanged,
    /// The policy-authorized cipher-suite descriptor differed.
    CipherSuiteSelectionChanged,
    /// The selected protocol-version descriptor differed.
    VersionSelectionChanged,
}

impl fmt::Display for ServerHelloRetrySelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ExpectedHelloRetryRequest => {
                "the first retained server message is not HelloRetryRequest"
            }
            Self::ExpectedServerHello => "the final retained server message is not ServerHello",
            Self::ClientRandomChanged => "the retained ClientHello random changed after retry",
            Self::LegacySessionIdChanged => {
                "the retained ClientHello legacy session ID changed after retry"
            }
            Self::CipherSuiteOfferChanged => "the retained cipher-suite offer changed after retry",
            Self::VersionOfferChanged => "the retained version offer changed after retry",
            Self::ExtensionPolicyChanged => {
                "the retained extension-context policy changed after retry"
            }
            Self::CipherSuiteSelectionChanged => {
                "the policy-authorized cipher suite changed after retry"
            }
            Self::VersionSelectionChanged => "the selected TLS version changed after retry",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ServerHelloRetrySelectionError {}

/// One retry and final ServerHello pair with stable public selection evidence.
pub struct BoundServerHelloRetrySelection<
    'retry_client,
    'retry_cipher_offer,
    'retry_server,
    'retry_version_offer,
    'retry_rules,
    'final_client,
    'final_cipher_offer,
    'final_server,
    'final_version_offer,
    'final_rules,
> {
    retry: BoundServerHelloVersionSelection<
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
    >,
    server_hello: BoundServerHelloVersionSelection<
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >,
}

impl<
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >
    BoundServerHelloRetrySelection<
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >
{
    /// Borrow the exact retained HelloRetryRequest selection proof.
    #[must_use]
    pub const fn retry(
        &self,
    ) -> &BoundServerHelloVersionSelection<
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
    > {
        &self.retry
    }

    /// Borrow the exact retained final ServerHello selection proof.
    #[must_use]
    pub const fn server_hello(
        &self,
    ) -> &BoundServerHelloVersionSelection<
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    > {
        &self.server_hello
    }
}

impl fmt::Debug for BoundServerHelloRetrySelection<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundServerHelloRetrySelection")
            .field("retry", &"[RETAINED]")
            .field("server_hello", &"[RETAINED]")
            .field("continuity", &"[POLICY-BOUND]")
            .finish()
    }
}

/// Bind one retained retry selection to one retained final selection.
///
/// The comparison is deliberately complete for every public continuity field
/// exposed by the two prior proofs. Extension-specific ClientHello2 changes
/// remain the responsibility of a later retry-transition policy.
pub fn bind_server_hello_retry_selection<
    'retry_client,
    'retry_cipher_offer,
    'retry_server,
    'retry_version_offer,
    'retry_rules,
    'final_client,
    'final_cipher_offer,
    'final_server,
    'final_version_offer,
    'final_rules,
>(
    retry: BoundServerHelloVersionSelection<
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
    >,
    server_hello: BoundServerHelloVersionSelection<
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >,
) -> Result<
    BoundServerHelloRetrySelection<
        'retry_client,
        'retry_cipher_offer,
        'retry_server,
        'retry_version_offer,
        'retry_rules,
        'final_client,
        'final_cipher_offer,
        'final_server,
        'final_version_offer,
        'final_rules,
    >,
    ServerHelloRetrySelectionError,
> {
    if retry.kind() != Tls13ServerHelloKind::HelloRetryRequest {
        return Err(ServerHelloRetrySelectionError::ExpectedHelloRetryRequest);
    }
    if server_hello.kind() != Tls13ServerHelloKind::ServerHello {
        return Err(ServerHelloRetrySelectionError::ExpectedServerHello);
    }

    let retry_selection = retry.selection();
    let final_selection = server_hello.selection();
    if retry_selection.client_hello().random() != final_selection.client_hello().random() {
        return Err(ServerHelloRetrySelectionError::ClientRandomChanged);
    }
    if retry_selection.client_hello().legacy_session_id()
        != final_selection.client_hello().legacy_session_id()
    {
        return Err(ServerHelloRetrySelectionError::LegacySessionIdChanged);
    }
    if retry_selection.client_offer() != final_selection.client_offer() {
        return Err(ServerHelloRetrySelectionError::CipherSuiteOfferChanged);
    }
    if retry.version_offer() != server_hello.version_offer() {
        return Err(ServerHelloRetrySelectionError::VersionOfferChanged);
    }
    if retry.extension_policy() != server_hello.extension_policy() {
        return Err(ServerHelloRetrySelectionError::ExtensionPolicyChanged);
    }
    if retry_selection.cipher_suite() != final_selection.cipher_suite() {
        return Err(ServerHelloRetrySelectionError::CipherSuiteSelectionChanged);
    }
    if retry.version() != server_hello.version() {
        return Err(ServerHelloRetrySelectionError::VersionSelectionChanged);
    }

    Ok(BoundServerHelloRetrySelection {
        retry,
        server_hello,
    })
}
