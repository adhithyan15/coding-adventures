//! Exact TLS 1.3 HelloRetryRequest-to-ServerHello selection continuity.
//!
//! A [`BoundServerHelloVersionSelection`] proves one server message in
//! isolation. A retry handshake needs a second relation: the retry and final
//! selections must describe one continuous negotiation. This crate retains
//! both non-copyable proofs and compares the public fields that must remain
//! stable before later state and transcript composition may consume them. It
//! can also attach the repository-owned retry key-share transition to the
//! exact extension data retained by those four decoded messages.
//!
//! This boundary intentionally does not validate every permitted ClientHello2
//! change. Its key-share composition does not generate key material, invoke
//! key agreement, rewrite a transcript, advance connection state, perform I/O,
//! or provide TLS or HTTPS.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_extension_block::ExtensionType;
use tls13_key_share::{decode_client_key_shares, KeyShareErrorKind, KeyShareLimits};
use tls13_key_share_retry::{
    bind_retried_client_key_share, bind_retry_selected_group, bind_retry_server_key_share,
    BoundRetryKeyShare, BoundRetryServerKeyShare, RetryKeyShareError,
};
use tls13_named_group_policy::NamedGroupPolicy;
use tls13_server_hello_message::Tls13ServerHelloKind;
use tls13_server_hello_version_selection::BoundServerHelloVersionSelection;
use tls13_supported_groups::{
    decode_supported_groups, NamedGroupListError, NamedGroupListLimits,
    SUPPORTED_GROUPS_EXTENSION_TYPE,
};

/// RFC 9846 `key_share` extension type.
pub const KEY_SHARE_EXTENSION_TYPE: u16 = 51;

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

/// Stable message-composition failures retaining no public handshake payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryKeyShareMessageError {
    /// The first ClientHello omitted `supported_groups`.
    MissingInitialSupportedGroups,
    /// The first ClientHello omitted `key_share`.
    MissingInitialClientKeyShare,
    /// HelloRetryRequest omitted its selected-group `key_share`.
    MissingRetryKeyShare,
    /// The second ClientHello omitted its replacement `key_share`.
    MissingRetriedClientKeyShare,
    /// The final ServerHello omitted its selected `key_share`.
    MissingFinalServerKeyShare,
    /// The exact first-ClientHello supported-group list was malformed.
    InitialSupportedGroups(NamedGroupListError),
    /// The exact first-ClientHello key-share vector was malformed or disallowed.
    InitialClientKeyShare(KeyShareErrorKind),
    /// The exact second-ClientHello key-share vector was malformed or disallowed.
    RetriedClientKeyShare(KeyShareErrorKind),
    /// The exact retry transition failed public group-policy checks.
    RetryTransition(RetryKeyShareError),
    /// The exact final ServerHello share failed retry-offer binding.
    FinalServerShare(RetryKeyShareError),
}

impl fmt::Display for RetryKeyShareMessageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingInitialSupportedGroups => {
                "the first ClientHello is missing supported_groups"
            }
            Self::MissingInitialClientKeyShare => "the first ClientHello is missing key_share",
            Self::MissingRetryKeyShare => "HelloRetryRequest is missing key_share",
            Self::MissingRetriedClientKeyShare => "the second ClientHello is missing key_share",
            Self::MissingFinalServerKeyShare => "the final ServerHello is missing key_share",
            Self::InitialSupportedGroups(_) => {
                "the first ClientHello supported_groups value is invalid"
            }
            Self::InitialClientKeyShare(_) => "the first ClientHello key_share value is invalid",
            Self::RetriedClientKeyShare(_) => "the second ClientHello key_share value is invalid",
            Self::RetryTransition(_) => "the retained retry key-share transition is invalid",
            Self::FinalServerShare(_) => {
                "the retained final ServerHello key_share value is invalid"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RetryKeyShareMessageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InitialSupportedGroups(error) => Some(error),
            Self::RetryTransition(error) | Self::FinalServerShare(error) => Some(error),
            Self::MissingInitialSupportedGroups
            | Self::MissingInitialClientKeyShare
            | Self::MissingRetryKeyShare
            | Self::MissingRetriedClientKeyShare
            | Self::MissingFinalServerKeyShare
            | Self::InitialClientKeyShare(_)
            | Self::RetriedClientKeyShare(_) => None,
        }
    }
}

/// One retry key-share transition attached to four exact decoded messages.
///
/// The retained named-group policy is the only policy later used to decode the
/// final ServerHello share. The value intentionally derives neither `Copy` nor
/// `Clone`, keeping the public transition attached to its message evidence.
pub struct BoundRetryKeyShareMessages<
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
    selection: BoundServerHelloRetrySelection<
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
    named_group_policy: NamedGroupPolicy,
    transition: BoundRetryKeyShare<'retry_server, 'retry_client, 'retry_client, 'final_client>,
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
    BoundRetryKeyShareMessages<
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
    /// Borrow the complete retry-to-final selection proof.
    #[must_use]
    pub const fn selection(
        &self,
    ) -> &BoundServerHelloRetrySelection<
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
        &self.selection
    }

    /// Return the exact named-group policy retained for every key-share decode.
    #[must_use]
    pub const fn named_group_policy(&self) -> NamedGroupPolicy {
        self.named_group_policy
    }

    /// Borrow the exact retry key-share transition extracted from the messages.
    #[must_use]
    pub const fn transition(
        &self,
    ) -> &BoundRetryKeyShare<'retry_server, 'retry_client, 'retry_client, 'final_client> {
        &self.transition
    }
}

