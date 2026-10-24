//! # DNS NameConstraints structure
//!
//! This crate decodes a narrow, bounded RFC 5280 structure and evaluates its
//! permitted and excluded DNS subtrees against caller-supplied DNS names. It
//! deliberately grants no certificate-evidence, path, trust-root, or TLS
//! authority.

#![deny(unsafe_code)]

use der_asn1::{decode_implicit_ia5_string, Asn1Decoder, Asn1Element, Asn1Error, Asn1ErrorKind};
use der_tlv::{DerTag, TagClass};
use std::error::Error;
use std::fmt;

const SEQUENCE_TAG: u32 = 16;
const PERMITTED_SUBTREES_TAG: u32 = 0;
const EXCLUDED_SUBTREES_TAG: u32 = 1;
const DNS_NAME_TAG: u32 = 2;

/// RFC 5280 id-ce-nameConstraints (2.5.29.30).
pub const NAME_CONSTRAINTS_OID: &[u64] = &[2, 5, 29, 30];

/// Maximum DNS GeneralSubtree values retained from one constraints value.
pub const MAX_DNS_SUBTREES: usize = 256;
/// Maximum presented DNS names evaluated against one constraints value.
pub const MAX_PRESENTED_DNS_NAMES: usize = 256;

/// Stable syntax failures for DNS constraints and presented DNS names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DnsNameSyntaxError {
    Empty,
    TooLong,
    NonAscii,
    EmptyLabel,
    LabelTooLong,
    InvalidCharacter,
    LeadingHyphen,
    TrailingHyphen,
    WildcardNotAllowed,
    InvalidWildcard,
}

/// Stable payload-free DNS constraint evaluation failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DnsConstraintErrorKind {
    TooManyPresentedDnsNames,
    InvalidPermittedSubtree(DnsNameSyntaxError),
    InvalidExcludedSubtree(DnsNameSyntaxError),
    InvalidPresentedDnsName(DnsNameSyntaxError),
    PresentedDnsNameExcluded,
    PresentedDnsNameNotPermitted,
}

/// One redacted evaluation failure and its relevant collection index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DnsConstraintError {
    kind: DnsConstraintErrorKind,
    item_index: usize,
}

impl DnsConstraintError {
    const fn new(kind: DnsConstraintErrorKind, item_index: usize) -> Self {
        Self { kind, item_index }
    }

    pub const fn kind(self) -> DnsConstraintErrorKind {
        self.kind
    }

    pub const fn item_index(self) -> usize {
        self.item_index
    }
}

impl fmt::Display for DnsConstraintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 DNS constraint error {:?} at item {}",
            self.kind, self.item_index
        )
    }
}

impl Error for DnsConstraintError {}

/// Payload-free evidence that all supplied DNS names satisfied the constraints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstrainedDnsNames {
    count: usize,
}

impl ConstrainedDnsNames {
    pub const fn count(self) -> usize {
        self.count
    }

    pub const fn is_empty(self) -> bool {
        self.count == 0
    }
}

/// Stable payload-free failure categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameConstraintsErrorKind {
    Structure(Asn1ErrorKind),
    UnsupportedTopLevelField,
    DuplicateTopLevelField,
    UnsupportedTopLevelFieldOrder,
    EmptyPermittedSubtrees,
    EmptyExcludedSubtrees,
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
    permitted_len: usize,
    excluded_dns_subtrees: [&'a str; MAX_DNS_SUBTREES],
    excluded_len: usize,
}

impl fmt::Debug for NameConstraints<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NameConstraints")
            .field("permitted_dns_subtrees", &self.permitted_len)
            .field("excluded_dns_subtrees", &self.excluded_len)
            .finish()
    }
}

impl<'a> NameConstraints<'a> {
    /// Return permitted DNS subtree strings in wire order.
    pub fn permitted_dns_subtrees(&self) -> &[&'a str] {
        &self.permitted_dns_subtrees[..self.permitted_len]
    }

    /// Return excluded DNS subtree strings in wire order.
    pub fn excluded_dns_subtrees(&self) -> &[&'a str] {
        &self.excluded_dns_subtrees[..self.excluded_len]
    }

    /// Return the number of retained permitted DNS subtrees.
    pub const fn permitted_len(&self) -> usize {
        self.permitted_len
    }

