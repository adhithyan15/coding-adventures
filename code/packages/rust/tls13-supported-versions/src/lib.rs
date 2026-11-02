//! Bounded TLS 1.3 `supported_versions` framing and selection policy.
//!
//! TLS 1.3 moves version negotiation out of the legacy ClientHello and
//! ServerHello version fields and into extension type 43. A client sends a
//! one-byte-length-prefixed preference list, while a server sends exactly one
//! selected two-byte version. Keeping those two wire shapes separate prevents
//! a caller from accidentally interpreting one in the other context.
//!
//! This crate is deliberately smaller than a handshake implementation. It
//! receives only one extension's already-delimited data and neither locates
//! the extension nor decides whether its surrounding handshake message is
//! legal.
//!
//! ```
//! use tls13_supported_versions::{ClientVersionOffer, TLS13_VERSION};
//!
//! let configured = [TLS13_VERSION];
//! let offer = ClientVersionOffer::new(&configured)?;
//! let mut encoded = [0_u8; 3];
//!
//! assert_eq!(offer.encode(&mut encoded)?, 3);
//! assert_eq!(encoded, [2, 0x03, 0x04]);
//! assert_eq!(
//!     offer.validate_server_selection(&[0x03, 0x04])?.wire_id(),
//!     TLS13_VERSION,
//! );
//! # Ok::<(), tls13_supported_versions::VersionError>(())
//! ```

#![forbid(unsafe_code)]

use core::fmt;

/// The IANA extension type for `supported_versions`.
pub const SUPPORTED_VERSIONS_EXTENSION_TYPE: u16 = 43;

/// TLS 1.3's protocol version wire identifier.
pub const TLS13_VERSION: u16 = 0x0304;

/// RFC 8446's maximum number of two-byte versions in the ClientHello vector.
pub const MAX_CLIENT_VERSION_COUNT: usize = 127;

/// The maximum encoded ClientHello extension-data length, including its
/// one-byte vector-length field.
pub const MAX_CLIENT_EXTENSION_DATA_LENGTH: usize = 255;

/// Stable, payload-free version framing and policy failures.
///
/// No variant retains a caller's version identifiers, input bytes, output
/// capacity, or configured preference order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VersionError {
    /// A client offer contained no versions.
    EmptyOffer,
    /// The offer exceeded RFC 8446's 254-byte vector ceiling.
    TooManyVersions,
    /// The same version identifier appeared more than once.
    DuplicateVersion,
    /// A TLS 1.3 client offer omitted TLS 1.3.
    MissingTls13,
    /// Caller-owned output could not hold the complete encoded offer.
    OutputTooSmall,
    /// Server extension data was not exactly one two-byte version.
    MalformedServerSelection,
    /// The server selected a version absent from the original client offer.
    UnofferedServerSelection,
    /// The server selected a version earlier than TLS 1.3.
    ServerSelectedPriorVersion,
    /// The server selected a later version this TLS 1.3 package cannot use.
    UnsupportedServerSelection,
}

impl fmt::Display for VersionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyOffer => "empty TLS version offer",
            Self::TooManyVersions => "TLS version offer exceeds the vector limit",
            Self::DuplicateVersion => "duplicate TLS version in offer",
            Self::MissingTls13 => "TLS 1.3 is absent from the version offer",
            Self::OutputTooSmall => "output is too small for the TLS version offer",
            Self::MalformedServerSelection => "malformed TLS server version selection",
            Self::UnofferedServerSelection => "server selected an unoffered TLS version",
            Self::ServerSelectedPriorVersion => "server selected a version before TLS 1.3",
            Self::UnsupportedServerSelection => "server selected an unsupported TLS version",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for VersionError {}

/// One validated, borrowed ClientHello version preference list.
///
/// The caller owns the slice and its ordering. Construction bounds all later
/// work to at most 127 entries, rejects duplicates, and proves that TLS 1.3 is
/// among the versions the caller is prepared to negotiate. Unknown values are
/// retained because RFC 8446 requires peers to ignore unknown versions.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ClientVersionOffer<'a> {
    versions: &'a [u16],
}

impl fmt::Debug for ClientVersionOffer<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientVersionOffer")
            .field("version_count", &self.versions.len())
            .field("contains_tls13", &self.contains(TLS13_VERSION))
            .finish_non_exhaustive()
    }
}