impl fmt::Debug for BoundRetryKeyShareMessages<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryKeyShareMessages")
            .field("selection", &"[RETAINED]")
            .field("named_group_policy", &"[POLICY-BOUND]")
            .field("transition", &"[RETAINED]")
            .finish()
    }
}

/// Extract and bind retry key-share evidence from exact retained messages.
///
/// The earlier version-selection proofs have already applied one exact
/// extension-context policy to both ClientHello/server-message pairs. This
/// function therefore performs semantic decoding only after locating types 10
/// and 51 inside those policy-authorized blocks.
pub fn bind_retry_key_share_messages<
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
    selection: BoundServerHelloRetrySelection<
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
    named_group_policy: NamedGroupPolicy,
    supported_group_limits: NamedGroupListLimits,
    key_share_limits: KeyShareLimits,
) -> Result<
    BoundRetryKeyShareMessages<
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
    RetryKeyShareMessageError,
> {
    let supported_groups_type = ExtensionType::from_wire_value(SUPPORTED_GROUPS_EXTENSION_TYPE);
    let key_share_type = ExtensionType::from_wire_value(KEY_SHARE_EXTENSION_TYPE);
    let initial_client_extensions = selection.retry().selection().client_hello().extensions();
    let retry_extensions = selection.retry().selection().server_hello().extensions();
    let retried_client_extensions = selection
        .server_hello()
        .selection()
        .client_hello()
        .extensions();

    let supported_groups_data = initial_client_extensions
        .find(supported_groups_type)
        .ok_or(RetryKeyShareMessageError::MissingInitialSupportedGroups)?
        .data();
    let initial_key_share_data = initial_client_extensions
        .find(key_share_type)
        .ok_or(RetryKeyShareMessageError::MissingInitialClientKeyShare)?
        .data();
    let retry_key_share_data = retry_extensions
        .find(key_share_type)
        .ok_or(RetryKeyShareMessageError::MissingRetryKeyShare)?
        .data();
    let retried_key_share_data = retried_client_extensions
        .find(key_share_type)
        .ok_or(RetryKeyShareMessageError::MissingRetriedClientKeyShare)?
        .data();

    let supported_groups = decode_supported_groups(supported_groups_data, supported_group_limits)
        .map_err(RetryKeyShareMessageError::InitialSupportedGroups)?;
    let initial_client_shares = decode_client_key_shares(
        initial_key_share_data,
        &named_group_policy,
        key_share_limits,
    )
    .map_err(|error| RetryKeyShareMessageError::InitialClientKeyShare(error.kind()))?;
    let retry_selection = bind_retry_selected_group(
        retry_key_share_data,
        &named_group_policy,
        supported_groups,
        initial_client_shares,
    )
    .map_err(RetryKeyShareMessageError::RetryTransition)?;
    let retried_client_shares = decode_client_key_shares(
        retried_key_share_data,
        &named_group_policy,
        key_share_limits,
    )
    .map_err(|error| RetryKeyShareMessageError::RetriedClientKeyShare(error.kind()))?;
    let transition = bind_retried_client_key_share(retry_selection, retried_client_shares)
        .map_err(RetryKeyShareMessageError::RetryTransition)?;

    Ok(BoundRetryKeyShareMessages {
        selection,
        named_group_policy,
        transition,
    })
}

/// Final ServerHello key-share evidence that remains attached to all messages.
pub struct BoundRetryServerKeyShareMessages<
    'bound,
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
    messages: &'bound BoundRetryKeyShareMessages<
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
    server_share: BoundRetryServerKeyShare<
        'final_server,
        'bound,
        'retry_server,
        'retry_client,
        'retry_client,
        'final_client,
    >,
}

impl<
        'bound,
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
    BoundRetryServerKeyShareMessages<
        'bound,
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
    /// Borrow the complete message-bound retry transition.
    #[must_use]
    pub const fn messages(
        &self,
    ) -> &'bound BoundRetryKeyShareMessages<
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
        self.messages
    }

    /// Borrow the final share decoded from the retained ordinary ServerHello.
    #[must_use]
    pub const fn server_share(
        &self,
    ) -> &BoundRetryServerKeyShare<
        'final_server,
        'bound,
        'retry_server,
        'retry_client,
        'retry_client,
        'final_client,
    > {
        &self.server_share
    }
}

impl fmt::Debug for BoundRetryServerKeyShareMessages<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundRetryServerKeyShareMessages")
            .field("messages", &"[RETAINED]")
            .field("server_share", &"[RETAINED]")
            .finish()
    }
}

/// Decode the final ServerHello share from the same retained message proof.
pub fn bind_final_server_key_share_message<
    'bound,
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
    messages: &'bound BoundRetryKeyShareMessages<
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
) -> Result<
    BoundRetryServerKeyShareMessages<
        'bound,
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
    RetryKeyShareMessageError,
> {
    let key_share_type = ExtensionType::from_wire_value(KEY_SHARE_EXTENSION_TYPE);
    let server_key_share_data = messages
        .selection
        .server_hello()
        .selection()
        .server_hello()
        .extensions()
        .find(key_share_type)
        .ok_or(RetryKeyShareMessageError::MissingFinalServerKeyShare)?
        .data();
    let server_share = bind_retry_server_key_share(
        &messages.transition,
        server_key_share_data,
        &messages.named_group_policy,
    )
    .map_err(RetryKeyShareMessageError::FinalServerShare)?;

    Ok(BoundRetryServerKeyShareMessages {
        messages,
        server_share,
    })
}
