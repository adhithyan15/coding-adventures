//! # Data-driven RFC 5280 distinguished-name comparison
//!
//! A distinguished name is an ordered sequence of RDNs, while each RDN is an
//! unordered set of attributes. This crate owns that bounded structural rule
//! and exact attribute-type matching. Attribute-value meaning stays with a
//! caller-supplied equivalence policy, because exact DER equality is not the
//! same claim as complete RFC 5280 name equality.

#![deny(unsafe_code)]

use x509_name::{X509AttributeTypeAndValue, X509Name};

/// Compare two decoded distinguished names with caller-owned value semantics.
///
/// RDN sequence order is significant. Attribute order within each RDN is not:
/// every left attribute must find one still-unmatched right attribute with the
/// same canonical OID and a value accepted by `values_equal`. The callback may
/// therefore run more than once for a left attribute when an RDN repeats an
/// attribute type. It must implement a stable equivalence relation.
///
/// This function performs no allocation and retains no input. It does not
/// decode or normalize DirectoryString values, select issuers, construct a
/// certificate path, or establish trust.
pub fn compare_x509_names_by<'left, 'right>(
    left: &X509Name<'left>,
    right: &X509Name<'right>,
    mut values_equal: impl FnMut(
        X509AttributeTypeAndValue<'left>,
        X509AttributeTypeAndValue<'right>,
    ) -> bool,
) -> bool {
    compare_x509_names_with(left, right, &mut values_equal)
}

fn compare_x509_names_with<'left, 'right>(
    left: &X509Name<'left>,
    right: &X509Name<'right>,
    values_equal: &mut dyn FnMut(
        X509AttributeTypeAndValue<'left>,
        X509AttributeTypeAndValue<'right>,
    ) -> bool,
) -> bool {
    if left.rdn_count() != right.rdn_count() {
        return false;
    }

    for (left_rdn, right_rdn) in left.rdns().zip(right.rdns()) {
        if left_rdn.len() != right_rdn.len() {
            return false;
        }

        // One Name contains at most 64 attributes, so one u64 can track every
        // possible member of this RDN without allocating from wire data.
        let mut matched_right = 0u64;
        for left_attribute in left_rdn.iter() {
            let mut found = false;
            for (right_index, right_attribute) in right_rdn.iter().enumerate() {
                let right_mask = 1u64 << right_index;
                if matched_right & right_mask != 0
                    || left_attribute.attribute_type().encoded()
                        != right_attribute.attribute_type().encoded()
                {
                    continue;
                }
                if values_equal(left_attribute, right_attribute) {
                    matched_right |= right_mask;
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }
    }

    true
}
