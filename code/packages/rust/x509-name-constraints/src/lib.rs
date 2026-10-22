//! # Reviewed-anchor DNS NameConstraints structure
//!
//! This crate decodes a narrow, bounded RFC 5280 structure. It deliberately
//! grants no constraint-evaluation, certificate-path, trust-root, or TLS
//! authority.

#![deny(unsafe_code)]

use der_asn1::{Asn1Decoder, Asn1ErrorKind};
use std::error::Error;
use std::fmt;

pub const MAX_DNS_SUBTREES: usize = 256;

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
    pub fn permitted_dns_subtrees(&self) -> &[&'a str] {
        &self.permitted_dns_subtrees[..self.len]
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

pub fn decode_name_constraints_contents<'a>(
    _decoder: &mut Asn1Decoder,
    _encoded_contents: &'a [u8],
) -> Result<NameConstraints<'a>, NameConstraintsError> {
    todo!("implemented after the test-first scaffold")
}
