//! # Reviewed-anchor DNS NameConstraints structure
//!
//! This crate decodes a narrow, bounded RFC 5280 structure. It deliberately
//! grants no constraint-evaluation, certificate-path, trust-root, or TLS
//! authority.

#![deny(unsafe_code)]

use der_asn1::{decode_implicit_ia5_string, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind};
use der_tlv::{DerTag, TagClass};
use std::error::Error;
use std::fmt;

const SEQUENCE_TAG: u32 = 16;
const PERMITTED_SUBTREES_TAG: u32 = 0;
const DNS_NAME_TAG: u32 = 2;

/// Maximum DNS GeneralSubtree values retained from one constraints value.
pub const MAX_DNS_SUBTREES: usize = 256;

/// Stable payload-free failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameConstraintsErrorKind {
    Structure(Asn1ErrorKind),
    UnsupportedTopLevelField,
    EmptyPermittedSubtrees,
    TooManySubtrees,
    InvalidGeneralSubtree,
    UnsupportedGeneralName,
    EmptyDnsSubtree,
    InvalidDnsSubtree(Asn1ErrorKind),
    UnsupportedGeneralSubtreeField,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameConstraintsError {
    kind: NameConstraintsErrorKind,
    offset: usize,
}

impl NameConstraintsError {
    const fn new(kind: NameConstraintsErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    fn structure(error: Asn1Error) -> Self {
        Self::new(
            NameConstraintsErrorKind::Structure(error.kind()),
            error.offset(),
        )
    }

    fn child_structure(error: Asn1Error, value_offset: usize, child_offset: usize) -> Self {
        let offset = match error.kind() {
            Asn1ErrorKind::Framing(_) => value_offset + error.offset(),
            _ => child_offset + error.offset(),
        };
        Self::new(NameConstraintsErrorKind::Structure(error.kind()), offset)
    }

    fn element_structure(error: Asn1Error, element_offset: usize) -> Self {
        Self::new(
            NameConstraintsErrorKind::Structure(error.kind()),
            element_offset + error.offset(),
        )
    }

    fn dns(error: Asn1Error, element_offset: usize) -> Self {
        Self::new(
            NameConstraintsErrorKind::InvalidDnsSubtree(error.kind()),
            element_offset + error.offset(),
        )
    }

    pub const fn kind(self) -> NameConstraintsErrorKind {
        self.kind
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl fmt::Display for NameConstraintsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 name constraints error {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl Error for NameConstraintsError {}

/// Borrowed DNS subtrees from one supported reviewed-anchor constraints value.
#[derive(Clone)]
pub struct NameConstraints<'a> {
    permitted_dns_subtrees: [&'a str; MAX_DNS_SUBTREES],
    len: usize,
}

impl fmt::Debug for NameConstraints<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NameConstraints")
            .field("permitted_dns_subtrees", &self.len)
            .finish()
    }
}

impl<'a> NameConstraints<'a> {
    /// Return permitted DNS subtree strings in wire order.
    pub fn permitted_dns_subtrees(&self) -> &[&'a str] {
        &self.permitted_dns_subtrees[..self.len]
    }

    /// Return the number of retained DNS subtrees.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Return whether no DNS subtree was retained.
    ///
    /// Successful decoding is never empty; this method keeps collection use
    /// explicit for downstream policy code.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Decode the supported DER contents of an RFC 5280 NameConstraints SEQUENCE.
///
/// The accepted subset is one `[0] permittedSubtrees` field containing
/// non-empty dNSName GeneralSubtree values with omitted distance fields.
pub fn decode_name_constraints_contents<'a>(
    decoder: &mut Asn1Decoder,
    encoded_contents: &'a [u8],
) -> Result<NameConstraints<'a>, NameConstraintsError> {
    let root = decoder
        .decode_exact(encoded_contents)
        .map_err(NameConstraintsError::structure)?;
    if !is_exact_tag(
        root.tag(),
        TagClass::ContextSpecific,
        true,
        PERMITTED_SUBTREES_TAG,
    ) {
        return Err(NameConstraintsError::new(
            NameConstraintsErrorKind::UnsupportedTopLevelField,
            0,
        ));
    }

    let value_offset = root.header().len();
    let value_length = root.value().len();
    let mut subtrees = decoder
        .implicit_constructed(root, PERMITTED_SUBTREES_TAG)
        .map_err(NameConstraintsError::structure)?;
    if subtrees.remaining().is_empty() {
        return Err(NameConstraintsError::new(
            NameConstraintsErrorKind::EmptyPermittedSubtrees,
            value_offset,
        ));
    }

    let mut decoded = NameConstraints {
        permitted_dns_subtrees: [""; MAX_DNS_SUBTREES],
        len: 0,
    };

    while !subtrees.remaining().is_empty() {
        let subtree_offset = child_offset(value_offset, value_length, subtrees.remaining().len());
        if decoded.len == MAX_DNS_SUBTREES {
            return Err(NameConstraintsError::new(
                NameConstraintsErrorKind::TooManySubtrees,
                subtree_offset,
            ));
        }

        let subtree = subtrees
            .read(decoder)
            .map_err(|error| {
                NameConstraintsError::child_structure(error, value_offset, subtree_offset)
            })?
            .expect("a non-empty DER cursor must yield one element");
        if !is_exact_tag(subtree.tag(), TagClass::Universal, true, SEQUENCE_TAG) {
            return Err(NameConstraintsError::new(
                NameConstraintsErrorKind::InvalidGeneralSubtree,
                subtree_offset,
            ));
        }

        let dns_name = decode_general_subtree(decoder, subtree, subtree_offset)?;
        decoded.permitted_dns_subtrees[decoded.len] = dns_name;
        decoded.len += 1;
    }

    subtrees.finish().map_err(NameConstraintsError::structure)?;
    Ok(decoded)
}

fn decode_general_subtree<'a>(
    decoder: &mut Asn1Decoder,
    subtree: Asn1Element<'a>,
    subtree_offset: usize,
) -> Result<&'a str, NameConstraintsError> {
    let value_offset = subtree_offset + subtree.header().len();
    let value_length = subtree.value().len();
    let mut fields = decoder
        .sequence(subtree)
        .map_err(|error| NameConstraintsError::element_structure(error, subtree_offset))?;

    let base_offset = child_offset(value_offset, value_length, fields.remaining().len());
    let Some(base) = fields
        .read(decoder)
        .map_err(|error| NameConstraintsError::child_structure(error, value_offset, base_offset))?
    else {
        return Err(NameConstraintsError::new(
            NameConstraintsErrorKind::UnsupportedGeneralName,
            base_offset,
        ));
    };

    if !is_exact_tag(base.tag(), TagClass::ContextSpecific, false, DNS_NAME_TAG) {
        return Err(NameConstraintsError::new(
            NameConstraintsErrorKind::UnsupportedGeneralName,
            base_offset,
        ));
    }
    if base.value().is_empty() {
        return Err(NameConstraintsError::new(
            NameConstraintsErrorKind::EmptyDnsSubtree,
            base_offset + base.header().len(),
        ));
    }

    let dns_name = decode_implicit_ia5_string(base, DNS_NAME_TAG)
        .map_err(|error| NameConstraintsError::dns(error, base_offset))?;

    let trailing_offset = child_offset(value_offset, value_length, fields.remaining().len());
    if fields
        .read(decoder)
        .map_err(|error| {
            NameConstraintsError::child_structure(error, value_offset, trailing_offset)
        })?
        .is_some()
    {
        return Err(NameConstraintsError::new(
            NameConstraintsErrorKind::UnsupportedGeneralSubtreeField,
            trailing_offset,
        ));
    }
    fields
        .finish()
        .map_err(|error| NameConstraintsError::element_structure(error, subtree_offset))?;
    Ok(dns_name)
}

fn is_exact_tag(tag: DerTag, class: TagClass, constructed: bool, number: u32) -> bool {
    tag.class == class && tag.constructed == constructed && tag.number == number
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
