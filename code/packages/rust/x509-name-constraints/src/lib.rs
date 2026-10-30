//! # DNS and IP NameConstraints structure
//!
//! This crate decodes a narrow, bounded RFC 5280 structure and evaluates its
//! permitted and excluded DNS or IP subtrees. Separate DNS-only and IP-only
//! entry points fail closed on the other retained name form, while a combined
//! policy evaluates both against caller-supplied typed names. The crate
//! deliberately grants no certificate-evidence, path, trust-root, or TLS
//! authority.

#![deny(unsafe_code)]

use der_asn1::{
    decode_implicit_ia5_string, decode_implicit_octet_string, Asn1Decoder, Asn1Element, Asn1Error,
    Asn1ErrorKind,
};
use der_tlv::{DerTag, TagClass};
use std::error::Error;
use std::fmt;
use std::net::IpAddr;

const SEQUENCE_TAG: u32 = 16;
const PERMITTED_SUBTREES_TAG: u32 = 0;
const EXCLUDED_SUBTREES_TAG: u32 = 1;
const DNS_NAME_TAG: u32 = 2;
const IP_ADDRESS_TAG: u32 = 7;

/// RFC 5280 id-ce-nameConstraints (2.5.29.30).
pub const NAME_CONSTRAINTS_OID: &[u64] = &[2, 5, 29, 30];

/// Maximum DNS GeneralSubtree values retained from one permitted or excluded field.
pub const MAX_DNS_SUBTREES: usize = 256;
/// Maximum total GeneralSubtree values retained from either top-level field.
pub const MAX_GENERAL_SUBTREES: usize = MAX_DNS_SUBTREES;
/// Maximum presented DNS names evaluated against one constraints value.
pub const MAX_PRESENTED_DNS_NAMES: usize = 256;
/// Maximum presented IP addresses evaluated against one constraints value.
pub const MAX_PRESENTED_IP_ADDRESSES: usize = 256;

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
    UnevaluatedIpSubtrees,
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

/// Stable syntax failures for one encoded IP address-and-mask subtree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpConstraintSyntaxError {
    InvalidLength,
    NonContiguousMask,
    NonCanonicalNetwork,
}

/// Stable payload-free IP constraint evaluation failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpConstraintErrorKind {
    TooManyPresentedIpAddresses,
    UnevaluatedDnsSubtrees,
    InvalidPermittedSubtree(IpConstraintSyntaxError),
    InvalidExcludedSubtree(IpConstraintSyntaxError),
    PresentedIpAddressExcluded,
    PresentedIpAddressNotPermitted,
}

/// One redacted IP evaluation failure and its relevant collection index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IpConstraintError {
    kind: IpConstraintErrorKind,
    item_index: usize,
}

impl IpConstraintError {
    const fn new(kind: IpConstraintErrorKind, item_index: usize) -> Self {
        Self { kind, item_index }
    }

    pub const fn kind(self) -> IpConstraintErrorKind {
        self.kind
    }

    pub const fn item_index(self) -> usize {
        self.item_index
    }
}

impl fmt::Display for IpConstraintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 IP constraint error {:?} at item {}",
            self.kind, self.item_index
        )
    }
}

impl Error for IpConstraintError {}

/// Payload-free evidence that all supplied IP addresses satisfied the constraints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstrainedIpAddresses {
    count: usize,
}

impl ConstrainedIpAddresses {
    pub const fn count(self) -> usize {
        self.count
    }

    pub const fn is_empty(self) -> bool {
        self.count == 0
    }
}

/// Stable payload-free failures from combined DNS and IP evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameConstraintEvaluationErrorKind {
    Dns(DnsConstraintErrorKind),
    Ip(IpConstraintErrorKind),
}

/// One redacted combined evaluation failure and its relevant collection index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameConstraintEvaluationError {
    kind: NameConstraintEvaluationErrorKind,
    item_index: usize,
}

impl NameConstraintEvaluationError {
    const fn dns(error: DnsConstraintError) -> Self {
        Self {
            kind: NameConstraintEvaluationErrorKind::Dns(error.kind()),
            item_index: error.item_index(),
        }
    }

