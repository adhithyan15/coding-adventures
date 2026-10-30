//! Bounded TLS 1.3 `KeyShareEntry` framing.
//!
//! [`KeyShareEntry`] owns no key bytes. It borrows one non-empty key-exchange
//! payload, preserves its exact two-byte named-group identifier, and validates
//! the payload against a caller-supplied [`KeyShareLimits`] value. Unknown and
//! reserved group identifiers remain data for a later group policy.
//!
//! Encoding and prefix decoding implement only the RFC 8446 structure shared
//! by ClientHello and ServerHello key-share extension data. This crate does not
//! build an extension vector, authorize a named group, generate a secret,
//! perform key agreement, validate a public key, process a handshake, perform
//! I/O, or provide TLS or HTTPS.

#![forbid(unsafe_code)]

use core::fmt;

/// Bytes before the key-exchange payload in one encoded entry.
pub const KEY_SHARE_ENTRY_HEADER_LENGTH: usize = 4;

/// Largest key-exchange payload representable by the RFC 8446 structure.
pub const MAX_KEY_EXCHANGE_LENGTH: usize = u16::MAX as usize;

/// Largest complete entry representable by the RFC 8446 structure.
pub const MAX_ENCODED_KEY_SHARE_ENTRY_LENGTH: usize =
    KEY_SHARE_ENTRY_HEADER_LENGTH + MAX_KEY_EXCHANGE_LENGTH;

/// Stable failures that retain no named-group identifier or key bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyShareEntryError {
    /// A configured maximum was zero or exceeded the wire-format maximum.
    InvalidMaximumKeyExchangeLength,
    /// The key-exchange payload or declared payload length was zero.
    EmptyKeyExchange,
    /// The payload exceeded the caller's reviewed maximum.
    KeyExchangeTooLong,
    /// Caller-owned output could not hold the complete encoded entry.
    OutputTooSmall,
    /// Input ended before the group and length fields were complete.
    TruncatedHeader,
    /// Input ended before the complete declared payload was available.
    TruncatedKeyExchange,
}

impl fmt::Display for KeyShareEntryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidMaximumKeyExchangeLength => "invalid maximum TLS key-share payload length",
            Self::EmptyKeyExchange => "empty TLS key-share payload",
            Self::KeyExchangeTooLong => "TLS key-share payload exceeds the configured limit",
            Self::OutputTooSmall => "output is too small for the TLS key-share entry",
            Self::TruncatedHeader => "truncated TLS key-share entry header",
            Self::TruncatedKeyExchange => "truncated TLS key-share payload",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for KeyShareEntryError {}

/// Caller-selected public work and buffering bound for one entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyShareLimits {
    max_key_exchange_length: u16,
}

impl KeyShareLimits {
    /// Construct a reviewed non-zero maximum that fits the wire format.
    pub fn new(max_key_exchange_length: usize) -> Result<Self, KeyShareEntryError> {
        let max_key_exchange_length = u16::try_from(max_key_exchange_length)
            .map_err(|_| KeyShareEntryError::InvalidMaximumKeyExchangeLength)?;
        if max_key_exchange_length == 0 {
            return Err(KeyShareEntryError::InvalidMaximumKeyExchangeLength);
        }
        Ok(Self {
            max_key_exchange_length,
        })
    }

    /// Return the caller-selected maximum payload length.
    #[must_use]
    pub const fn max_key_exchange_length(self) -> usize {
        self.max_key_exchange_length as usize
    }
}

/// One validated borrowed TLS 1.3 key-share entry.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct KeyShareEntry<'input> {
    group: u16,
    key_exchange: &'input [u8],
}

impl<'input> KeyShareEntry<'input> {
    /// Validate one exact named-group identifier and borrowed payload.
    pub fn new(
        group: u16,
        key_exchange: &'input [u8],
        limits: KeyShareLimits,
    ) -> Result<Self, KeyShareEntryError> {
        validate_key_exchange_length(key_exchange.len(), limits)?;
        Ok(Self {
            group,
            key_exchange,
        })
    }

    /// Decode one complete entry prefix and return the unconsumed input.
    ///
    /// An over-policy declared length is rejected before payload availability
    /// is checked, so a caller never needs to buffer bytes beyond its limit.
    pub fn decode_prefix(
        input: &'input [u8],
        limits: KeyShareLimits,
    ) -> Result<(Self, &'input [u8]), KeyShareEntryError> {
        if input.len() < KEY_SHARE_ENTRY_HEADER_LENGTH {
            return Err(KeyShareEntryError::TruncatedHeader);
        }

        let group = u16::from_be_bytes([input[0], input[1]]);
        let key_exchange_length = usize::from(u16::from_be_bytes([input[2], input[3]]));
        validate_key_exchange_length(key_exchange_length, limits)?;

        let encoded_length = KEY_SHARE_ENTRY_HEADER_LENGTH + key_exchange_length;
        if input.len() < encoded_length {
            return Err(KeyShareEntryError::TruncatedKeyExchange);
        }
        let (encoded, remainder) = input.split_at(encoded_length);
        Ok((
            Self {
                group,
                key_exchange: &encoded[KEY_SHARE_ENTRY_HEADER_LENGTH..],
            },
            remainder,
        ))
    }

    /// Return the exact two-byte named-group identifier.
    #[must_use]
    pub const fn group(self) -> u16 {
        self.group
    }

    /// Return the exact borrowed key-exchange payload.
    #[must_use]
    pub const fn key_exchange(self) -> &'input [u8] {
        self.key_exchange
    }

    /// Return the complete encoded entry length.
    #[must_use]
    pub const fn encoded_len(self) -> usize {
        KEY_SHARE_ENTRY_HEADER_LENGTH + self.key_exchange.len()
    }

    /// Encode the complete entry into caller-owned output.
    ///
    /// Capacity is validated before any output byte changes. Bytes after the
    /// returned length remain untouched.
    pub fn encode(self, output: &mut [u8]) -> Result<usize, KeyShareEntryError> {
        let encoded_len = self.encoded_len();
        if output.len() < encoded_len {
            return Err(KeyShareEntryError::OutputTooSmall);
        }

        output[..2].copy_from_slice(&self.group.to_be_bytes());
        output[2..4].copy_from_slice(&(self.key_exchange.len() as u16).to_be_bytes());
        output[4..encoded_len].copy_from_slice(self.key_exchange);
        Ok(encoded_len)
    }
}

impl fmt::Debug for KeyShareEntry<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KeyShareEntry")
            .field("group", &"[OPAQUE GROUP ID]")
            .field("key_exchange_length", &self.key_exchange.len())
            .field("key_exchange", &"[OPAQUE PUBLIC KEY DATA]")
            .finish_non_exhaustive()
    }
}

fn validate_key_exchange_length(
    key_exchange_length: usize,
    limits: KeyShareLimits,
) -> Result<(), KeyShareEntryError> {
    if key_exchange_length == 0 {
        return Err(KeyShareEntryError::EmptyKeyExchange);
    }
    if key_exchange_length > limits.max_key_exchange_length() {
        return Err(KeyShareEntryError::KeyExchangeTooLong);
    }
    Ok(())
}
