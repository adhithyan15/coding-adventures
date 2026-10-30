//! Bounded TLS Application-Layer Protocol Negotiation extension data.
//!
//! RFC 7301 represents a client ALPN offer as a two-byte-length-prefixed list
//! of opaque, non-empty, one-byte-length-prefixed protocol names. The server
//! uses the same list syntax but must select exactly one offered protocol.
//! This crate keeps those framing and binding rules together while leaving
//! protocol enablement and handshake placement to caller policy.

#![forbid(unsafe_code)]

use core::fmt;

/// The TLS extension type for `application_layer_protocol_negotiation`.
pub const ALPN_EXTENSION_TYPE: u16 = 16;

/// Maximum protocol-name length from RFC 7301's `ProtocolName` syntax.
pub const MAX_ALPN_PROTOCOL_NAME_LENGTH: usize = u8::MAX as usize;

/// Fixed work bound supported by this package.
pub const MAX_ALPN_PROTOCOL_COUNT: usize = 64;

/// Maximum ALPN extension-data length admitted by the TLS extension envelope.
pub const MAX_ALPN_EXTENSION_DATA_LENGTH: usize = u16::MAX as usize;

/// Stable, payload-free ALPN framing and policy failures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AlpnError {
    /// A caller-selected decode count ceiling was zero or exceeded the package bound.
    InvalidProtocolCountLimit,
    /// A client offer or decoded protocol list was empty.
    EmptyProtocolList,
    /// The list exceeded the selected protocol-count ceiling.
    TooManyProtocols,
    /// One protocol name was empty.
    EmptyProtocolName,
    /// One configured protocol name exceeded 255 bytes.
    ProtocolNameTooLong,
    /// The same opaque protocol name appeared more than once.
    DuplicateProtocol,
    /// The complete extension data cannot fit the TLS extension envelope.
    ExtensionDataTooLong,
    /// The encoded list was truncated, had trailing bytes, or had a false length.
    MalformedProtocolList,
    /// Caller-owned output could not hold the complete encoded offer.
    OutputTooSmall,
    /// Server extension data did not contain exactly one well-formed protocol.
    MalformedServerSelection,
    /// The server selected a protocol absent from the original client offer.
    UnofferedServerSelection,
}

impl fmt::Display for AlpnError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidProtocolCountLimit => "invalid ALPN protocol-count limit",
            Self::EmptyProtocolList => "empty ALPN protocol list",
            Self::TooManyProtocols => "ALPN protocol list exceeds its count limit",
            Self::EmptyProtocolName => "empty ALPN protocol name",
            Self::ProtocolNameTooLong => "ALPN protocol name exceeds 255 bytes",
            Self::DuplicateProtocol => "duplicate ALPN protocol name",
            Self::ExtensionDataTooLong => "ALPN extension data exceeds the TLS limit",
            Self::MalformedProtocolList => "malformed ALPN protocol list",
            Self::OutputTooSmall => "output is too small for the ALPN protocol list",
            Self::MalformedServerSelection => "malformed ALPN server selection",
            Self::UnofferedServerSelection => "server selected an unoffered ALPN protocol",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for AlpnError {}

/// One validated caller-configured client preference list.
///
/// Protocol identifiers remain opaque byte strings. The order is the caller's
/// preference order and is preserved exactly during encoding.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ClientAlpnOffer<'a> {
    protocols: &'a [&'a [u8]],
    encoded_len: usize,
}

impl fmt::Debug for ClientAlpnOffer<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientAlpnOffer")
            .field("protocol_count", &self.protocols.len())
            .finish_non_exhaustive()
    }
}

