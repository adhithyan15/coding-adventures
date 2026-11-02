//! # Bounded TLS 1.3 key-share framing
//!
//! This crate validates and frames public TLS 1.3 `KeyShareEntry` values under
//! the repository-owned named-group policy. Client shares are retained without
//! allocation, and one ServerHello share must select a group from the exact
//! retained client offer.
//!
//! It performs no key agreement, entropy access, secret handling, transcript
//! update, handshake processing, certificate validation, I/O, or networking.

#![deny(unsafe_code)]

use core::fmt;
use tls13_named_group_policy::{
    NamedGroup, NamedGroupPolicy, PolicyError, SelectedNamedGroup, SUPPORTED_NAMED_GROUP_COUNT,
};

/// Maximum retained client key shares, matching the reviewed group registry.
pub const MAX_KEY_SHARE_ENTRIES: usize = SUPPORTED_NAMED_GROUP_COUNT;

/// Stable limit-construction failures retaining no caller values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyShareLimitsErrorKind {
    /// The caller supplied a zero entry ceiling.
    ZeroMaximumEntries,
    /// The caller ceiling exceeds the reviewed named-group registry.
    MaximumEntriesTooLarge,
}

/// A redacted key-share limit failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct KeyShareLimitsError {
    kind: KeyShareLimitsErrorKind,
}

impl KeyShareLimitsError {
    const fn new(kind: KeyShareLimitsErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> KeyShareLimitsErrorKind {
        self.kind
    }
}

impl fmt::Debug for KeyShareLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KeyShareLimitsError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for KeyShareLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TLS 1.3 key-share limits error {:?}", self.kind)
    }
}

impl std::error::Error for KeyShareLimitsError {}

/// Validated caller-owned work limits for ClientHello decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyShareLimits {
    maximum_entries: usize,
}

impl KeyShareLimits {
    /// Validate a nonzero entry ceiling within the reviewed group registry.
    pub const fn new(maximum_entries: usize) -> Result<Self, KeyShareLimitsError> {
        if maximum_entries == 0 {
            return Err(KeyShareLimitsError::new(
                KeyShareLimitsErrorKind::ZeroMaximumEntries,
            ));
        }
        if maximum_entries > MAX_KEY_SHARE_ENTRIES {
            return Err(KeyShareLimitsError::new(
                KeyShareLimitsErrorKind::MaximumEntriesTooLarge,
            ));
        }
        Ok(Self { maximum_entries })
    }

    /// Return the validated retained-entry ceiling.
    #[must_use]
    pub const fn maximum_entries(self) -> usize {
        self.maximum_entries
    }
}

/// Stable key-share failures retaining no group, key, length, or output data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyShareErrorKind {
    /// ClientHello extension data is missing its vector-length field.
    TruncatedClientVectorLength,
    /// The declared ClientHello vector length does not consume the exact input.
    ClientVectorLengthMismatch,
    /// The reviewed client profile requires at least one key share.
    EmptyClientShareList,
    /// The caller or wire input exceeds the retained-entry ceiling.
    TooManyClientShares,
    /// A key-share entry is missing its complete group and length header.
    TruncatedEntryHeader,
    /// A key-exchange length does not consume the exact entry bytes.
    KeyExchangeLengthMismatch,
    /// One ClientHello list repeats a named group.
    DuplicateNamedGroup,
    /// Named-group selection or exact-length validation failed.
    NamedGroupPolicy(PolicyError),
    /// The complete ClientHello vector length cannot be represented on wire.
    EncodedLengthOverflow,
    /// Caller-owned output cannot hold the complete encoded extension data.
    OutputTooSmall,
    /// ServerHello selected a group absent from the exact retained client offer.
    ServerGroupNotOffered,
}

/// A redacted TLS 1.3 key-share failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct KeyShareError {
    kind: KeyShareErrorKind,
}

impl KeyShareError {
    const fn new(kind: KeyShareErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> KeyShareErrorKind {
        self.kind
    }
}

impl fmt::Debug for KeyShareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KeyShareError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for KeyShareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TLS 1.3 key-share error {:?}", self.kind)
    }
}

impl std::error::Error for KeyShareError {}

/// One policy-authorized public TLS key-share entry.
///
/// The value is deliberately neither `Copy` nor `Clone`; it binds the selected
/// group metadata to the exact caller-borrowed public key-exchange bytes.
pub struct ClientKeyShareEntry<'key_exchange> {
    selected_group: SelectedNamedGroup,
    key_exchange: &'key_exchange [u8],
}

impl ClientKeyShareEntry<'_> {
    /// Return the exact policy-selected group metadata.
    #[must_use]
    pub const fn selected_group(&self) -> SelectedNamedGroup {
        self.selected_group
    }

    /// Return the exact public key-exchange bytes validated for that group.
    #[must_use]
    pub const fn key_exchange(&self) -> &[u8] {
        self.key_exchange
    }
}