    /// Return the number of retained excluded DNS subtrees.
    pub const fn excluded_len(&self) -> usize {
        self.excluded_len
    }

    /// Return the total number of retained DNS subtrees.
    pub const fn len(&self) -> usize {
        self.permitted_len + self.excluded_len
    }

    /// Return whether no DNS subtree was retained.
    ///
    /// Successful decoding is never empty; this method keeps collection use
    /// explicit for downstream policy code.
    pub const fn is_empty(&self) -> bool {
        self.permitted_len == 0 && self.excluded_len == 0
    }
}

/// Evaluate presented DNS subjectAltName values against both constraint sets.
pub fn evaluate_dns_names(
    constraints: &NameConstraints<'_>,
    presented_dns_names: &[&str],
) -> Result<ConstrainedDnsNames, DnsConstraintError> {
    if presented_dns_names.len() > MAX_PRESENTED_DNS_NAMES {
        return Err(DnsConstraintError::new(
            DnsConstraintErrorKind::TooManyPresentedDnsNames,
            MAX_PRESENTED_DNS_NAMES,
        ));
    }

    let mut validated_permitted = [ValidConstraint::EMPTY; MAX_DNS_SUBTREES];
    for (index, subtree) in constraints.permitted_dns_subtrees().iter().enumerate() {
        validated_permitted[index] = validate_constraint(subtree).map_err(|reason| {
            DnsConstraintError::new(
                DnsConstraintErrorKind::InvalidPermittedSubtree(reason),
                index,
            )
        })?;
    }
    let mut validated_excluded = [ValidConstraint::EMPTY; MAX_DNS_SUBTREES];
    for (index, subtree) in constraints.excluded_dns_subtrees().iter().enumerate() {
        validated_excluded[index] = validate_constraint(subtree).map_err(|reason| {
            DnsConstraintError::new(
                DnsConstraintErrorKind::InvalidExcludedSubtree(reason),
                index,
            )
        })?;
    }

    for (index, name) in presented_dns_names.iter().enumerate() {
        validate_presented_name(name).map_err(|reason| {
            DnsConstraintError::new(
                DnsConstraintErrorKind::InvalidPresentedDnsName(reason),
                index,
            )
        })?;

        let excluded = validated_excluded[..constraints.excluded_len()]
            .iter()
            .any(|constraint| overlaps_excluded_subtree(name, *constraint));
        if excluded {
            return Err(DnsConstraintError::new(
                DnsConstraintErrorKind::PresentedDnsNameExcluded,
                index,
            ));
        }

        let permitted = constraints.permitted_len() == 0
            || validated_permitted[..constraints.permitted_len()]
                .iter()
                .any(|constraint| within_subtree(name, *constraint));
        if !permitted {
            return Err(DnsConstraintError::new(
                DnsConstraintErrorKind::PresentedDnsNameNotPermitted,
                index,
            ));
        }
    }

    Ok(ConstrainedDnsNames {
        count: presented_dns_names.len(),
    })
}

#[derive(Clone, Copy)]
struct ValidConstraint<'a> {
    base: &'a str,
    subdomains_only: bool,
}

impl ValidConstraint<'_> {
    const EMPTY: Self = Self {
        base: "",
        subdomains_only: false,
    };
}

fn validate_constraint(subtree: &str) -> Result<ValidConstraint<'_>, DnsNameSyntaxError> {
    let (base, subdomains_only) = match subtree.strip_prefix('.') {
        Some(base) => (base, true),
        None => (subtree, false),
    };
    validate_domain(base, WildcardPolicy::Reject)?;
    Ok(ValidConstraint {
        base,
        subdomains_only,
    })
}

fn validate_presented_name(name: &str) -> Result<(), DnsNameSyntaxError> {
    validate_domain(name, WildcardPolicy::AllowPresented)
}

#[derive(Clone, Copy)]
enum WildcardPolicy {
    Reject,
    AllowPresented,
}

