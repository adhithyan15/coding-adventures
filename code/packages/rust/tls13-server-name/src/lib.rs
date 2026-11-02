//! Bounded TLS 1.3 `server_name` extension framing.
//!
//! RFC 6066 gives the ClientHello extension data a nested two-byte-length-
//! prefixed list containing typed server names. The only defined name type is
//! `host_name`, and a list cannot contain two values of the same type. This
//! crate deliberately admits exactly one DNS hostname, which is the only wire
//! shape a current TLS client can use without inventing future name semantics.
//!
//! The validated name is borrowed and encoding is allocation-free. The crate
//! receives already-delimited extension data and does not decide whether an
//! extension is present in the right handshake message.

#![no_std]
#![forbid(unsafe_code)]

use core::fmt;

/// The IANA extension type for `server_name`.
pub const SERVER_NAME_EXTENSION_TYPE: u16 = 0;

/// The only `NameType` currently defined by RFC 6066.
pub const HOST_NAME_TYPE: u8 = 0;

/// The maximum textual DNS name length without a trailing root dot.
pub const MAX_DNS_NAME_LENGTH: usize = 253;

/// The maximum DNS label length.
pub const MAX_DNS_LABEL_LENGTH: usize = 63;

/// The maximum encoded extension-data length admitted by this package.
///
/// This includes the two-byte `ServerNameList` length, one-byte name type,
/// two-byte hostname length, and a maximum-size DNS hostname.
pub const MAX_CLIENT_EXTENSION_DATA_LENGTH: usize = 2 + 1 + 2 + MAX_DNS_NAME_LENGTH;

/// Stable, payload-free DNS hostname failures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InvalidDnsName {
    Empty,
    TooLong,
    NonAscii,
    TrailingDot,
    EmptyLabel,
    LabelTooLong,
    InvalidCharacter,
    LeadingHyphen,
    TrailingHyphen,
    IpLiteral,
}

/// Stable, payload-free server-name framing and policy failures.
///
/// No variant retains a hostname, extension bytes, caller capacity, or
/// observed wire length.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ServerNameError {
    InvalidDnsName(InvalidDnsName),
    MalformedClientExtension,
    UnsupportedNameType,
    OutputTooSmall,
    MalformedServerAcknowledgement,
}

impl fmt::Display for ServerNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidDnsName(_) => "invalid TLS server DNS name",
            Self::MalformedClientExtension => "malformed TLS client server-name extension",
            Self::UnsupportedNameType => "unsupported TLS server-name type",
            Self::OutputTooSmall => "output is too small for the TLS server-name extension",
            Self::MalformedServerAcknowledgement => "malformed TLS server-name acknowledgement",
        };
        formatter.write_str(message)
    }
}

impl core::error::Error for ServerNameError {}

/// One validated, borrowed DNS hostname for a ClientHello SNI extension.
///
/// The name is ASCII, contains no trailing dot or wildcard, is not an IP
/// literal, and satisfies the package's bounded DNS-label profile. Case is
/// preserved because DNS hostname comparison is case-insensitive but the wire
/// format does not require normalization.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct TlsServerName<'a> {
    dns_name: &'a str,
}

impl fmt::Debug for TlsServerName<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TlsServerName")
            .field("length", &self.dns_name.len())
            .field("label_count", &self.dns_name.split('.').count())
            .finish_non_exhaustive()
    }
}

impl<'a> TlsServerName<'a> {
    /// Validate one application-selected DNS hostname.
    pub fn new(dns_name: &'a str) -> Result<Self, ServerNameError> {
        validate_dns_name(dns_name).map_err(ServerNameError::InvalidDnsName)?;
        Ok(Self { dns_name })
    }