impl fmt::Debug for ClientKeyShareEntry<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClientKeyShareEntry")
    }
}

/// Validate one client key share under exact named-group policy.
pub fn validate_client_key_share<'key_exchange>(
    policy: &NamedGroupPolicy,
    wire_group: u16,
    key_exchange: &'key_exchange [u8],
) -> Result<ClientKeyShareEntry<'key_exchange>, KeyShareError> {
    let selected_group = policy
        .select(wire_group)
        .map_err(|error| KeyShareError::new(KeyShareErrorKind::NamedGroupPolicy(error)))?;
    selected_group
        .validate_key_exchange_length(key_exchange.len())
        .map_err(|error| KeyShareError::new(KeyShareErrorKind::NamedGroupPolicy(error)))?;
    Ok(ClientKeyShareEntry {
        selected_group,
        key_exchange,
    })
}

/// Exact encoded ClientHello `key_share` extension data.
pub struct EncodedClientKeyShares<'output> {
    bytes: &'output [u8],
}

impl EncodedClientKeyShares<'_> {
    /// Return the exact two-byte-length-prefixed extension data.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        self.bytes
    }
}

impl fmt::Debug for EncodedClientKeyShares<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EncodedClientKeyShares")
    }
}

/// Encode a nonempty, duplicate-free ClientHello key-share list.
///
/// Complete list, wire-length, and output-capacity checks finish before output
/// changes. Extra output capacity remains untouched.
pub fn encode_client_key_shares<'output>(
    entries: &[ClientKeyShareEntry<'_>],
    output: &'output mut [u8],
) -> Result<EncodedClientKeyShares<'output>, KeyShareError> {
    validate_entry_collection(entries)?;

    let mut payload_length = 0usize;
    for entry in entries {
        payload_length = payload_length
            .checked_add(4)
            .and_then(|length| length.checked_add(entry.key_exchange.len()))
            .ok_or_else(|| KeyShareError::new(KeyShareErrorKind::EncodedLengthOverflow))?;
    }
    let wire_payload_length = u16::try_from(payload_length)
        .map_err(|_| KeyShareError::new(KeyShareErrorKind::EncodedLengthOverflow))?;
    let encoded_length = payload_length
        .checked_add(2)
        .ok_or_else(|| KeyShareError::new(KeyShareErrorKind::EncodedLengthOverflow))?;
    if output.len() < encoded_length {
        return Err(KeyShareError::new(KeyShareErrorKind::OutputTooSmall));
    }

    output[..2].copy_from_slice(&wire_payload_length.to_be_bytes());
    let mut cursor = 2;
    for entry in entries {
        let parameters = entry.selected_group.parameters();
        output[cursor..cursor + 2].copy_from_slice(&parameters.group().wire_id().to_be_bytes());
        let key_length = u16::try_from(entry.key_exchange.len())
            .expect("reviewed named-group key-exchange length fits u16");
        output[cursor + 2..cursor + 4].copy_from_slice(&key_length.to_be_bytes());
        cursor += 4;
        let end = cursor + entry.key_exchange.len();
        output[cursor..end].copy_from_slice(entry.key_exchange);
        cursor = end;
    }
    Ok(EncodedClientKeyShares {
        bytes: &output[..encoded_length],
    })
}

/// Decoded ClientHello key shares bound to policy-selected groups.
///
/// The value is deliberately neither `Copy` nor `Clone` and retains the exact
/// public bytes supplied by the peer.
pub struct DecodedClientKeyShares<'input> {
    entries: [Option<ClientKeyShareEntry<'input>>; MAX_KEY_SHARE_ENTRIES],
    entry_count: usize,
}

impl DecodedClientKeyShares<'_> {
    /// Return the number of retained entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entry_count
    }

    /// Return whether no entries were retained.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entry_count == 0
    }

    /// Iterate over retained entries in wire order.
    pub fn entries(&self) -> impl Iterator<Item = &ClientKeyShareEntry<'_>> {
        self.entries[..self.entry_count]
            .iter()
            .map(|entry| entry.as_ref().expect("retained prefix is initialized"))
    }

    fn contains_group(&self, group: NamedGroup) -> bool {
        self.entries()
            .any(|entry| entry.selected_group.parameters().group() == group)
    }
}

impl fmt::Debug for DecodedClientKeyShares<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DecodedClientKeyShares")
            .field("entry_count", &self.entry_count)
            .finish()
    }
}

