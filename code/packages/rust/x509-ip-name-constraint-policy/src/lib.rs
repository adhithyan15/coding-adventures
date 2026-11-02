//! # Bounded IP-address NameConstraints policy
//!
//! RFC 5280 encodes an IPv4 name constraint as four address octets followed
//! by four mask octets, and an IPv6 constraint as sixteen address octets
//! followed by sixteen mask octets. This crate validates those public values
//! as CIDR ranges and evaluates caller-supplied IP Subject Alternative Name
//! octets. DER parsing and certificate/path evidence remain separate.

#![deny(unsafe_code)]

use core::fmt;

/// Maximum permitted constraints evaluated in one call.
pub const MAX_PERMITTED_IP_CONSTRAINTS: usize = 256;
/// Maximum excluded constraints evaluated in one call.
pub const MAX_EXCLUDED_IP_CONSTRAINTS: usize = 256;
/// Maximum presented IP addresses evaluated in one call.
pub const MAX_PRESENTED_IP_ADDRESSES: usize = 256;

/// Stable syntax failures for one encoded IP constraint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpConstraintSyntaxError {
    /// The value was neither 8 octets for IPv4 nor 32 octets for IPv6.
    InvalidLength,
    /// The mask was not a contiguous sequence of one bits followed by zeroes.
    NonContiguousMask,
}

/// Stable syntax failures for one presented IP address.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentedIpAddressSyntaxError {
    /// The value was neither 4 octets for IPv4 nor 16 octets for IPv6.
    InvalidLength,
}

/// Stable payload-free evaluation failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpNameConstraintErrorKind {
    TooManyPermittedConstraints,
    TooManyExcludedConstraints,
    TooManyPresentedAddresses,
    InvalidPermittedConstraint(IpConstraintSyntaxError),
    InvalidExcludedConstraint(IpConstraintSyntaxError),
    InvalidPresentedAddress(PresentedIpAddressSyntaxError),
    PresentedAddressExcluded,
    PresentedAddressNotPermitted,
}

/// One redacted failure and its relevant collection index.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct IpNameConstraintError {
    kind: IpNameConstraintErrorKind,
    item_index: usize,
}

impl IpNameConstraintError {
    const fn new(kind: IpNameConstraintErrorKind, item_index: usize) -> Self {
        Self { kind, item_index }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> IpNameConstraintErrorKind {
        self.kind
    }

    /// Return the index within the collection identified by the category.
    pub const fn item_index(self) -> usize {
        self.item_index
    }
}

impl fmt::Debug for IpNameConstraintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IpNameConstraintError")
            .field("kind", &self.kind)
            .field("item_index", &self.item_index)
            .finish()
    }
}

impl fmt::Display for IpNameConstraintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 IP name constraint error {:?} at item {}",
            self.kind, self.item_index
        )
    }
}

impl std::error::Error for IpNameConstraintError {}

/// Payload-free evidence that every supplied address satisfied the policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConstrainedIpAddresses {
    count: usize,
}

impl ConstrainedIpAddresses {
    /// Return the number of presented addresses evaluated.
    pub const fn count(self) -> usize {
        self.count
    }

    /// Return whether the evaluated collection was empty.
    pub const fn is_empty(self) -> bool {
        self.count == 0
    }
}

#[derive(Clone, Copy)]
struct ValidatedConstraint<'a> {
    address: &'a [u8],
    mask: &'a [u8],
}

impl ValidatedConstraint<'_> {
    const EMPTY: Self = Self {
        address: &[],
        mask: &[],
    };
}

