//! Exact TLS 1.3 ServerHello version and extension-context binding.
//!
//! [`tls13_server_hello_selection`] first binds one decoded ServerHello or
//! HelloRetryRequest to the exact decoded ClientHello, exact cipher-suite
//! offer, and exact session-ID echo. This crate carries that non-copyable
//! evidence through the next public-data boundary: extension placement,
//! response-offer binding, and the mandatory `supported_versions` selection.
//!
//! A caller-supplied [`ClientVersionOffer`] is policy evidence, not wire
//! evidence. Before using it, [`bind_server_hello_version_selection`] compares
//! its complete encoded ClientHello form with the extension data retained by
//! the exact decoded ClientHello. The server's exact extension data must then
//! select a version from that offer and exactly TLS 1.3.
//!
//! This crate does not compare retry and final messages, rewrite a transcript,
//! interpret key shares, derive keys, perform I/O, or provide TLS or HTTPS.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_extension_block::ExtensionType;
use tls13_extension_context_policy::{
    ExtensionContextPolicy, ExtensionContextPolicyError, ServerExtensionContext,
};
use tls13_server_hello_message::Tls13ServerHelloKind;
use tls13_server_hello_selection::BoundServerHelloSelection;
use tls13_supported_versions::{ClientVersionOffer, SelectedTls13Version, VersionError};

/// RFC 8446 `supported_versions` extension type.
pub const SUPPORTED_VERSIONS_EXTENSION_TYPE: u16 = 43;

/// Stable failures that retain no handshake, extension, version, or policy data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerHelloVersionSelectionError {
    /// The selected data-driven extension policy rejected either exact block.
    ExtensionPolicy(ExtensionContextPolicyError),
    /// The exact ClientHello omitted its required version offer extension.
    MissingClientSupportedVersions,
    /// The exact server message omitted its required version selection extension.
    MissingServerSupportedVersions,
    /// The supplied policy offer did not encode to the exact ClientHello data.
    ClientVersionOfferMismatch,
    /// The exact server extension did not select offered, supported TLS 1.3.
    VersionSelection(VersionError),
}

impl fmt::Display for ServerHelloVersionSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ExtensionPolicy(_) => {
                "TLS extension context policy rejected ServerHello evidence"
            }
            Self::MissingClientSupportedVersions => {
                "the exact ClientHello is missing supported_versions"
            }
            Self::MissingServerSupportedVersions => {
                "the exact server message is missing supported_versions"
            }
            Self::ClientVersionOfferMismatch => {
                "version offer does not match the exact ClientHello"
            }
            Self::VersionSelection(_) => "server supported_versions selection was not authorized",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ServerHelloVersionSelectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ExtensionPolicy(error) => Some(error),
            Self::VersionSelection(error) => Some(error),
            Self::MissingClientSupportedVersions
            | Self::MissingServerSupportedVersions
            | Self::ClientVersionOfferMismatch => None,
        }
    }
}

/// One exact ServerHello version selection attached to prior selection evidence.
pub struct BoundServerHelloVersionSelection<'client, 'cipher_offer, 'server, 'version_offer, 'rules>
{
    selection: BoundServerHelloSelection<'client, 'cipher_offer, 'server>,
    version_offer: ClientVersionOffer<'version_offer>,
    extension_policy: ExtensionContextPolicy<'rules>,
    version: SelectedTls13Version,
}

impl<'client, 'cipher_offer, 'server, 'version_offer, 'rules>
    BoundServerHelloVersionSelection<'client, 'cipher_offer, 'server, 'version_offer, 'rules>
{
    /// Borrow the complete earlier ClientHello-to-ServerHello selection proof.
    #[must_use]
    pub const fn selection(&self) -> &BoundServerHelloSelection<'client, 'cipher_offer, 'server> {
        &self.selection
    }

    /// Return whether the retained server message is ordinary or retry-shaped.
    #[must_use]
    pub const fn kind(&self) -> Tls13ServerHelloKind {
        self.selection.kind()
    }

    /// Return the exact ClientHello version offer proven to match retained wire data.
    #[must_use]
    pub const fn version_offer(&self) -> ClientVersionOffer<'version_offer> {
        self.version_offer
    }

    /// Return the exact caller-selected context policy used for both messages.
    #[must_use]
    pub const fn extension_policy(&self) -> ExtensionContextPolicy<'rules> {
        self.extension_policy
    }

    /// Return the selected protocol version proven to be exactly TLS 1.3.
    #[must_use]
    pub const fn version(&self) -> SelectedTls13Version {
        self.version
    }
}

impl fmt::Debug for BoundServerHelloVersionSelection<'_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundServerHelloVersionSelection")
            .field("selection", &"[RETAINED]")
            .field("kind", &self.kind())
            .field("version_offer", &"[POLICY-BOUND]")
            .field("extension_policy", &"[POLICY-BOUND]")
            .field("version", &"[POLICY-BOUND]")
            .finish()
    }
}

/// Bind exact extension and TLS 1.3 version evidence to a prior selection.
///
/// Context policy runs before semantic decoding so an extension that is
/// recognized but forbidden in this message cannot become negotiation
/// evidence. The complete detached version offer is then compared against the
/// exact ClientHello extension data before it may authorize the server value.
pub fn bind_server_hello_version_selection<
    'client,
    'cipher_offer,
    'server,
    'version_offer,
    'rules,
>(
    selection: BoundServerHelloSelection<'client, 'cipher_offer, 'server>,
    version_offer: ClientVersionOffer<'version_offer>,
    extension_policy: ExtensionContextPolicy<'rules>,
) -> Result<
    BoundServerHelloVersionSelection<'client, 'cipher_offer, 'server, 'version_offer, 'rules>,
    ServerHelloVersionSelectionError,
> {
    let client_extensions = selection.client_hello().extensions();
    let server_extensions = selection.server_hello().extensions();

    let server_context = match selection.kind() {
        Tls13ServerHelloKind::ServerHello => ServerExtensionContext::ServerHello,
        Tls13ServerHelloKind::HelloRetryRequest => ServerExtensionContext::HelloRetryRequest,
    };
    extension_policy
        .validate_server_message(server_context, server_extensions, client_extensions)
        .map_err(ServerHelloVersionSelectionError::ExtensionPolicy)?;

    let supported_versions = ExtensionType::from_wire_value(SUPPORTED_VERSIONS_EXTENSION_TYPE);
    let client_data = client_extensions
        .find(supported_versions)
        .ok_or(ServerHelloVersionSelectionError::MissingClientSupportedVersions)?
        .data();
    let server_data = server_extensions
        .find(supported_versions)
        .ok_or(ServerHelloVersionSelectionError::MissingServerSupportedVersions)?
        .data();

    if !offer_matches_client_data(version_offer, client_data) {
        return Err(ServerHelloVersionSelectionError::ClientVersionOfferMismatch);
    }

    let version = version_offer
        .validate_server_selection(server_data)
        .map_err(ServerHelloVersionSelectionError::VersionSelection)?;

    Ok(BoundServerHelloVersionSelection {
        selection,
        version_offer,
        extension_policy,
        version,
    })
}

fn offer_matches_client_data(offer: ClientVersionOffer<'_>, data: &[u8]) -> bool {
    if data.len() != offer.encoded_len() || data.first().copied() != Some((offer.len() * 2) as u8) {
        return false;
    }

    let (pairs, remainder) = data[1..].as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    pairs
        .iter()
        .enumerate()
        .all(|(index, bytes)| offer.get(index) == Some(u16::from_be_bytes([bytes[0], bytes[1]])))
}
