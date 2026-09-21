//! # Bounded RFC 5280 Subject Alternative Name values
//!
//! This crate recognizes one Subject Alternative Name extension and retains
//! only the DNS and IP identities needed by a separate identity matcher. It
//! deliberately grants no certificate-path, trust-root, or TLS authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_implicit_ia5_string, decode_implicit_object_identifier, decode_implicit_octet_string,
    Asn1Decoder, Asn1Error, Asn1ErrorKind,
};
use der_tlv::TagClass;
use std::error::Error;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use x509_extension::X509Extension;

/// RFC 5280 `id-ce-subjectAltName` (`2.5.29.17`).
pub const SUBJECT_ALT_NAME_OID: &[u64] = &[2, 5, 29, 17];

/// Maximum number of GeneralName values accepted from one extension.
pub const MAX_GENERAL_NAMES: usize = 256;

/// Stable failure categories. No category retains source bytes or names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubjectAltNameErrorKind {
    WrongExtensionId,
    Structure(Asn1ErrorKind),
    Empty,
    TooManyNames,
    InvalidGeneralNameTag,
    EmptyGeneralName,
    InvalidIa5Name(Asn1ErrorKind),
    InvalidIpAddressLength,
    InvalidRegisteredId(Asn1ErrorKind),
}

/// A redacted failure at an offset local to the encapsulated value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubjectAltNameError {
    kind: SubjectAltNameErrorKind,
    offset: usize,
}

impl SubjectAltNameError {
    const fn new(kind: SubjectAltNameErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            SubjectAltNameErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(SubjectAltNameErrorKind::Structure(error.kind()), offset)
    }

    fn ia5(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            SubjectAltNameErrorKind::InvalidIa5Name(error.kind()),
            child_offset + error.offset(),
        )
    }

    fn registered_id(error: Asn1Error, child_offset: usize) -> Self {
        Self::new(
            SubjectAltNameErrorKind::InvalidRegisteredId(error.kind()),
            child_offset + error.offset(),
        )
    }

    pub const fn kind(self) -> SubjectAltNameErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for SubjectAltNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 subject alternative name error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for SubjectAltNameError {}

/// One bounded collection of identities decoded from Subject Alternative Name.
#[derive(Clone)]
pub struct SubjectAlternativeName<'a> {
    dns_names: [&'a str; MAX_GENERAL_NAMES],
    ip_addresses: [IpAddr; MAX_GENERAL_NAMES],
    dns_len: usize,
    ip_len: usize,
    total_names: usize,
}

impl fmt::Debug for SubjectAlternativeName<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubjectAlternativeName")
            .field("dns_names", &self.dns_len)
            .field("ip_addresses", &self.ip_len)
            .field("opaque_names", &self.opaque_names())
            .finish()
    }
}

impl<'a> SubjectAlternativeName<'a> {
    /// Borrow decoded DNS-ID values in wire order relative to other DNS-IDs.
    pub fn dns_names(&self) -> &[&'a str] {
        &self.dns_names[..self.dns_len]
    }

    /// Borrow decoded IP-ID values in wire order relative to other IP-IDs.
    pub fn ip_addresses(&self) -> &[IpAddr] {
        &self.ip_addresses[..self.ip_len]
    }

    pub const fn len(&self) -> usize {
        self.total_names
    }

    pub const fn is_empty(&self) -> bool {
        self.total_names == 0
    }

    /// Number of validated primitive or bounded constructed choices not
    /// retained as DNS or IP identities.
    pub const fn opaque_names(&self) -> usize {
        self.total_names - self.dns_len - self.ip_len
    }
}