impl<'a> ClientAlpnOffer<'a> {
    /// Validate one bounded, duplicate-free client preference list.
    pub fn new(protocols: &'a [&'a [u8]]) -> Result<Self, AlpnError> {
        if protocols.is_empty() {
            return Err(AlpnError::EmptyProtocolList);
        }
        if protocols.len() > MAX_ALPN_PROTOCOL_COUNT {
            return Err(AlpnError::TooManyProtocols);
        }

        let mut vector_len = 0_usize;
        for (index, protocol) in protocols.iter().copied().enumerate() {
            validate_protocol_name(protocol)?;
            if protocols[..index].contains(&protocol) {
                return Err(AlpnError::DuplicateProtocol);
            }
            vector_len += 1 + protocol.len();
        }
        let encoded_len = vector_len + 2;
        debug_assert!(encoded_len <= MAX_ALPN_EXTENSION_DATA_LENGTH);

        Ok(Self {
            protocols,
            encoded_len,
        })
    }

    /// Return the number of offered protocols.
    #[must_use]
    pub const fn len(self) -> usize {
        self.protocols.len()
    }

    /// Return whether this validated offer is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.protocols.is_empty()
    }

    /// Return the exact opaque protocol at one preference index.
    #[must_use]
    pub fn get(self, index: usize) -> Option<&'a [u8]> {
        self.protocols.get(index).copied()
    }

    /// Return whether the exact opaque protocol was offered.
    #[must_use]
    pub fn contains(self, protocol: &[u8]) -> bool {
        self.protocols.contains(&protocol)
    }

    /// Return the complete encoded extension-data length.
    #[must_use]
    pub const fn encoded_len(self) -> usize {
        self.encoded_len
    }

    /// Encode the complete ClientHello ALPN extension data.
    ///
    /// Capacity is checked before output changes, making failure atomic.
    pub fn encode(self, output: &mut [u8]) -> Result<usize, AlpnError> {
        if output.len() < self.encoded_len {
            return Err(AlpnError::OutputTooSmall);
        }

        let vector_len = self.encoded_len - 2;
        output[..2].copy_from_slice(&(vector_len as u16).to_be_bytes());
        let mut cursor = 2;
        for protocol in self.protocols.iter().copied() {
            output[cursor] = protocol.len() as u8;
            cursor += 1;
            output[cursor..cursor + protocol.len()].copy_from_slice(protocol);
            cursor += protocol.len();
        }
        debug_assert_eq!(cursor, self.encoded_len);
        Ok(self.encoded_len)
    }

    /// Validate one server-selected protocol against this exact offer.
    ///
    /// All framing failures are collapsed to `MalformedServerSelection` so no
    /// partial server value or count is exposed. A well-formed but unoffered
    /// selection remains distinguishable for protocol-state handling.
    pub fn validate_server_selection<'selection>(
        self,
        extension_data: &'selection [u8],
    ) -> Result<SelectedAlpnProtocol<'selection>, AlpnError> {
        let selection = DecodedAlpnProtocolList::decode(extension_data, 1)
            .map_err(|_| AlpnError::MalformedServerSelection)?;
        let protocol = selection
            .get(0)
            .ok_or(AlpnError::MalformedServerSelection)?;
        if !self.contains(protocol) {
            return Err(AlpnError::UnofferedServerSelection);
        }
        Ok(SelectedAlpnProtocol { protocol })
    }
}

/// One exact validated encoded protocol-name list.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct DecodedAlpnProtocolList<'a> {
    protocol_bytes: &'a [u8],
    protocol_count: usize,
}

impl fmt::Debug for DecodedAlpnProtocolList<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DecodedAlpnProtocolList")
            .field("protocol_count", &self.protocol_count)
            .finish_non_exhaustive()
    }
}