    const fn ip(error: IpConstraintError) -> Self {
        Self {
            kind: NameConstraintEvaluationErrorKind::Ip(error.kind()),
            item_index: error.item_index(),
        }
    }

    pub const fn kind(self) -> NameConstraintEvaluationErrorKind {
        self.kind
    }

    pub const fn item_index(self) -> usize {
        self.item_index
    }
}

impl fmt::Display for NameConstraintEvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 combined name constraint error {:?} at item {}",
            self.kind, self.item_index
        )
    }
}

impl Error for NameConstraintEvaluationError {}

/// Payload-free evidence that both supported name forms satisfied the constraints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstrainedDnsAndIpNames {
    dns_count: usize,
    ip_count: usize,
}

impl ConstrainedDnsAndIpNames {
    pub const fn dns_count(self) -> usize {
        self.dns_count
    }

    pub const fn ip_count(self) -> usize {
        self.ip_count
    }

    pub const fn total_count(self) -> usize {
        self.dns_count + self.ip_count
    }

    pub const fn is_empty(self) -> bool {
        self.dns_count == 0 && self.ip_count == 0
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
    InvalidIpSubtreeLength,
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

/// Borrowed DNS and IP subtrees from one supported constraints value.
#[derive(Clone)]
pub struct NameConstraints<'a> {
    permitted_dns_subtrees: [&'a str; MAX_DNS_SUBTREES],
    permitted_len: usize,
    permitted_ip_subtrees: [&'a [u8]; MAX_GENERAL_SUBTREES],
    permitted_ip_len: usize,
    permitted_total_len: usize,
    excluded_dns_subtrees: [&'a str; MAX_DNS_SUBTREES],
    excluded_len: usize,
    excluded_ip_subtrees: [&'a [u8]; MAX_GENERAL_SUBTREES],
    excluded_ip_len: usize,
    excluded_total_len: usize,
}

impl fmt::Debug for NameConstraints<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NameConstraints")
            .field("permitted_dns_subtrees", &self.permitted_len)
            .field("permitted_ip_subtrees", &self.permitted_ip_len)
            .field("excluded_dns_subtrees", &self.excluded_len)
            .field("excluded_ip_subtrees", &self.excluded_ip_len)
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

    /// Return permitted IP address-and-mask values in wire order relative to
    /// other permitted IP subtrees.
    pub fn permitted_ip_subtrees(&self) -> &[&'a [u8]] {
        &self.permitted_ip_subtrees[..self.permitted_ip_len]
    }

    /// Return excluded IP address-and-mask values in wire order relative to
    /// other excluded IP subtrees.
    pub fn excluded_ip_subtrees(&self) -> &[&'a [u8]] {
        &self.excluded_ip_subtrees[..self.excluded_ip_len]
    }

    /// Return the number of retained permitted DNS subtrees.
    pub const fn permitted_len(&self) -> usize {
        self.permitted_len
    }

    /// Return the number of retained excluded DNS subtrees.
    pub const fn excluded_len(&self) -> usize {
        self.excluded_len
    }

    /// Return the number of retained permitted IP subtrees.
    pub const fn permitted_ip_len(&self) -> usize {
        self.permitted_ip_len
    }

    /// Return the number of retained excluded IP subtrees.
    pub const fn excluded_ip_len(&self) -> usize {
        self.excluded_ip_len
    }

    /// Return the total number of retained DNS and IP subtrees.
    pub const fn len(&self) -> usize {
        self.permitted_total_len + self.excluded_total_len
    }

    /// Return whether no DNS or IP subtree was retained.
    ///
    /// Successful decoding is never empty; this method keeps collection use
    /// explicit for downstream policy code.
    pub const fn is_empty(&self) -> bool {
        self.permitted_total_len == 0 && self.excluded_total_len == 0
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
    if constraints.permitted_ip_len() != 0 || constraints.excluded_ip_len() != 0 {
        return Err(DnsConstraintError::new(
            DnsConstraintErrorKind::UnevaluatedIpSubtrees,
            0,
        ));
    }

    evaluate_dns_names_inner(constraints, presented_dns_names)
}

fn evaluate_dns_names_inner(
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

/// Evaluate presented IP subjectAltName values against IP-only constraints.
///
/// This entry point refuses retained DNS subtrees so a caller cannot silently
/// skip that name form. Use [`evaluate_dns_and_ip_names`] for mixed values.
pub fn evaluate_ip_addresses(
    constraints: &NameConstraints<'_>,
    presented_ip_addresses: &[IpAddr],
) -> Result<ConstrainedIpAddresses, IpConstraintError> {
    if presented_ip_addresses.len() > MAX_PRESENTED_IP_ADDRESSES {
        return Err(IpConstraintError::new(
            IpConstraintErrorKind::TooManyPresentedIpAddresses,
            MAX_PRESENTED_IP_ADDRESSES,
        ));
    }
    if constraints.permitted_len() != 0 || constraints.excluded_len() != 0 {
        return Err(IpConstraintError::new(
            IpConstraintErrorKind::UnevaluatedDnsSubtrees,
            0,
        ));
    }

    evaluate_ip_addresses_inner(constraints, presented_ip_addresses)
}

/// Evaluate both supported GeneralName forms without silently dropping either.
pub fn evaluate_dns_and_ip_names(
    constraints: &NameConstraints<'_>,
    presented_dns_names: &[&str],
    presented_ip_addresses: &[IpAddr],
) -> Result<ConstrainedDnsAndIpNames, NameConstraintEvaluationError> {
    // Validate every retained constraint before examining either presented
    // collection. The second calls below repeat that bounded validation while
    // applying the names, keeping the internal evidence local and unforgeable.
    evaluate_dns_names_inner(constraints, &[]).map_err(NameConstraintEvaluationError::dns)?;
    evaluate_ip_addresses_inner(constraints, &[]).map_err(NameConstraintEvaluationError::ip)?;

    let dns = evaluate_dns_names_inner(constraints, presented_dns_names)
        .map_err(NameConstraintEvaluationError::dns)?;
    let ip = evaluate_ip_addresses_inner(constraints, presented_ip_addresses)
        .map_err(NameConstraintEvaluationError::ip)?;
    Ok(ConstrainedDnsAndIpNames {
        dns_count: dns.count(),
        ip_count: ip.count(),
    })
}

fn evaluate_ip_addresses_inner(
    constraints: &NameConstraints<'_>,
    presented_ip_addresses: &[IpAddr],
) -> Result<ConstrainedIpAddresses, IpConstraintError> {
    if presented_ip_addresses.len() > MAX_PRESENTED_IP_ADDRESSES {
        return Err(IpConstraintError::new(
            IpConstraintErrorKind::TooManyPresentedIpAddresses,
            MAX_PRESENTED_IP_ADDRESSES,
        ));
    }

    validate_ip_subtrees(
        constraints.permitted_ip_subtrees(),
        IpSubtreeClass::Permitted,
    )?;
    validate_ip_subtrees(constraints.excluded_ip_subtrees(), IpSubtreeClass::Excluded)?;

    for (index, address) in presented_ip_addresses.iter().enumerate() {
        let excluded = constraints.excluded_ip_subtrees().iter().any(|encoded| {
            validate_ip_subtree(encoded).is_ok_and(|subtree| ip_address_matches(address, subtree))
        });
        if excluded {
            return Err(IpConstraintError::new(
                IpConstraintErrorKind::PresentedIpAddressExcluded,
                index,
            ));
        }

        let permitted = constraints.permitted_ip_len() == 0
            || constraints.permitted_ip_subtrees().iter().any(|encoded| {
                validate_ip_subtree(encoded)
                    .is_ok_and(|subtree| ip_address_matches(address, subtree))
            });
        if !permitted {
            return Err(IpConstraintError::new(
                IpConstraintErrorKind::PresentedIpAddressNotPermitted,
                index,
            ));
        }
    }

    Ok(ConstrainedIpAddresses {
        count: presented_ip_addresses.len(),
    })
}

#[derive(Clone, Copy)]
enum IpSubtreeClass {
    Permitted,
    Excluded,
}

fn validate_ip_subtrees(
    subtrees: &[&[u8]],
    class: IpSubtreeClass,
) -> Result<(), IpConstraintError> {
    for (index, encoded) in subtrees.iter().enumerate() {
        validate_ip_subtree(encoded).map_err(|reason| {
            let kind = match class {
                IpSubtreeClass::Permitted => IpConstraintErrorKind::InvalidPermittedSubtree(reason),
                IpSubtreeClass::Excluded => IpConstraintErrorKind::InvalidExcludedSubtree(reason),
            };
            IpConstraintError::new(kind, index)
        })?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ValidIpSubtree<'a> {
    network: &'a [u8],
    mask: &'a [u8],
}

fn validate_ip_subtree(encoded: &[u8]) -> Result<ValidIpSubtree<'_>, IpConstraintSyntaxError> {
    let address_length = match encoded.len() {
        8 => 4,
        32 => 16,
        _ => return Err(IpConstraintSyntaxError::InvalidLength),
    };
    let (network, mask) = encoded.split_at(address_length);

    let mut zero_seen = false;
    for byte in mask {
        for shift in (0..8).rev() {
            let set = byte & (1 << shift) != 0;
            if set && zero_seen {
                return Err(IpConstraintSyntaxError::NonContiguousMask);
            }
            zero_seen |= !set;
        }
    }
    if network
        .iter()
        .zip(mask)
        .any(|(address_byte, mask_byte)| address_byte & !mask_byte != 0)
    {
        return Err(IpConstraintSyntaxError::NonCanonicalNetwork);
    }

    Ok(ValidIpSubtree { network, mask })
}

fn ip_address_matches(address: &IpAddr, subtree: ValidIpSubtree<'_>) -> bool {
    match address {
        IpAddr::V4(address) => ip_octets_match(&address.octets(), subtree),
        IpAddr::V6(address) => ip_octets_match(&address.octets(), subtree),
    }
}

fn ip_octets_match(address: &[u8], subtree: ValidIpSubtree<'_>) -> bool {
    address.len() == subtree.network.len()
        && address.iter().zip(subtree.network).zip(subtree.mask).all(
            |((address_byte, network_byte), mask_byte)| {
                address_byte & mask_byte == network_byte & mask_byte
            },
        )
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
    let permitted = decode_subtrees_field(
        decoder,
        root,
        0,
        PERMITTED_SUBTREES_TAG,
        NameConstraintsErrorKind::EmptyPermittedSubtrees,
        false,
    )?;
    Ok(NameConstraints {
        permitted_dns_subtrees: permitted.dns_subtrees,
        permitted_len: permitted.dns_len,
        permitted_ip_subtrees: permitted.ip_subtrees,
        permitted_ip_len: permitted.ip_len,
        permitted_total_len: permitted.total_len,
        excluded_dns_subtrees: [""; MAX_DNS_SUBTREES],
        excluded_len: 0,
        excluded_ip_subtrees: [&[]; MAX_GENERAL_SUBTREES],
        excluded_ip_len: 0,
        excluded_total_len: 0,
    })
}

/// Decode one exact DER-encoded RFC 5280 `NameConstraints` SEQUENCE.
///
/// Certificate extension values contain this complete SEQUENCE rather than
/// only the sequence contents retained by reviewed trust-anchor snapshots. The
/// accepted subset contains a non-empty `[0]` permitted field, a non-empty
/// `[1]` excluded field, or both in schema order, with dNSName and iPAddress
/// subtrees only.
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
        permitted_ip_subtrees: [&[]; MAX_GENERAL_SUBTREES],
        permitted_ip_len: 0,
        permitted_total_len: 0,
        excluded_dns_subtrees: [""; MAX_DNS_SUBTREES],
        excluded_len: 0,
        excluded_ip_subtrees: [&[]; MAX_GENERAL_SUBTREES],
        excluded_ip_len: 0,
        excluded_total_len: 0,
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
            let subtrees = decode_subtrees_field(
                decoder,
                field,
                field_offset,
                PERMITTED_SUBTREES_TAG,
                NameConstraintsErrorKind::EmptyPermittedSubtrees,
                true,
            )?;
            decoded.permitted_dns_subtrees = subtrees.dns_subtrees;
            decoded.permitted_len = subtrees.dns_len;
            decoded.permitted_ip_subtrees = subtrees.ip_subtrees;
            decoded.permitted_ip_len = subtrees.ip_len;
            decoded.permitted_total_len = subtrees.total_len;
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
            let subtrees = decode_subtrees_field(
                decoder,
                field,
                field_offset,
                EXCLUDED_SUBTREES_TAG,
                NameConstraintsErrorKind::EmptyExcludedSubtrees,
                true,
            )?;
            decoded.excluded_dns_subtrees = subtrees.dns_subtrees;
            decoded.excluded_len = subtrees.dns_len;
            decoded.excluded_ip_subtrees = subtrees.ip_subtrees;
            decoded.excluded_ip_len = subtrees.ip_len;
            decoded.excluded_total_len = subtrees.total_len;
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

struct DecodedSubtrees<'a> {
    dns_subtrees: [&'a str; MAX_DNS_SUBTREES],
    dns_len: usize,
    ip_subtrees: [&'a [u8]; MAX_GENERAL_SUBTREES],
    ip_len: usize,
    total_len: usize,
}

fn decode_subtrees_field<'a>(
    decoder: &mut Asn1Decoder,
    root: Asn1Element<'a>,
    root_offset: usize,
    tag_number: u32,
    empty_kind: NameConstraintsErrorKind,
    allow_ip: bool,
) -> Result<DecodedSubtrees<'a>, NameConstraintsError> {
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

    let mut decoded = DecodedSubtrees {
        dns_subtrees: [""; MAX_DNS_SUBTREES],
        dns_len: 0,
        ip_subtrees: [&[]; MAX_GENERAL_SUBTREES],
        ip_len: 0,
        total_len: 0,
    };

    while !subtrees.remaining().is_empty() {
        let subtree_offset = child_offset(value_offset, value_length, subtrees.remaining().len());
        if decoded.total_len == MAX_GENERAL_SUBTREES {
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

        match decode_general_subtree(decoder, subtree, subtree_offset, allow_ip)? {
            GeneralSubtree::Dns(dns_name) => {
                decoded.dns_subtrees[decoded.dns_len] = dns_name;
                decoded.dns_len += 1;
            }
            GeneralSubtree::Ip(ip_address) => {
                decoded.ip_subtrees[decoded.ip_len] = ip_address;
                decoded.ip_len += 1;
            }
        }
        decoded.total_len += 1;
    }

    subtrees.finish().map_err(NameConstraintsError::structure)?;
    Ok(decoded)
}

enum GeneralSubtree<'a> {
    Dns(&'a str),
    Ip(&'a [u8]),
}

fn decode_general_subtree<'a>(
    decoder: &mut Asn1Decoder,
    subtree: Asn1Element<'a>,
    subtree_offset: usize,
    allow_ip: bool,
) -> Result<GeneralSubtree<'a>, NameConstraintsError> {
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

    let decoded_base = if is_exact_tag(base.tag(), TagClass::ContextSpecific, false, DNS_NAME_TAG) {
        if base.value().is_empty() {
            return Err(NameConstraintsError::new(
                NameConstraintsErrorKind::EmptyDnsSubtree,
                base_offset + base.header().len(),
            ));
        }
        let dns_name = decode_implicit_ia5_string(base, DNS_NAME_TAG)
            .map_err(|error| NameConstraintsError::dns(error, base_offset))?;
        GeneralSubtree::Dns(dns_name)
    } else if allow_ip && is_exact_tag(base.tag(), TagClass::ContextSpecific, false, IP_ADDRESS_TAG)
    {
        let ip_address = decode_implicit_octet_string(base, IP_ADDRESS_TAG)
            .map_err(|error| NameConstraintsError::element_structure(error, base_offset))?;
        if !matches!(ip_address.len(), 8 | 32) {
            return Err(NameConstraintsError::new(
                NameConstraintsErrorKind::InvalidIpSubtreeLength,
                base_offset + base.header().len(),
            ));
        }
        GeneralSubtree::Ip(ip_address)
    } else {
        return Err(NameConstraintsError::new(
            NameConstraintsErrorKind::UnsupportedGeneralName,
            base_offset,
        ));
    };

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
    Ok(decoded_base)
}

fn is_exact_tag(tag: DerTag, class: TagClass, constructed: bool, number: u32) -> bool {
    tag.class == class && tag.constructed == constructed && tag.number == number
}

const fn child_offset(value_offset: usize, value_length: usize, remaining_length: usize) -> usize {
    value_offset + (value_length - remaining_length)
}