/// Decode exact ClientHello `key_share` extension data.
pub fn decode_client_key_shares<'input>(
    input: &'input [u8],
    policy: &NamedGroupPolicy,
    limits: KeyShareLimits,
) -> Result<DecodedClientKeyShares<'input>, KeyShareError> {
    if input.len() < 2 {
        return Err(KeyShareError::new(
            KeyShareErrorKind::TruncatedClientVectorLength,
        ));
    }
    let declared_length = u16::from_be_bytes([input[0], input[1]]) as usize;
    if declared_length != input.len() - 2 {
        return Err(KeyShareError::new(
            KeyShareErrorKind::ClientVectorLengthMismatch,
        ));
    }
    if declared_length == 0 {
        return Err(KeyShareError::new(KeyShareErrorKind::EmptyClientShareList));
    }

    let mut decoded = DecodedClientKeyShares {
        entries: core::array::from_fn(|_| None),
        entry_count: 0,
    };
    let mut cursor = 2;
    while cursor < input.len() {
        if decoded.entry_count == limits.maximum_entries {
            return Err(KeyShareError::new(KeyShareErrorKind::TooManyClientShares));
        }
        if input.len() - cursor < 4 {
            return Err(KeyShareError::new(KeyShareErrorKind::TruncatedEntryHeader));
        }
        let wire_group = u16::from_be_bytes([input[cursor], input[cursor + 1]]);
        let key_length = u16::from_be_bytes([input[cursor + 2], input[cursor + 3]]) as usize;
        cursor += 4;
        let end = cursor
            .checked_add(key_length)
            .ok_or_else(|| KeyShareError::new(KeyShareErrorKind::KeyExchangeLengthMismatch))?;
        if end > input.len() {
            return Err(KeyShareError::new(
                KeyShareErrorKind::KeyExchangeLengthMismatch,
            ));
        }
        let entry = validate_client_key_share(policy, wire_group, &input[cursor..end])?;
        let group = entry.selected_group.parameters().group();
        if decoded.contains_group(group) {
            return Err(KeyShareError::new(KeyShareErrorKind::DuplicateNamedGroup));
        }
        decoded.entries[decoded.entry_count] = Some(entry);
        decoded.entry_count += 1;
        cursor = end;
    }
    Ok(decoded)
}

/// One exact ServerHello key share bound to the retained client offer.
pub struct DecodedServerKeyShare<'input, 'offer, 'client_input> {
    entry: ClientKeyShareEntry<'input>,
    offered_client_shares: &'offer DecodedClientKeyShares<'client_input>,
}

impl DecodedServerKeyShare<'_, '_, '_> {
    /// Return the exact policy-selected server group metadata.
    #[must_use]
    pub const fn selected_group(&self) -> SelectedNamedGroup {
        self.entry.selected_group
    }

    /// Return the exact public server key-exchange bytes.
    #[must_use]
    pub const fn key_exchange(&self) -> &[u8] {
        self.entry.key_exchange
    }

    /// Return the exact retained client offer used for selection binding.
    #[must_use]
    pub const fn offered_client_shares(&self) -> &DecodedClientKeyShares<'_> {
        self.offered_client_shares
    }
}

impl fmt::Debug for DecodedServerKeyShare<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DecodedServerKeyShare")
    }
}

/// Decode one exact ServerHello `KeyShareEntry` and bind it to the client offer.
pub fn decode_server_key_share<'input, 'offer, 'client_input>(
    input: &'input [u8],
    policy: &NamedGroupPolicy,
    offered_client_shares: &'offer DecodedClientKeyShares<'client_input>,
) -> Result<DecodedServerKeyShare<'input, 'offer, 'client_input>, KeyShareError> {
    if input.len() < 4 {
        return Err(KeyShareError::new(KeyShareErrorKind::TruncatedEntryHeader));
    }
    let wire_group = u16::from_be_bytes([input[0], input[1]]);
    let key_length = u16::from_be_bytes([input[2], input[3]]) as usize;
    if key_length != input.len() - 4 {
        return Err(KeyShareError::new(
            KeyShareErrorKind::KeyExchangeLengthMismatch,
        ));
    }
    let entry = validate_client_key_share(policy, wire_group, &input[4..])?;
    if !offered_client_shares.contains_group(entry.selected_group.parameters().group()) {
        return Err(KeyShareError::new(KeyShareErrorKind::ServerGroupNotOffered));
    }
    Ok(DecodedServerKeyShare {
        entry,
        offered_client_shares,
    })
}

fn validate_entry_collection(entries: &[ClientKeyShareEntry<'_>]) -> Result<(), KeyShareError> {
    if entries.is_empty() {
        return Err(KeyShareError::new(KeyShareErrorKind::EmptyClientShareList));
    }
    if entries.len() > MAX_KEY_SHARE_ENTRIES {
        return Err(KeyShareError::new(KeyShareErrorKind::TooManyClientShares));
    }
    for (index, entry) in entries.iter().enumerate() {
        let group = entry.selected_group.parameters().group();
        if entries[..index]
            .iter()
            .any(|prior| prior.selected_group.parameters().group() == group)
        {
            return Err(KeyShareError::new(KeyShareErrorKind::DuplicateNamedGroup));
        }
    }
    Ok(())
}