    /// Decode exact ClientHello `server_name` extension data.
    ///
    /// The admitted profile contains exactly one `host_name` entry. Future or
    /// unknown name types fail closed rather than becoming supported implicitly.
    pub fn decode_client_hello(extension_data: &'a [u8]) -> Result<Self, ServerNameError> {
        if extension_data.len() < 5 {
            return Err(ServerNameError::MalformedClientExtension);
        }

        let list_length = usize::from(u16::from_be_bytes([extension_data[0], extension_data[1]]));
        let name_type = extension_data[2];
        let name_length = usize::from(u16::from_be_bytes([extension_data[3], extension_data[4]]));
        let rest = &extension_data[5..];

        if list_length == 0 || list_length != extension_data.len() - 2 {
            return Err(ServerNameError::MalformedClientExtension);
        }
        if name_type != HOST_NAME_TYPE {
            return Err(ServerNameError::UnsupportedNameType);
        }

        if name_length == 0 || name_length != rest.len() {
            return Err(ServerNameError::MalformedClientExtension);
        }

        let dns_name = core::str::from_utf8(rest)
            .map_err(|_| ServerNameError::InvalidDnsName(InvalidDnsName::NonAscii))?;
        Self::new(dns_name)
    }

    /// Return the validated hostname exactly as supplied by the caller.
    #[must_use]
    pub const fn as_str(self) -> &'a str {
        self.dns_name
    }

    /// Return the complete encoded ClientHello extension-data length.
    #[must_use]
    pub const fn encoded_len(self) -> usize {
        2 + 1 + 2 + self.dns_name.len()
    }

    /// Encode exact ClientHello extension data into caller-owned output.
    ///
    /// Complete capacity is checked before output is modified.
    pub fn encode_client_hello(self, output: &mut [u8]) -> Result<usize, ServerNameError> {
        let encoded_len = self.encoded_len();
        if output.len() < encoded_len {
            return Err(ServerNameError::OutputTooSmall);
        }

        let list_length = 1 + 2 + self.dns_name.len();
        let list_length = (list_length as u16).to_be_bytes();
        let name_length = (self.dns_name.len() as u16).to_be_bytes();

        output[0..2].copy_from_slice(&list_length);
        output[2] = HOST_NAME_TYPE;
        output[3..5].copy_from_slice(&name_length);
        output[5..encoded_len].copy_from_slice(self.dns_name.as_bytes());
        Ok(encoded_len)
    }

    /// Validate an EncryptedExtensions `server_name` acknowledgement.
    ///
    /// RFC 6066 requires the server's extension data to be empty. Presence and
    /// message-context authorization remain the caller's responsibility.
    pub fn validate_server_acknowledgement(
        self,
        extension_data: &[u8],
    ) -> Result<(), ServerNameError> {
        if extension_data.is_empty() {
            Ok(())
        } else {
            Err(ServerNameError::MalformedServerAcknowledgement)
        }
    }
}

fn validate_dns_name(name: &str) -> Result<(), InvalidDnsName> {
    if name.is_empty() {
        return Err(InvalidDnsName::Empty);
    }
    if name.len() > MAX_DNS_NAME_LENGTH {
        return Err(InvalidDnsName::TooLong);
    }
    if !name.is_ascii() {
        return Err(InvalidDnsName::NonAscii);
    }
    if name.ends_with('.') {
        return Err(InvalidDnsName::TrailingDot);
    }
    if name.contains(':') || is_ipv4_literal(name) {
        return Err(InvalidDnsName::IpLiteral);
    }

    for label in name.split('.') {
        if label.is_empty() {
            return Err(InvalidDnsName::EmptyLabel);
        }
        if label.len() > MAX_DNS_LABEL_LENGTH {
            return Err(InvalidDnsName::LabelTooLong);
        }
        if label.starts_with('-') {
            return Err(InvalidDnsName::LeadingHyphen);
        }
        if label.ends_with('-') {
            return Err(InvalidDnsName::TrailingHyphen);
        }
        if !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(InvalidDnsName::InvalidCharacter);
        }
    }

    Ok(())
}

fn is_ipv4_literal(name: &str) -> bool {
    let mut count = 0_usize;
    for component in name.split('.') {
        count += 1;
        if count > 4 || component.is_empty() || !is_ipv4_number_component(component) {
            return false;
        }
    }
    count != 0
}

fn is_ipv4_number_component(component: &str) -> bool {
    if let Some(hexadecimal) = component
        .strip_prefix("0x")
        .or_else(|| component.strip_prefix("0X"))
    {
        !hexadecimal.is_empty() && hexadecimal.bytes().all(|byte| byte.is_ascii_hexdigit())
    } else {
        component.bytes().all(|byte| byte.is_ascii_digit())
    }
}
