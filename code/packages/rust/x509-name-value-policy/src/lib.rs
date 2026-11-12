//! # Data-driven X.509 Name attribute-value policy
//!
//! This crate binds validated caller-supplied attribute-type OIDs to a small
//! repository-owned set of comparison rules. It removes executable value
//! callbacks from later certificate-link callers without embedding an
//! attribute registry or claiming complete RFC 5280 Name equality.

#![deny(unsafe_code)]

use der_asn1::ObjectIdentifier;
use std::error::Error;
use std::fmt;
use x509_directory_string::{
    compare_x509_directory_strings_by_scalar, decode_x509_directory_string,
    X509DirectoryStringComparisonError,
};
use x509_name::{X509AttributeTypeAndValue, X509Name};
use x509_name_comparison::compare_x509_names_by;

/// Maximum number of attribute-type rules in one policy.
pub const MAX_X509_NAME_VALUE_POLICY_ENTRIES: usize = 64;

/// One repository-owned value-comparison rule selected by policy data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509NameValueComparisonRule {
    /// Require the complete canonical DER elements to be byte-for-byte equal.
    ExactDer,
    /// Decode both values as DirectoryString and compare exact Unicode scalars.
    DirectoryStringScalar,
}

/// One caller-supplied attribute-type rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509NameValuePolicyEntry<'a> {
    attribute_type: &'a [u64],
    rule: X509NameValueComparisonRule,
}

impl<'a> X509NameValuePolicyEntry<'a> {
    pub const fn new(attribute_type: &'a [u64], rule: X509NameValueComparisonRule) -> Self {
        Self {
            attribute_type,
            rule,
        }
    }

    pub const fn attribute_type(self) -> &'a [u64] {
        self.attribute_type
    }

    pub const fn rule(self) -> X509NameValueComparisonRule {
        self.rule
    }
}

/// Stable policy-definition failures. No category retains OID arcs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509NameValuePolicyDefinitionErrorKind {
    TooManyEntries,
    InvalidAttributeType,
    DuplicateAttributeType,
}

/// A payload-free invalid-policy error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509NameValuePolicyDefinitionError {
    kind: X509NameValuePolicyDefinitionErrorKind,
}

impl X509NameValuePolicyDefinitionError {
    const fn new(kind: X509NameValuePolicyDefinitionErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509NameValuePolicyDefinitionErrorKind {
        self.kind
    }
}

impl fmt::Display for X509NameValuePolicyDefinitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 Name value policy definition error {:?}",
            self.kind
        )
    }
}

impl Error for X509NameValuePolicyDefinitionError {}

/// Stable comparison failures. No category retains certificate or OID data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X509NameValueComparisonErrorKind {
    UnconfiguredAttributeType,
    InvalidDirectoryString,
    UnsupportedTeletexString,
}

/// A payload-free fail-closed comparison error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509NameValueComparisonError {
    kind: X509NameValueComparisonErrorKind,
}

impl X509NameValueComparisonError {
    const fn new(kind: X509NameValueComparisonErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509NameValueComparisonErrorKind {
        self.kind
    }
}

impl fmt::Display for X509NameValueComparisonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 Name value comparison error {:?}",
            self.kind
        )
    }
}

impl Error for X509NameValueComparisonError {}

/// One validated, bounded caller-supplied attribute-value policy.
///
/// The policy borrows its definition and allocates nothing. An empty policy is
/// valid and rejects every non-empty Name as unconfigured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X509NameValuePolicy<'a> {
    entries: &'a [X509NameValuePolicyEntry<'a>],
}