fn validate_domain(name: &str, wildcard_policy: WildcardPolicy) -> Result<(), DnsNameSyntaxError> {
    if name.is_empty() {
        return Err(DnsNameSyntaxError::Empty);
    }
    if name.len() > 253 {
        return Err(DnsNameSyntaxError::TooLong);
    }
    if !name.is_ascii() {
        return Err(DnsNameSyntaxError::NonAscii);
    }
    if name.contains('*') {
        match wildcard_policy {
            WildcardPolicy::Reject => return Err(DnsNameSyntaxError::WildcardNotAllowed),
            WildcardPolicy::AllowPresented => {
                if !name.starts_with("*.") || name[2..].is_empty() || name[2..].contains('*') {
                    return Err(DnsNameSyntaxError::InvalidWildcard);
                }
            }
        }
    }

    for (index, label) in name.split('.').enumerate() {
        if label.is_empty() {
            return Err(DnsNameSyntaxError::EmptyLabel);
        }
        if label.len() > 63 {
            return Err(DnsNameSyntaxError::LabelTooLong);
        }
        if label == "*" && index == 0 && matches!(wildcard_policy, WildcardPolicy::AllowPresented) {
            continue;
        }
        if label.as_bytes()[0] == b'-' {
            return Err(DnsNameSyntaxError::LeadingHyphen);
        }
        if label.as_bytes()[label.len() - 1] == b'-' {
            return Err(DnsNameSyntaxError::TrailingHyphen);
        }
        if !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(if label.contains('*') {
                DnsNameSyntaxError::InvalidWildcard
            } else {
                DnsNameSyntaxError::InvalidCharacter
            });
        }
    }

    Ok(())
}

fn within_subtree(name: &str, constraint: ValidConstraint<'_>) -> bool {
    if name.eq_ignore_ascii_case(constraint.base) {
        return !constraint.subdomains_only;
    }
    if name.len() <= constraint.base.len() {
        return false;
    }

    let base_offset = name.len() - constraint.base.len();
    name.as_bytes()[base_offset - 1] == b'.'
        && name[base_offset..].eq_ignore_ascii_case(constraint.base)
}

fn overlaps_excluded_subtree(name: &str, constraint: ValidConstraint<'_>) -> bool {
    if within_subtree(name, constraint) {
        return true;
    }

    // A presented wildcard stands for exactly one concrete left-most label.
    // Ordinary subtree matching covers constraints at or above its fixed
    // suffix. An exact excluded base one label below that suffix also overlaps
    // one possible expansion and must therefore reject the wildcard.
    let Some(wildcard_base) = name.strip_prefix("*.") else {
        return false;
    };
    if constraint.subdomains_only || constraint.base.len() <= wildcard_base.len() {
        return false;
    }

    let suffix_offset = constraint.base.len() - wildcard_base.len();
    constraint.base.as_bytes()[suffix_offset - 1] == b'.'
        && constraint.base[suffix_offset..].eq_ignore_ascii_case(wildcard_base)
        && !constraint.base[..suffix_offset - 1].contains('.')
}

/// Decode the supported DER contents of an RFC 5280 NameConstraints SEQUENCE.
///
/// The snapshot accepted subset is one `[0] permittedSubtrees` field containing
/// non-empty dNSName GeneralSubtree values with omitted distance fields.
pub fn decode_name_constraints_contents<'a>(
    decoder: &mut Asn1Decoder,
    encoded_contents: &'a [u8],
) -> Result<NameConstraints<'a>, NameConstraintsError> {
    let root = decoder
        .decode_exact(encoded_contents)
        .map_err(NameConstraintsError::structure)?;
    let (permitted_dns_subtrees, permitted_len) = decode_subtrees_field(
        decoder,
        root,
        0,
        PERMITTED_SUBTREES_TAG,
        NameConstraintsErrorKind::EmptyPermittedSubtrees,
    )?;
    Ok(NameConstraints {
        permitted_dns_subtrees,
        permitted_len,
        excluded_dns_subtrees: [""; MAX_DNS_SUBTREES],
        excluded_len: 0,
    })
}