impl<'a> ClientVersionOffer<'a> {
    /// Validate a bounded, duplicate-free preference list containing TLS 1.3.
    pub fn new(versions: &'a [u16]) -> Result<Self, VersionError> {
        if versions.is_empty() {
            return Err(VersionError::EmptyOffer);
        }
        if versions.len() > MAX_CLIENT_VERSION_COUNT {
            return Err(VersionError::TooManyVersions);
        }

        let mut contains_tls13 = false;
        for (index, version) in versions.iter().copied().enumerate() {
            if version == TLS13_VERSION {
                contains_tls13 = true;
            }
            if versions[..index].contains(&version) {
                return Err(VersionError::DuplicateVersion);
            }
        }
        if !contains_tls13 {
            return Err(VersionError::MissingTls13);
        }

        Ok(Self { versions })
    }

    /// Return the number of configured versions.
    #[must_use]
    pub const fn len(self) -> usize {
        self.versions.len()
    }

    /// Return whether this offer is empty.
    ///
    /// Validated offers are never empty, so this is always false for a value
    /// constructed by [`Self::new`].
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.versions.is_empty()
    }

    /// Return whether a version is present in the offer.
    #[must_use]
    pub fn contains(self, version: u16) -> bool {
        self.versions.contains(&version)
    }

    /// Return the version at one preference index.
    #[must_use]
    pub fn get(self, index: usize) -> Option<u16> {
        self.versions.get(index).copied()
    }

    /// Return the complete encoded ClientHello extension-data length.
    #[must_use]
    pub const fn encoded_len(self) -> usize {
        1 + self.versions.len() * 2
    }

    /// Encode the ClientHello extension data into caller-owned output.
    ///
    /// The first byte is the two-byte version vector length. Capacity is
    /// checked before the output is changed, so failure is atomic.
    pub fn encode(self, output: &mut [u8]) -> Result<usize, VersionError> {
        let encoded_len = self.encoded_len();
        if output.len() < encoded_len {
            return Err(VersionError::OutputTooSmall);
        }

        let vector_len = self.versions.len() * 2;
        output[0] = vector_len as u8;
        let (slots, remainder) = output[1..encoded_len].as_chunks_mut::<2>();
        debug_assert!(remainder.is_empty());
        for (slot, version) in slots.iter_mut().zip(self.versions.iter().copied()) {
            slot.copy_from_slice(&version.to_be_bytes());
        }
        Ok(encoded_len)
    }

    /// Validate one ServerHello or HelloRetryRequest selected version.
    ///
    /// RFC 8446 gives both messages the same two-byte encoding. This method
    /// first binds the response to the original offer, then rejects versions
    /// before TLS 1.3 and later versions whose semantics this crate does not
    /// implement.
    pub fn validate_server_selection(
        self,
        extension_data: &[u8],
    ) -> Result<SelectedTls13Version, VersionError> {
        let [major, minor] = extension_data else {
            return Err(VersionError::MalformedServerSelection);
        };
        let selected = u16::from_be_bytes([*major, *minor]);
        if !self.contains(selected) {
            return Err(VersionError::UnofferedServerSelection);
        }
        if selected < TLS13_VERSION {
            return Err(VersionError::ServerSelectedPriorVersion);
        }
        if selected != TLS13_VERSION {
            return Err(VersionError::UnsupportedServerSelection);
        }
        Ok(SelectedTls13Version { wire_id: selected })
    }
}

/// A server version selection proven to be offered and exactly TLS 1.3.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SelectedTls13Version {
    wire_id: u16,
}

