//! Bounded TLS 1.3 client cipher-suite offer framing and selection binding.
//!
//! [`ClientCipherSuiteOffer`] preserves a caller-owned preference-ordered list
//! of exact two-byte wire identifiers, including explicitly configured unknown
//! values used by reserved-value or GREASE policy. Construction bounds the
//! list, rejects duplicates, and proves that at least one entry is authorized
//! by a repository-owned [`CipherSuitePolicy`].
//!
//! Encoding writes the RFC 8446 two-byte-length-prefixed vector into
//! caller-owned storage only after validating complete capacity. Server
//! selection first has to be present in the original offer and then has to pass
//! the same cipher-suite policy, returning its bound algorithm and record-limit
//! descriptor.
//!
//! This crate does not construct a `ClientHello`, select a suite for the peer,
//! implement cryptography, process a handshake, perform I/O, or provide TLS or
//! HTTPS.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_cipher_suite_policy::{CipherSuitePolicy, PolicyError, SelectedCipherSuite};

/// Reviewed ceiling for entries retained by one client offer.
///
/// The TLS wire vector can represent more entries, but this package narrows the
/// caller-controlled work bound to keep duplicate checks small and predictable.
pub const MAX_CLIENT_CIPHER_SUITE_OFFER_COUNT: usize = 64;

/// Largest encoded offer produced by this package, including the length field.
pub const MAX_ENCODED_CLIENT_CIPHER_SUITE_OFFER_LENGTH: usize =
    2 + MAX_CLIENT_CIPHER_SUITE_OFFER_COUNT * 2;

/// Stable failures that retain no offered or selected wire identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CipherSuiteOfferError {
    /// The caller supplied no cipher-suite identifiers.
    EmptyOffer,
    /// The caller exceeded the reviewed entry-count ceiling.
    TooManyCipherSuites,
    /// The same exact wire identifier appeared more than once.
    DuplicateCipherSuite,
    /// No offered identifier was authorized by the supplied suite policy.
    NoPolicyAuthorizedCipherSuite,
    /// Caller-owned output could not hold the complete encoded offer.
    OutputTooSmall,
    /// The server selected an identifier absent from the original offer.
    UnofferedServerSelection,
    /// The offered server selection was rejected by the suite policy.
    PolicyRejectedServerSelection(PolicyError),
}

impl fmt::Display for CipherSuiteOfferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyOffer => "empty TLS cipher-suite offer",
            Self::TooManyCipherSuites => "TLS cipher-suite offer exceeds the entry limit",
            Self::DuplicateCipherSuite => "duplicate TLS cipher suite in client offer",
            Self::NoPolicyAuthorizedCipherSuite => {
                "TLS cipher-suite offer contains no policy-authorized suite"
            }
            Self::OutputTooSmall => "output is too small for the TLS cipher-suite offer",
            Self::UnofferedServerSelection => "server selected an unoffered TLS cipher suite",
            Self::PolicyRejectedServerSelection(_) => {
                "server-selected TLS cipher suite was rejected by policy"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CipherSuiteOfferError {}

/// One validated borrowed client cipher-suite preference list.
///
/// Wire identifiers remain caller-owned data. Unknown entries are retained in
/// order, while the copied policy proves that at least one entry is usable and
/// later binds a server selection to the same authorization.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ClientCipherSuiteOffer<'input> {
    wire_ids: &'input [u16],
    policy: CipherSuitePolicy,
}

impl<'input> ClientCipherSuiteOffer<'input> {
    /// Validate one bounded, duplicate-free offer against a suite policy.
    pub fn new(
        wire_ids: &'input [u16],
        policy: CipherSuitePolicy,
    ) -> Result<Self, CipherSuiteOfferError> {
        if wire_ids.is_empty() {
            return Err(CipherSuiteOfferError::EmptyOffer);
        }
        if wire_ids.len() > MAX_CLIENT_CIPHER_SUITE_OFFER_COUNT {
            return Err(CipherSuiteOfferError::TooManyCipherSuites);
        }

        let mut has_authorized_suite = false;
        for (index, wire_id) in wire_ids.iter().copied().enumerate() {
            if wire_ids[..index].contains(&wire_id) {
                return Err(CipherSuiteOfferError::DuplicateCipherSuite);
            }
            has_authorized_suite |= policy.select(wire_id).is_ok();
        }
        if !has_authorized_suite {
            return Err(CipherSuiteOfferError::NoPolicyAuthorizedCipherSuite);
        }

        Ok(Self { wire_ids, policy })
    }

    /// Return the number of retained preference entries.
    #[must_use]
    pub const fn len(self) -> usize {
        self.wire_ids.len()
    }

    /// Return whether the offer is empty.
    ///
    /// Validated offers are never empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.wire_ids.is_empty()
    }

    /// Return the exact identifier at one preference index.
    #[must_use]
    pub fn get(self, index: usize) -> Option<u16> {
        self.wire_ids.get(index).copied()
    }

    /// Return whether one exact identifier was offered.
    #[must_use]
    pub fn contains(self, wire_id: u16) -> bool {
        self.wire_ids.contains(&wire_id)
    }

    /// Return the complete encoded vector length, including its two-byte prefix.
    #[must_use]
    pub const fn encoded_len(self) -> usize {
        2 + self.wire_ids.len() * 2
    }

    /// Encode the complete ClientHello cipher-suite vector.
    ///
    /// Capacity is validated before any output byte changes, so failure is
    /// atomic. Bytes after the returned length are not modified.
    pub fn encode(self, output: &mut [u8]) -> Result<usize, CipherSuiteOfferError> {
        let encoded_len = self.encoded_len();
        if output.len() < encoded_len {
            return Err(CipherSuiteOfferError::OutputTooSmall);
        }

        let vector_length = (self.wire_ids.len() * 2) as u16;
        output[..2].copy_from_slice(&vector_length.to_be_bytes());
        let (slots, remainder) = output[2..encoded_len].as_chunks_mut::<2>();
        debug_assert!(remainder.is_empty());
        for (slot, wire_id) in slots.iter_mut().zip(self.wire_ids.iter().copied()) {
            slot.copy_from_slice(&wire_id.to_be_bytes());
        }
        Ok(encoded_len)
    }

    /// Bind one server-selected identifier to this offer and its policy.
    pub fn validate_server_selection(
        self,
        wire_id: u16,
    ) -> Result<SelectedCipherSuite, CipherSuiteOfferError> {
        if !self.contains(wire_id) {
            return Err(CipherSuiteOfferError::UnofferedServerSelection);
        }
        self.policy
            .select(wire_id)
            .map_err(CipherSuiteOfferError::PolicyRejectedServerSelection)
    }
}

impl fmt::Debug for ClientCipherSuiteOffer<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientCipherSuiteOffer")
            .field("cipher_suite_count", &self.wire_ids.len())
            .field("wire_ids", &"[OPAQUE POLICY DATA]")
            .finish_non_exhaustive()
    }
}