/// Decode one exact DER-encoded RFC 5280 `NameConstraints` SEQUENCE.
///
/// Certificate extension values contain this complete SEQUENCE rather than
/// only the sequence contents retained by reviewed trust-anchor snapshots. The
/// accepted subset contains a non-empty `[0]` permitted field, a non-empty
/// `[1]` excluded field, or both in schema order, with dNSName subtrees only.
pub fn decode_name_constraints<'a>(
    decoder: &mut Asn1Decoder,
    encoded: &'a [u8],
) -> Result<NameConstraints<'a>, NameConstraintsError> {
    let root = decoder
        .decode_exact(encoded)
        .map_err(NameConstraintsError::structure)?;
    let value_offset = root.header().len();
    let value_length = root.value().len();
    let mut fields = decoder
        .sequence(root)
        .map_err(NameConstraintsError::structure)?;
    let mut decoded = NameConstraints {
        permitted_dns_subtrees: [""; MAX_DNS_SUBTREES],
        permitted_len: 0,
        excluded_dns_subtrees: [""; MAX_DNS_SUBTREES],
        excluded_len: 0,
    };
    let mut seen_permitted = false;
    let mut seen_excluded = false;
    while !fields.remaining().is_empty() {
        let field_offset = child_offset(value_offset, value_length, fields.remaining().len());
        let field = fields
            .read(decoder)
            .map_err(|error| {
                NameConstraintsError::child_structure(error, value_offset, field_offset)
            })?
            .expect("a non-empty DER cursor must yield one element");

        if is_exact_tag(
            field.tag(),
            TagClass::ContextSpecific,
            true,
            PERMITTED_SUBTREES_TAG,
        ) {
            if seen_permitted {
                return Err(NameConstraintsError::new(
                    NameConstraintsErrorKind::DuplicateTopLevelField,
                    field_offset,
                ));
            }
            if seen_excluded {
                return Err(NameConstraintsError::new(
                    NameConstraintsErrorKind::UnsupportedTopLevelFieldOrder,
                    field_offset,
                ));
            }
            let (subtrees, len) = decode_subtrees_field(
                decoder,
                field,
                field_offset,
                PERMITTED_SUBTREES_TAG,
                NameConstraintsErrorKind::EmptyPermittedSubtrees,
            )?;
            decoded.permitted_dns_subtrees = subtrees;
            decoded.permitted_len = len;
            seen_permitted = true;
        } else if is_exact_tag(
            field.tag(),
            TagClass::ContextSpecific,
            true,
            EXCLUDED_SUBTREES_TAG,
        ) {
            if seen_excluded {
                return Err(NameConstraintsError::new(
                    NameConstraintsErrorKind::DuplicateTopLevelField,
                    field_offset,
                ));
            }
            let (subtrees, len) = decode_subtrees_field(
                decoder,
                field,
                field_offset,
                EXCLUDED_SUBTREES_TAG,
                NameConstraintsErrorKind::EmptyExcludedSubtrees,
            )?;
            decoded.excluded_dns_subtrees = subtrees;
            decoded.excluded_len = len;
            seen_excluded = true;
        } else {
            return Err(NameConstraintsError::new(
                NameConstraintsErrorKind::UnsupportedTopLevelField,
                field_offset,
            ));
        }
    }
    fields.finish().map_err(NameConstraintsError::structure)?;
    if !seen_permitted && !seen_excluded {
        return Err(NameConstraintsError::new(
            NameConstraintsErrorKind::UnsupportedTopLevelField,
            value_offset,
        ));
    }
    Ok(decoded)
}

fn decode_subtrees_field<'a>(
    decoder: &mut Asn1Decoder,
    root: Asn1Element<'a>,
    root_offset: usize,
    tag_number: u32,
    empty_kind: NameConstraintsErrorKind,
) -> Result<([&'a str; MAX_DNS_SUBTREES], usize), NameConstraintsError> {
    if !is_exact_tag(root.tag(), TagClass::ContextSpecific, true, tag_number) {
        return Err(NameConstraintsError::new(
            NameConstraintsErrorKind::UnsupportedTopLevelField,
            root_offset,
        ));
    }

    let value_offset = root_offset + root.header().len();
    let value_length = root.value().len();
    let mut subtrees = decoder
        .implicit_constructed(root, tag_number)
        .map_err(|error| NameConstraintsError::element_structure(error, root_offset))?;
    if subtrees.remaining().is_empty() {
        return Err(NameConstraintsError::new(empty_kind, value_offset));
    }

    let mut decoded = [""; MAX_DNS_SUBTREES];
    let mut len = 0;

    while !subtrees.remaining().is_empty() {
        let subtree_offset = child_offset(value_offset, value_length, subtrees.remaining().len());
        if len == MAX_DNS_SUBTREES {
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
        decoded[len] = dns_name;
        len += 1;
    }

    subtrees.finish().map_err(NameConstraintsError::structure)?;
    Ok((decoded, len))
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