impl<'a> X509NameValuePolicy<'a> {
    pub fn new(
        entries: &'a [X509NameValuePolicyEntry<'a>],
    ) -> Result<Self, X509NameValuePolicyDefinitionError> {
        if entries.len() > MAX_X509_NAME_VALUE_POLICY_ENTRIES {
            return Err(X509NameValuePolicyDefinitionError::new(
                X509NameValuePolicyDefinitionErrorKind::TooManyEntries,
            ));
        }
        for (index, entry) in entries.iter().copied().enumerate() {
            if !valid_oid_arcs(entry.attribute_type) {
                return Err(X509NameValuePolicyDefinitionError::new(
                    X509NameValuePolicyDefinitionErrorKind::InvalidAttributeType,
                ));
            }
            if entries[..index]
                .iter()
                .any(|earlier| earlier.attribute_type == entry.attribute_type)
            {
                return Err(X509NameValuePolicyDefinitionError::new(
                    X509NameValuePolicyDefinitionErrorKind::DuplicateAttributeType,
                ));
            }
        }
        Ok(Self { entries })
    }

    pub const fn entries(self) -> &'a [X509NameValuePolicyEntry<'a>] {
        self.entries
    }

    /// Compare two already decoded Names under this exact policy.
    ///
    /// Every attribute in both Names is preflighted before structural matching.
    /// Missing rules fail closed. For attributes configured with
    /// [`X509NameValueComparisonRule::DirectoryStringScalar`], malformed
    /// DirectoryString values and unsupported TeletexString values also fail
    /// closed even when another structural mismatch would otherwise avoid the
    /// value callback. [`X509NameValueComparisonRule::ExactDer`] intentionally
    /// treats the complete canonical value element as opaque policy data.
    pub fn compare_names(
        self,
        left: &X509Name<'_>,
        right: &X509Name<'_>,
    ) -> Result<bool, X509NameValueComparisonError> {
        self.preflight_name(left)?;
        self.preflight_name(right)?;

        let mut failure = None;
        let equal = compare_x509_names_by(left, right, |left, right| {
            match self.compare_values(left, right) {
                Ok(equal) => equal,
                Err(error) => {
                    failure.get_or_insert(error);
                    false
                }
            }
        });
        failure.map_or(Ok(equal), Err)
    }

    fn preflight_name(self, name: &X509Name<'_>) -> Result<(), X509NameValueComparisonError> {
        for rdn in name.rdns() {
            for attribute in rdn.iter() {
                let rule = self.rule_for(attribute.attribute_type())?;
                if rule == X509NameValueComparisonRule::DirectoryStringScalar {
                    let value = decode_x509_directory_string(attribute.value()).map_err(|_| {
                        X509NameValueComparisonError::new(
                            X509NameValueComparisonErrorKind::InvalidDirectoryString,
                        )
                    })?;
                    if value.unicode_scalars().is_none() {
                        return Err(X509NameValueComparisonError::new(
                            X509NameValueComparisonErrorKind::UnsupportedTeletexString,
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn compare_values(
        self,
        left: X509AttributeTypeAndValue<'_>,
        right: X509AttributeTypeAndValue<'_>,
    ) -> Result<bool, X509NameValueComparisonError> {
        match self.rule_for(left.attribute_type())? {
            X509NameValueComparisonRule::ExactDer => {
                Ok(left.value().encoded() == right.value().encoded())
            }
            X509NameValueComparisonRule::DirectoryStringScalar => {
                let left = decode_x509_directory_string(left.value()).map_err(|_| {
                    X509NameValueComparisonError::new(
                        X509NameValueComparisonErrorKind::InvalidDirectoryString,
                    )
                })?;
                let right = decode_x509_directory_string(right.value()).map_err(|_| {
                    X509NameValueComparisonError::new(
                        X509NameValueComparisonErrorKind::InvalidDirectoryString,
                    )
                })?;
                compare_x509_directory_strings_by_scalar(left, right).map_err(|error| match error {
                    X509DirectoryStringComparisonError::UnsupportedTeletexString => {
                        X509NameValueComparisonError::new(
                            X509NameValueComparisonErrorKind::UnsupportedTeletexString,
                        )
                    }
                })
            }
        }
    }

    fn rule_for(
        self,
        attribute_type: ObjectIdentifier<'_>,
    ) -> Result<X509NameValueComparisonRule, X509NameValueComparisonError> {
        self.entries
            .iter()
            .find(|entry| attribute_type.equals(entry.attribute_type))
            .map(|entry| entry.rule)
            .ok_or_else(|| {
                X509NameValueComparisonError::new(
                    X509NameValueComparisonErrorKind::UnconfiguredAttributeType,
                )
            })
    }
}

const fn valid_oid_arcs(identifier: &[u64]) -> bool {
    if identifier.len() < 2 || identifier[0] > 2 {
        return false;
    }
    identifier[0] == 2 || identifier[1] < 40
}