/// Decode one generic extension as RFC 5280 Subject Alternative Name.
pub fn decode_subject_alt_name<'a>(
    decoder: &mut Asn1Decoder,
    extension: X509Extension<'a>,
) -> Result<SubjectAlternativeName<'a>, SubjectAltNameError> {
    if !extension.extension_id().equals(SUBJECT_ALT_NAME_OID) {
        return Err(SubjectAltNameError::new(
            SubjectAltNameErrorKind::WrongExtensionId,
            0,
        ));
    }

    let root = decoder
        .decode_exact(extension.extension_value())
        .map_err(SubjectAltNameError::structure)?;
    let value_offset = root.header().len();
    let value_length = root.value().len();
    let mut names = decoder
        .sequence(root)
        .map_err(SubjectAltNameError::structure)?;
    let mut decoded = SubjectAlternativeName {
        dns_names: [""; MAX_GENERAL_NAMES],
        ip_addresses: [IpAddr::V4(Ipv4Addr::UNSPECIFIED); MAX_GENERAL_NAMES],
        dns_len: 0,
        ip_len: 0,
        total_names: 0,
    };

    while !names.remaining().is_empty() {
        let child_offset = child_offset(value_offset, value_length, names.remaining().len());
        let element = names
            .read(decoder)
            .map_err(|error| {
                SubjectAltNameError::child_structure(error, value_offset, child_offset)
            })?
            .expect("a non-empty DER cursor must yield one element");

        if decoded.total_names == MAX_GENERAL_NAMES {
            return Err(SubjectAltNameError::new(
                SubjectAltNameErrorKind::TooManyNames,
                child_offset,
            ));
        }

        validate_general_name_tag(element.tag(), child_offset)?;
        if element.value().is_empty() {
            return Err(SubjectAltNameError::new(
                SubjectAltNameErrorKind::EmptyGeneralName,
                child_offset + element.header().len(),
            ));
        }

        match element.tag().number {
            1 | 6 => {
                decode_implicit_ia5_string(element, element.tag().number)
                    .map_err(|error| SubjectAltNameError::ia5(error, child_offset))?;
            }
            2 => {
                let dns_name = decode_implicit_ia5_string(element, 2)
                    .map_err(|error| SubjectAltNameError::ia5(error, child_offset))?;
                decoded.dns_names[decoded.dns_len] = dns_name;
                decoded.dns_len += 1;
            }
            7 => {
                let octets = decode_implicit_octet_string(element, 7)
                    .map_err(SubjectAltNameError::structure)?;
                let address = match octets {
                    [a, b, c, d] => IpAddr::V4(Ipv4Addr::new(*a, *b, *c, *d)),
                    bytes if bytes.len() == 16 => {
                        let mut address = [0u8; 16];
                        address.copy_from_slice(bytes);
                        IpAddr::V6(Ipv6Addr::from(address))
                    }
                    _ => {
                        return Err(SubjectAltNameError::new(
                            SubjectAltNameErrorKind::InvalidIpAddressLength,
                            child_offset + element.header().len(),
                        ));
                    }
                };
                decoded.ip_addresses[decoded.ip_len] = address;
                decoded.ip_len += 1;
            }
            8 => {
                decode_implicit_object_identifier(element, 8, decoder.limits())
                    .map_err(|error| SubjectAltNameError::registered_id(error, child_offset))?;
            }
            0 | 3 | 4 | 5 => {}
            _ => unreachable!("validated GeneralName tag range"),
        }
        decoded.total_names += 1;
    }

    if decoded.is_empty() {
        return Err(SubjectAltNameError::new(
            SubjectAltNameErrorKind::Empty,
            value_offset,
        ));
    }
    names.finish().map_err(SubjectAltNameError::structure)?;
    Ok(decoded)
}

fn validate_general_name_tag(
    tag: der_tlv::DerTag,
    offset: usize,
) -> Result<(), SubjectAltNameError> {
    let expected_constructed = match tag.number {
        0 | 3 | 4 | 5 => true,
        1 | 2 | 6 | 7 | 8 => false,
        _ => {
            return Err(SubjectAltNameError::new(
                SubjectAltNameErrorKind::InvalidGeneralNameTag,
                offset,
            ));
        }
    };
    if tag.class != TagClass::ContextSpecific || tag.constructed != expected_constructed {
        return Err(SubjectAltNameError::new(
            SubjectAltNameErrorKind::InvalidGeneralNameTag,
            offset,
        ));
    }
    Ok(())
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
