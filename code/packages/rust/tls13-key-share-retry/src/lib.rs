//! # TLS 1.3 retry key-share transition evidence
//!
//! RFC 9846 permits a server to request one mutually supported key-exchange
//! group in `HelloRetryRequest`. The selected group must have appeared in the
//! first `supported_groups` list, must not have appeared in the first
//! `key_share` list, and must become the only share in the second
//! `ClientHello`.
//!
//! This crate binds those public values without allocation. It deliberately
//! does not claim that the replacement key bytes were freshly generated,
//! sequence a handshake, update a transcript, or provide TLS or HTTPS.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_key_share::DecodedClientKeyShares;
use tls13_named_group_policy::{NamedGroupPolicy, PolicyError, SelectedNamedGroup};
use tls13_supported_groups::NamedGroupList;

/// Stable retry-transition failures retaining no peer or policy payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryKeyShareError {
    /// HelloRetryRequest key-share extension data was not exactly two bytes.
    InvalidSelectedGroupLength,
    /// The selected identifier was unknown or disallowed by named-group policy.
    NamedGroupPolicy(PolicyError),
    /// The selected group was absent from the first supported-group list.
    SelectedGroupNotSupported,
    /// The selected group already had a share in the first ClientHello.
    SelectedGroupAlreadyOffered,
    /// The second ClientHello did not contain exactly one key-share entry.
    RetriedShareCount,
    /// The sole second-ClientHello share used a different group.
    RetriedGroupMismatch,
}

impl fmt::Display for RetryKeyShareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidSelectedGroupLength => "invalid HelloRetryRequest selected-group length",
            Self::NamedGroupPolicy(_) => {
                "HelloRetryRequest selected group failed named-group policy"
            }
            Self::SelectedGroupNotSupported => "HelloRetryRequest selected an unsupported group",
            Self::SelectedGroupAlreadyOffered => {
                "HelloRetryRequest selected a group already offered"
            }
            Self::RetriedShareCount => "retried ClientHello must contain exactly one key share",
            Self::RetriedGroupMismatch => {
                "retried ClientHello key-share group does not match the retry"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RetryKeyShareError {}

/// A policy-authorized retry selection bound to the exact first ClientHello
/// group evidence.
///
/// The value is deliberately neither [`Copy`] nor [`Clone`]. It consumes both
/// decoded lists so later composition cannot silently substitute either one.
pub struct RetrySelectedGroup<'retry, 'supported, 'initial> {
    extension_data: &'retry [u8],
    supported_groups: NamedGroupList<'supported>,
    initial_client_shares: DecodedClientKeyShares<'initial>,
    selected_group: SelectedNamedGroup,
}

impl fmt::Debug for RetrySelectedGroup<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RetrySelectedGroup")
    }
}

impl<'retry, 'supported, 'initial> RetrySelectedGroup<'retry, 'supported, 'initial> {
    /// Borrow the exact two-byte HelloRetryRequest extension data.
    #[must_use]
    pub const fn extension_data(&self) -> &'retry [u8] {
        self.extension_data
    }

    /// Borrow the exact first-ClientHello supported-group list.
    #[must_use]
    pub const fn supported_groups(&self) -> &NamedGroupList<'supported> {
        &self.supported_groups
    }

    /// Borrow the exact first-ClientHello key-share offer.
    #[must_use]
    pub const fn initial_client_shares(&self) -> &DecodedClientKeyShares<'initial> {
        &self.initial_client_shares
    }

    /// Return the policy-authorized requested group descriptor.
    #[must_use]
    pub const fn selected_group(&self) -> SelectedNamedGroup {
        self.selected_group
    }
}

/// Decode and bind one HelloRetryRequest selected group.
///
/// The extension data is exactly the two-byte `NamedGroup` value: unlike the
/// ClientHello list, it has no vector-length prefix. Success consumes and
/// retains the exact first-ClientHello group evidence.
pub fn bind_retry_selected_group<'retry, 'supported, 'initial>(
    extension_data: &'retry [u8],
    policy: &NamedGroupPolicy,
    supported_groups: NamedGroupList<'supported>,
    initial_client_shares: DecodedClientKeyShares<'initial>,
) -> Result<RetrySelectedGroup<'retry, 'supported, 'initial>, RetryKeyShareError> {
    let [high, low] = extension_data else {
        return Err(RetryKeyShareError::InvalidSelectedGroupLength);
    };
    let wire_group = u16::from_be_bytes([*high, *low]);
    let selected_group = policy
        .select(wire_group)
        .map_err(RetryKeyShareError::NamedGroupPolicy)?;
    if !supported_groups.contains(wire_group) {
        return Err(RetryKeyShareError::SelectedGroupNotSupported);
    }
    let selected = selected_group.parameters().group();
    if initial_client_shares
        .entries()
        .any(|entry| entry.selected_group().parameters().group() == selected)
    {
        return Err(RetryKeyShareError::SelectedGroupAlreadyOffered);
    }

    Ok(RetrySelectedGroup {
        extension_data,
        supported_groups,
        initial_client_shares,
        selected_group,
    })
}

/// Complete retry key-share evidence retaining both ClientHello offers.
///
/// This value proves only the public group transition. In particular, it does
/// not prove how the second entry's public key bytes or private scalar were
/// generated.
pub struct BoundRetryKeyShare<'retry, 'supported, 'initial, 'retried> {
    selection: RetrySelectedGroup<'retry, 'supported, 'initial>,
    retried_client_shares: DecodedClientKeyShares<'retried>,
}

impl fmt::Debug for BoundRetryKeyShare<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundRetryKeyShare")
    }
}

impl<'retry, 'supported, 'initial, 'retried>
    BoundRetryKeyShare<'retry, 'supported, 'initial, 'retried>
{
    /// Borrow the exact retained retry selection and initial evidence.
    #[must_use]
    pub const fn selection(&self) -> &RetrySelectedGroup<'retry, 'supported, 'initial> {
        &self.selection
    }

    /// Borrow the exact retained second-ClientHello key-share vector.
    #[must_use]
    pub const fn retried_client_shares(&self) -> &DecodedClientKeyShares<'retried> {
        &self.retried_client_shares
    }

    /// Return the policy-authorized group common to retry and replacement.
    #[must_use]
    pub const fn selected_group(&self) -> SelectedNamedGroup {
        self.selection.selected_group
    }
}

/// Require one second-ClientHello share for exactly the retry-selected group.
pub fn bind_retried_client_key_share<'retry, 'supported, 'initial, 'retried>(
    selection: RetrySelectedGroup<'retry, 'supported, 'initial>,
    retried_client_shares: DecodedClientKeyShares<'retried>,
) -> Result<BoundRetryKeyShare<'retry, 'supported, 'initial, 'retried>, RetryKeyShareError> {
    if retried_client_shares.len() != 1 {
        return Err(RetryKeyShareError::RetriedShareCount);
    }
    let retried_group = retried_client_shares
        .entries()
        .next()
        .expect("the exact one-entry count was checked")
        .selected_group()
        .parameters()
        .group();
    if retried_group != selection.selected_group.parameters().group() {
        return Err(RetryKeyShareError::RetriedGroupMismatch);
    }

    Ok(BoundRetryKeyShare {
        selection,
        retried_client_shares,
    })
}