/// Evaluate presented IP addresses against permitted and excluded subnets.
///
/// Each constraint is the complete RFC 5280 `iPAddress` GeneralName value:
/// address octets followed by an equal-width CIDR mask. Each presented value
/// is only the exact address octets from Subject Alternative Name. All
/// constraints are validated before any presented address is accepted.
pub fn evaluate_ip_name_constraints(
    permitted_constraints: &[&[u8]],
    excluded_constraints: &[&[u8]],
    presented_addresses: &[&[u8]],
) -> Result<ConstrainedIpAddresses, IpNameConstraintError> {
    enforce_capacity(
        permitted_constraints.len(),
        MAX_PERMITTED_IP_CONSTRAINTS,
        IpNameConstraintErrorKind::TooManyPermittedConstraints,
    )?;
    enforce_capacity(
        excluded_constraints.len(),
        MAX_EXCLUDED_IP_CONSTRAINTS,
        IpNameConstraintErrorKind::TooManyExcludedConstraints,
    )?;
    enforce_capacity(
        presented_addresses.len(),
        MAX_PRESENTED_IP_ADDRESSES,
        IpNameConstraintErrorKind::TooManyPresentedAddresses,
    )?;

    let mut permitted = [ValidatedConstraint::EMPTY; MAX_PERMITTED_IP_CONSTRAINTS];
    for (index, value) in permitted_constraints.iter().enumerate() {
        permitted[index] = validate_constraint(value).map_err(|reason| {
            IpNameConstraintError::new(
                IpNameConstraintErrorKind::InvalidPermittedConstraint(reason),
                index,
            )
        })?;
    }

    let mut excluded = [ValidatedConstraint::EMPTY; MAX_EXCLUDED_IP_CONSTRAINTS];
    for (index, value) in excluded_constraints.iter().enumerate() {
        excluded[index] = validate_constraint(value).map_err(|reason| {
            IpNameConstraintError::new(
                IpNameConstraintErrorKind::InvalidExcludedConstraint(reason),
                index,
            )
        })?;
    }

    for (index, presented) in presented_addresses.iter().enumerate() {
        validate_presented_address(presented).map_err(|reason| {
            IpNameConstraintError::new(
                IpNameConstraintErrorKind::InvalidPresentedAddress(reason),
                index,
            )
        })?;

        if excluded[..excluded_constraints.len()]
            .iter()
            .any(|constraint| constraint.matches(presented))
        {
            return Err(IpNameConstraintError::new(
                IpNameConstraintErrorKind::PresentedAddressExcluded,
                index,
            ));
        }

        if !permitted_constraints.is_empty()
            && !permitted[..permitted_constraints.len()]
                .iter()
                .any(|constraint| constraint.matches(presented))
        {
            return Err(IpNameConstraintError::new(
                IpNameConstraintErrorKind::PresentedAddressNotPermitted,
                index,
            ));
        }
    }

    Ok(ConstrainedIpAddresses {
        count: presented_addresses.len(),
    })
}

fn enforce_capacity(
    actual: usize,
    maximum: usize,
    kind: IpNameConstraintErrorKind,
) -> Result<(), IpNameConstraintError> {
    if actual > maximum {
        Err(IpNameConstraintError::new(kind, maximum))
    } else {
        Ok(())
    }
}

fn validate_constraint(value: &[u8]) -> Result<ValidatedConstraint<'_>, IpConstraintSyntaxError> {
    let address_length = match value.len() {
        8 => 4,
        32 => 16,
        _ => return Err(IpConstraintSyntaxError::InvalidLength),
    };
    let (address, mask) = value.split_at(address_length);
    if !is_contiguous_mask(mask) {
        return Err(IpConstraintSyntaxError::NonContiguousMask);
    }
    Ok(ValidatedConstraint { address, mask })
}

fn validate_presented_address(value: &[u8]) -> Result<(), PresentedIpAddressSyntaxError> {
    match value.len() {
        4 | 16 => Ok(()),
        _ => Err(PresentedIpAddressSyntaxError::InvalidLength),
    }
}

fn is_contiguous_mask(mask: &[u8]) -> bool {
    let mut saw_zero = false;
    for octet in mask {
        for bit in (0..8).rev() {
            let is_one = octet & (1 << bit) != 0;
            if saw_zero && is_one {
                return false;
            }
            saw_zero |= !is_one;
        }
    }
    true
}

impl ValidatedConstraint<'_> {
    fn matches(self, presented: &[u8]) -> bool {
        presented.len() == self.address.len()
            && presented
                .iter()
                .zip(self.address.iter().zip(self.mask))
                .all(|(presented, (base, mask))| presented & mask == base & mask)
    }
}
