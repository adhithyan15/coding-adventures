use der_asn1::{Asn1Decoder, Asn1Limits};
use std::cell::Cell;
use x509_name::{decode_x509_name, X509AttributeTypeAndValue, X509Name};
use x509_name_comparison::compare_x509_names_by;

const COMMON_NAME: &[u8] = &[0x55, 0x04, 0x03];
const COUNTRY_NAME: &[u8] = &[0x55, 0x04, 0x06];

fn der(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(value.len() + 4);
    encoded.push(tag);
    match value.len() {
        length @ 0..=127 => encoded.push(length as u8),
        length @ 128..=255 => encoded.extend([0x81, length as u8]),
        length @ 256..=65_535 => {
            encoded.extend([0x82, (length >> 8) as u8, length as u8]);
        }
        _ => panic!("test value too large"),
    }
    encoded.extend(value);
    encoded
}

fn attribute(oid: &[u8], value_tag: u8, value: &[u8]) -> Vec<u8> {
    let mut fields = der(0x06, oid);
    fields.extend(der(value_tag, value));
    der(0x30, &fields)
}

fn rdn(mut attributes: Vec<Vec<u8>>) -> Vec<u8> {
    attributes.sort();
    der(0x31, &attributes.concat())
}

fn name(rdns: Vec<Vec<u8>>) -> Vec<u8> {
    der(0x30, &rdns.concat())
}

fn decode(encoded: &[u8]) -> X509Name<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    decode_x509_name(&mut decoder, root).unwrap()
}

fn exact_value(left: X509AttributeTypeAndValue<'_>, right: X509AttributeTypeAndValue<'_>) -> bool {
    left.value().encoded() == right.value().encoded()
}

fn text_bytes(left: X509AttributeTypeAndValue<'_>, right: X509AttributeTypeAndValue<'_>) -> bool {
    left.value().value() == right.value().value()
}

#[test]
fn compares_single_and_multi_rdn_names_in_sequence_order() {
    let single_encoding = name(vec![rdn(vec![attribute(COMMON_NAME, 0x0c, b"service")])]);
    let single = decode(&single_encoding);
    assert!(compare_x509_names_by(&single, &single, exact_value));

    let left_encoding = name(vec![
        rdn(vec![attribute(COUNTRY_NAME, 0x13, b"US")]),
        rdn(vec![attribute(COMMON_NAME, 0x0c, b"service")]),
    ]);
    let right_encoding = name(vec![
        rdn(vec![attribute(COMMON_NAME, 0x0c, b"service")]),
        rdn(vec![attribute(COUNTRY_NAME, 0x13, b"US")]),
    ]);
    let left = decode(&left_encoding);
    let right = decode(&right_encoding);
    assert!(!compare_x509_names_by(&left, &right, exact_value));
}

#[test]
fn rejects_rdn_count_and_size_mismatches_before_value_comparison() {
    let one_encoding = name(vec![rdn(vec![attribute(COMMON_NAME, 0x0c, b"A")])]);
    let two_rdns_encoding = name(vec![
        rdn(vec![attribute(COMMON_NAME, 0x0c, b"A")]),
        rdn(vec![attribute(COUNTRY_NAME, 0x13, b"US")]),
    ]);
    let two_attributes_encoding = name(vec![rdn(vec![
        attribute(COMMON_NAME, 0x0c, b"A"),
        attribute(COUNTRY_NAME, 0x13, b"US"),
    ])]);
    let one = decode(&one_encoding);
    let two_rdns = decode(&two_rdns_encoding);
    let two_attributes = decode(&two_attributes_encoding);
    let calls = Cell::new(0usize);

    assert!(!compare_x509_names_by(&one, &two_rdns, |_, _| {
        calls.set(calls.get() + 1);
        true
    }));
    assert!(!compare_x509_names_by(&one, &two_attributes, |_, _| {
        calls.set(calls.get() + 1);
        true
    }));
    assert_eq!(calls.get(), 0);
}

#[test]
fn rejects_oid_mismatch_without_invoking_value_policy() {
    let left_encoding = name(vec![rdn(vec![attribute(COMMON_NAME, 0x0c, b"same")])]);
    let right_encoding = name(vec![rdn(vec![attribute(COUNTRY_NAME, 0x0c, b"same")])]);
    let left = decode(&left_encoding);
    let right = decode(&right_encoding);
    let calls = Cell::new(0usize);

    assert!(!compare_x509_names_by(&left, &right, |_, _| {
        calls.set(calls.get() + 1);
        true
    }));
    assert_eq!(calls.get(), 0);
}

#[test]
fn matches_multivalued_rdns_independently_of_der_set_order() {
    let left_encoding = name(vec![rdn(vec![
        attribute(COMMON_NAME, 0x0c, b"A"),
        attribute(COMMON_NAME, 0x13, b"B"),
    ])]);
    let right_encoding = name(vec![rdn(vec![
        attribute(COMMON_NAME, 0x13, b"A"),
        attribute(COMMON_NAME, 0x0c, b"B"),
    ])]);
    let left = decode(&left_encoding);
    let right = decode(&right_encoding);
    let calls = Cell::new(0usize);

    assert!(compare_x509_names_by(&left, &right, |left, right| {
        calls.set(calls.get() + 1);
        text_bytes(left, right)
    }));
    assert_eq!(
        calls.get(),
        3,
        "the first left value probes both candidates"
    );
}

#[test]
fn repeated_types_require_distinct_matching_values_and_fail_closed() {
    let left_encoding = name(vec![rdn(vec![
        attribute(COMMON_NAME, 0x0c, b"A"),
        attribute(COMMON_NAME, 0x0c, b"B"),
    ])]);
    let right_encoding = name(vec![rdn(vec![
        attribute(COMMON_NAME, 0x0c, b"A"),
        attribute(COMMON_NAME, 0x0c, b"A"),
    ])]);
    let left = decode(&left_encoding);
    let right = decode(&right_encoding);

    assert!(!compare_x509_names_by(&left, &right, text_bytes));
}

#[test]
fn empty_names_compare_equal_without_callback_invocation() {
    let encoding = name(vec![]);
    let left = decode(&encoding);
    let right = decode(&encoding);
    let calls = Cell::new(0usize);

    assert!(compare_x509_names_by(&left, &right, |_, _| {
        calls.set(calls.get() + 1);
        false
    }));
    assert_eq!(calls.get(), 0);
}

#[test]
fn compares_the_full_fixed_attribute_bound_without_allocation() {
    let attributes = (0u8..64)
        .map(|value| attribute(COMMON_NAME, 0x0c, &[value]))
        .collect();
    let encoding = name(vec![rdn(attributes)]);
    let left = decode(&encoding);
    let right = decode(&encoding);
    let calls = Cell::new(0usize);

    assert_eq!(left.attribute_count(), 64);
    assert!(compare_x509_names_by(&left, &right, |left, right| {
        calls.set(calls.get() + 1);
        exact_value(left, right)
    }));
    assert_eq!(calls.get(), 64);
}