impl<'a> DecodedAlpnProtocolList<'a> {
    /// Decode exact extension data under one caller-selected work ceiling.
    pub fn decode(
        extension_data: &'a [u8],
        maximum_protocol_count: usize,
    ) -> Result<Self, AlpnError> {
        if maximum_protocol_count == 0 || maximum_protocol_count > MAX_ALPN_PROTOCOL_COUNT {
            return Err(AlpnError::InvalidProtocolCountLimit);
        }
        if extension_data.len() > MAX_ALPN_EXTENSION_DATA_LENGTH {
            return Err(AlpnError::ExtensionDataTooLong);
        }
        let [length_high, length_low, protocol_bytes @ ..] = extension_data else {
            return Err(AlpnError::MalformedProtocolList);
        };
        let declared_len = usize::from(u16::from_be_bytes([*length_high, *length_low]));
        if declared_len != protocol_bytes.len() {
            return Err(AlpnError::MalformedProtocolList);
        }
        if protocol_bytes.is_empty() {
            return Err(AlpnError::EmptyProtocolList);
        }

        let mut cursor = 0_usize;
        let mut protocol_count = 0_usize;
        while cursor < protocol_bytes.len() {
            let protocol_start = cursor;
            let name_len = usize::from(protocol_bytes[cursor]);
            cursor += 1;
            if name_len == 0 {
                return Err(AlpnError::EmptyProtocolName);
            }
            let name_end = cursor
                .checked_add(name_len)
                .ok_or(AlpnError::MalformedProtocolList)?;
            let Some(protocol) = protocol_bytes.get(cursor..name_end) else {
                return Err(AlpnError::MalformedProtocolList);
            };
            protocol_count += 1;
            if protocol_count > maximum_protocol_count {
                return Err(AlpnError::TooManyProtocols);
            }
            if encoded_prefix_contains(&protocol_bytes[..protocol_start], protocol) {
                return Err(AlpnError::DuplicateProtocol);
            }
            cursor = name_end;
        }

        Ok(Self {
            protocol_bytes,
            protocol_count,
        })
    }

    /// Return the number of decoded protocol names.
    #[must_use]
    pub const fn len(self) -> usize {
        self.protocol_count
    }

    /// Return whether this validated list is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.protocol_count == 0
    }

    /// Return the exact opaque protocol at one preference index.
    #[must_use]
    pub fn get(self, index: usize) -> Option<&'a [u8]> {
        self.iter().nth(index)
    }

    /// Iterate over exact opaque protocol names in wire order.
    pub fn iter(self) -> AlpnProtocolIter<'a> {
        AlpnProtocolIter {
            remaining: self.protocol_bytes,
        }
    }
}

/// Iterator over a previously validated ALPN protocol-name list.
pub struct AlpnProtocolIter<'a> {
    remaining: &'a [u8],
}

impl<'a> Iterator for AlpnProtocolIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let (&length, tail) = self.remaining.split_first()?;
        let length = usize::from(length);
        let (protocol, remaining) = tail.split_at(length);
        self.remaining = remaining;
        Some(protocol)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.remaining.len() / 2))
    }
}

impl fmt::Debug for AlpnProtocolIter<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AlpnProtocolIter")
    }
}

/// One server selection proven to be well formed and present in the offer.
#[derive(Eq, PartialEq)]
pub struct SelectedAlpnProtocol<'a> {
    protocol: &'a [u8],
}

impl<'a> SelectedAlpnProtocol<'a> {
    /// Return the exact opaque server-selected protocol bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.protocol
    }
}

impl fmt::Debug for SelectedAlpnProtocol<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SelectedAlpnProtocol")
    }
}

fn validate_protocol_name(protocol: &[u8]) -> Result<(), AlpnError> {
    if protocol.is_empty() {
        return Err(AlpnError::EmptyProtocolName);
    }
    if protocol.len() > MAX_ALPN_PROTOCOL_NAME_LENGTH {
        return Err(AlpnError::ProtocolNameTooLong);
    }
    Ok(())
}

fn encoded_prefix_contains(mut encoded: &[u8], expected: &[u8]) -> bool {
    while let Some((&length, tail)) = encoded.split_first() {
        let length = usize::from(length);
        let (protocol, remaining) = tail.split_at(length);
        if protocol == expected {
            return true;
        }
        encoded = remaining;
    }
    false
}