impl SelectedTls13Version {
    /// Return the selected protocol-version wire identifier.
    #[must_use]
    pub const fn wire_id(self) -> u16 {
        self.wire_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_preserves_preference_order_and_unknown_values() {
        let versions = [0x7a7a, TLS13_VERSION, 0x0303];
        let offer = ClientVersionOffer::new(&versions).expect("offer is valid");

        assert_eq!(offer.len(), 3);
        assert!(!offer.is_empty());
        assert_eq!(offer.get(0), Some(0x7a7a));
        assert_eq!(offer.get(1), Some(TLS13_VERSION));
        assert_eq!(offer.get(2), Some(0x0303));
        assert_eq!(offer.get(3), None);
        assert!(offer.contains(TLS13_VERSION));
        assert!(!offer.contains(0x0302));
        assert_eq!(offer.encoded_len(), 7);

        let mut output = [0xa5; 9];
        assert_eq!(offer.encode(&mut output), Ok(7));
        assert_eq!(&output[..7], &[6, 0x7a, 0x7a, 0x03, 0x04, 0x03, 0x03]);
        assert_eq!(&output[7..], &[0xa5, 0xa5]);
    }

    #[test]
    fn offer_validation_is_bounded_and_duplicate_free() {
        assert_eq!(ClientVersionOffer::new(&[]), Err(VersionError::EmptyOffer));
        assert_eq!(
            ClientVersionOffer::new(&[0x0303]),
            Err(VersionError::MissingTls13)
        );
        assert_eq!(
            ClientVersionOffer::new(&[TLS13_VERSION, TLS13_VERSION]),
            Err(VersionError::DuplicateVersion)
        );

        let maximum = [TLS13_VERSION; MAX_CLIENT_VERSION_COUNT];
        assert_eq!(
            ClientVersionOffer::new(&maximum),
            Err(VersionError::DuplicateVersion)
        );

        let too_many = [TLS13_VERSION; MAX_CLIENT_VERSION_COUNT + 1];
        assert_eq!(
            ClientVersionOffer::new(&too_many),
            Err(VersionError::TooManyVersions)
        );
    }

    #[test]
    fn maximum_distinct_offer_fits_the_rfc_vector() {
        let mut versions = [0_u16; MAX_CLIENT_VERSION_COUNT];
        for (index, version) in versions.iter_mut().enumerate() {
            *version = index as u16;
        }
        versions[MAX_CLIENT_VERSION_COUNT - 1] = TLS13_VERSION;

        let offer = ClientVersionOffer::new(&versions).expect("maximum offer is valid");
        let mut output = [0_u8; MAX_CLIENT_EXTENSION_DATA_LENGTH];
        assert_eq!(
            offer.encode(&mut output),
            Ok(MAX_CLIENT_EXTENSION_DATA_LENGTH)
        );
        assert_eq!(output[0], 254);
        assert_eq!(&output[253..], &[0x03, 0x04]);
    }

    #[test]
    fn short_output_is_rejected_without_mutation() {
        let versions = [TLS13_VERSION, 0x0303];
        let offer = ClientVersionOffer::new(&versions).expect("offer is valid");
        let mut output = [0xa5; 4];

        assert_eq!(offer.encode(&mut output), Err(VersionError::OutputTooSmall));
        assert_eq!(output, [0xa5; 4]);
    }

    #[test]
    fn server_selection_is_exact_offered_and_tls13_only() {
        let versions = [0x0305, TLS13_VERSION, 0x0303];
        let offer = ClientVersionOffer::new(&versions).expect("offer is valid");

        let selected = offer
            .validate_server_selection(&[0x03, 0x04])
            .expect("TLS 1.3 was offered");
        assert_eq!(selected.wire_id(), TLS13_VERSION);
        assert_eq!(
            selected,
            SelectedTls13Version {
                wire_id: TLS13_VERSION
            }
        );

        assert_eq!(
            offer.validate_server_selection(&[]),
            Err(VersionError::MalformedServerSelection)
        );
        assert_eq!(
            offer.validate_server_selection(&[0x03]),
            Err(VersionError::MalformedServerSelection)
        );
        assert_eq!(
            offer.validate_server_selection(&[0x03, 0x04, 0x00]),
            Err(VersionError::MalformedServerSelection)
        );
        assert_eq!(
            offer.validate_server_selection(&[0x03, 0x02]),
            Err(VersionError::UnofferedServerSelection)
        );
        assert_eq!(
            offer.validate_server_selection(&[0x03, 0x03]),
            Err(VersionError::ServerSelectedPriorVersion)
        );
        assert_eq!(
            offer.validate_server_selection(&[0x03, 0x05]),
            Err(VersionError::UnsupportedServerSelection)
        );
    }

    #[test]
    fn diagnostics_do_not_expose_configured_versions() {
        let versions = [0xfefe, TLS13_VERSION];
        let offer = ClientVersionOffer::new(&versions).expect("offer is valid");
        let debug = format!("{offer:?}");
        assert!(debug.contains("version_count: 2"));
        assert!(debug.contains("contains_tls13: true"));
        assert!(!debug.contains("65278"));
        assert!(!debug.contains("fefe"));

        let cases = [
            (VersionError::EmptyOffer, "empty TLS version offer"),
            (
                VersionError::TooManyVersions,
                "TLS version offer exceeds the vector limit",
            ),
            (
                VersionError::DuplicateVersion,
                "duplicate TLS version in offer",
            ),
            (
                VersionError::MissingTls13,
                "TLS 1.3 is absent from the version offer",
            ),
            (
                VersionError::OutputTooSmall,
                "output is too small for the TLS version offer",
            ),
            (
                VersionError::MalformedServerSelection,
                "malformed TLS server version selection",
            ),
            (
                VersionError::UnofferedServerSelection,
                "server selected an unoffered TLS version",
            ),
            (
                VersionError::ServerSelectedPriorVersion,
                "server selected a version before TLS 1.3",
            ),
            (
                VersionError::UnsupportedServerSelection,
                "server selected an unsupported TLS version",
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }
}
