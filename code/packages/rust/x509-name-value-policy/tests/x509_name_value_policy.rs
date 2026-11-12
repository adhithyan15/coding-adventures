use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_name::{decode_x509_name, X509Name};
use x509_name_value_policy::{
    X509NameValueComparisonErrorKind, X509NameValueComparisonRule, X509NameValuePolicy,
    X509NameValuePolicyDefinitionErrorKind, X509NameValuePolicyEntry,
    MAX_X509_NAME_VALUE_POLICY_ENTRIES,
};

const COMMON_NAME: &[u64] = &[2, 5, 4, 3];
const COUNTRY_NAME: &[u64] = &[2, 5, 4, 6];
const SERIAL_NUMBER: &[u64] = &[2, 5, 4, 5];

const COMMON_NAME_OID: &[u8] = &[0x55, 0x04, 0x03];
const COUNTRY_NAME_OID: &[u8] = &[0x55, 0x04, 0x06];
const SERIAL_NUMBER_OID: &[u8] = &[0x55, 0x04, 0x05];

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

fn entry(
    attribute_type: &'static [u64],
    rule: X509NameValueComparisonRule,
) -> X509NameValuePolicyEntry<'static> {
    X509NameValuePolicyEntry::new(attribute_type, rule)
}

#[test]
fn validates_empty_invalid_duplicate_and_over_limit_policy_data() {
    let empty = X509NameValuePolicy::new(&[]).unwrap();
    let empty_name_encoding = name(vec![]);
    let empty_name = decode(&empty_name_encoding);
    assert_eq!(empty.compare_names(&empty_name, &empty_name), Ok(true));

    for invalid in [&[][..], &[3, 1][..], &[1, 40][..]] {
        let entries = [X509NameValuePolicyEntry::new(
            invalid,
            X509NameValueComparisonRule::ExactDer,
        )];
        assert_eq!(
            X509NameValuePolicy::new(&entries).unwrap_err().kind(),
            X509NameValuePolicyDefinitionErrorKind::InvalidAttributeType
        );
    }

    let duplicate = [
        entry(COMMON_NAME, X509NameValueComparisonRule::ExactDer),
        entry(
            COMMON_NAME,
            X509NameValueComparisonRule::DirectoryStringScalar,
        ),
    ];
    assert_eq!(
        X509NameValuePolicy::new(&duplicate).unwrap_err().kind(),
        X509NameValuePolicyDefinitionErrorKind::DuplicateAttributeType
    );

    let identifiers: Vec<Vec<u64>> = (0..=MAX_X509_NAME_VALUE_POLICY_ENTRIES)
        .map(|suffix| vec![2, 999, suffix as u64])
        .collect();
    let too_many: Vec<_> = identifiers
        .iter()
        .map(|identifier| {
            X509NameValuePolicyEntry::new(identifier, X509NameValueComparisonRule::ExactDer)
        })
        .collect();
    assert_eq!(
        X509NameValuePolicy::new(&too_many).unwrap_err().kind(),
        X509NameValuePolicyDefinitionErrorKind::TooManyEntries
    );
}

#[test]
fn selects_exact_der_and_directory_string_rules_from_policy_data() {
    let entries = [
        entry(
            COMMON_NAME,
            X509NameValueComparisonRule::DirectoryStringScalar,
        ),
        entry(SERIAL_NUMBER, X509NameValueComparisonRule::ExactDer),
    ];
    let policy = X509NameValuePolicy::new(&entries).unwrap();
    assert_eq!(policy.entries(), &entries);

    let left_encoding = name(vec![rdn(vec![
        attribute(COMMON_NAME_OID, 0x0c, b"Alpha"),
        attribute(SERIAL_NUMBER_OID, 0x13, b"42"),
    ])]);
    let right_encoding = name(vec![rdn(vec![
        attribute(COMMON_NAME_OID, 0x13, b"Alpha"),
        attribute(SERIAL_NUMBER_OID, 0x13, b"42"),
    ])]);
    assert_eq!(
        policy.compare_names(&decode(&left_encoding), &decode(&right_encoding)),
        Ok(true)
    );

    let serial_reencoded = name(vec![rdn(vec![
        attribute(COMMON_NAME_OID, 0x13, b"Alpha"),
        attribute(SERIAL_NUMBER_OID, 0x0c, b"42"),
    ])]);
    assert_eq!(
        policy.compare_names(&decode(&left_encoding), &decode(&serial_reencoded)),
        Ok(false),
        "exact-DER policy includes the value tag"
    );
}

#[test]
fn compares_directory_strings_across_supported_encodings_but_not_case_or_space() {
    let entries = [entry(
        COMMON_NAME,
        X509NameValueComparisonRule::DirectoryStringScalar,
    )];
    let policy = X509NameValuePolicy::new(&entries).unwrap();
    let utf8_encoding = name(vec![rdn(vec![attribute(
        COMMON_NAME_OID,
        0x0c,
        "Aé".as_bytes(),
    )])]);
    let bmp_encoding = name(vec![rdn(vec![attribute(
        COMMON_NAME_OID,
        0x1e,
        &[0x00, 0x41, 0x00, 0xe9],
    )])]);
    assert_eq!(
        policy.compare_names(&decode(&utf8_encoding), &decode(&bmp_encoding)),
        Ok(true)
    );

    for value in [b"a\xc3\xa9".as_slice(), b"A \xc3\xa9".as_slice()] {
        let mismatch = name(vec![rdn(vec![attribute(COMMON_NAME_OID, 0x0c, value)])]);
        assert_eq!(
            policy.compare_names(&decode(&utf8_encoding), &decode(&mismatch)),
            Ok(false)
        );
    }
}

#[test]
fn rejects_unconfigured_attribute_types_in_either_name_before_matching() {
    let entries = [entry(
        COMMON_NAME,
        X509NameValueComparisonRule::DirectoryStringScalar,
    )];
    let policy = X509NameValuePolicy::new(&entries).unwrap();
    let configured_encoding = name(vec![rdn(vec![attribute(COMMON_NAME_OID, 0x0c, b"A")])]);
    let unconfigured_encoding = name(vec![rdn(vec![attribute(COUNTRY_NAME_OID, 0x13, b"US")])]);
    let configured = decode(&configured_encoding);
    let unconfigured = decode(&unconfigured_encoding);

    for result in [
        policy.compare_names(&unconfigured, &configured),
        policy.compare_names(&configured, &unconfigured),
    ] {
        assert_eq!(
            result.unwrap_err().kind(),
            X509NameValueComparisonErrorKind::UnconfiguredAttributeType
        );
    }
}

#[test]
fn rejects_malformed_and_unmapped_directory_strings_in_either_name() {
    let entries = [entry(
        COMMON_NAME,
        X509NameValueComparisonRule::DirectoryStringScalar,
    )];
    let policy = X509NameValuePolicy::new(&entries).unwrap();
    let valid_encoding = name(vec![rdn(vec![attribute(COMMON_NAME_OID, 0x0c, b"A")])]);
    let malformed_encoding = name(vec![rdn(vec![attribute(COMMON_NAME_OID, 0x16, b"A")])]);
    let teletex_encoding = name(vec![rdn(vec![attribute(COMMON_NAME_OID, 0x14, b"A")])]);
    let valid = decode(&valid_encoding);
    let malformed = decode(&malformed_encoding);
    let teletex = decode(&teletex_encoding);

    for result in [
        policy.compare_names(&malformed, &valid),
        policy.compare_names(&valid, &malformed),
    ] {
        assert_eq!(
            result.unwrap_err().kind(),
            X509NameValueComparisonErrorKind::InvalidDirectoryString
        );
    }
    for result in [
        policy.compare_names(&teletex, &valid),
        policy.compare_names(&valid, &teletex),
    ] {
        assert_eq!(
            result.unwrap_err().kind(),
            X509NameValueComparisonErrorKind::UnsupportedTeletexString
        );
    }
}

#[test]
fn preflight_errors_win_over_structural_mismatch_and_remain_payload_free() {
    let entries = [entry(
        COMMON_NAME,
        X509NameValueComparisonRule::DirectoryStringScalar,
    )];
    let policy = X509NameValuePolicy::new(&entries).unwrap();
    let left_encoding = name(vec![rdn(vec![attribute(
        COUNTRY_NAME_OID,
        0x13,
        b"SECRET",
    )])]);
    let right_encoding = name(vec![]);
    let error = policy
        .compare_names(&decode(&left_encoding), &decode(&right_encoding))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        X509NameValueComparisonErrorKind::UnconfiguredAttributeType
    );
    assert!(!error.to_string().contains("SECRET"));
}

#[test]
fn preserves_unordered_rdn_matching_for_repeated_attribute_types() {
    let entries = [entry(
        COMMON_NAME,
        X509NameValueComparisonRule::DirectoryStringScalar,
    )];
    let policy = X509NameValuePolicy::new(&entries).unwrap();
    let left_encoding = name(vec![rdn(vec![
        attribute(COMMON_NAME_OID, 0x0c, b"A"),
        attribute(COMMON_NAME_OID, 0x0c, b"B"),
    ])]);
    let right_encoding = name(vec![rdn(vec![
        attribute(COMMON_NAME_OID, 0x13, b"B"),
        attribute(COMMON_NAME_OID, 0x13, b"A"),
    ])]);
    assert_eq!(
        policy.compare_names(&decode(&left_encoding), &decode(&right_encoding)),
        Ok(true)
    );

    let duplicate_encoding = name(vec![rdn(vec![
        attribute(COMMON_NAME_OID, 0x13, b"A"),
        attribute(COMMON_NAME_OID, 0x13, b"A"),
    ])]);
    assert_eq!(
        policy.compare_names(&decode(&left_encoding), &decode(&duplicate_encoding)),
        Ok(false)
    );
}

#[test]
fn exact_der_rule_treats_teletex_and_non_directory_values_as_opaque() {
    let entries = [entry(SERIAL_NUMBER, X509NameValueComparisonRule::ExactDer)];
    let policy = X509NameValuePolicy::new(&entries).unwrap();
    let encoding = name(vec![rdn(vec![attribute(
        SERIAL_NUMBER_OID,
        0x04,
        &[0xde, 0xad, 0xbe, 0xef],
    )])]);
    let decoded = decode(&encoding);
    assert_eq!(policy.compare_names(&decoded, &decoded), Ok(true));

    let teletex_encoding = name(vec![rdn(vec![attribute(
        SERIAL_NUMBER_OID,
        0x14,
        &[0x80, 0x41],
    )])]);
    let teletex = decode(&teletex_encoding);
    assert_eq!(policy.compare_names(&teletex, &teletex), Ok(true));
}

#[test]
fn entry_accessors_expose_only_caller_policy_data() {
    let entry = entry(COUNTRY_NAME, X509NameValueComparisonRule::ExactDer);
    assert_eq!(entry.attribute_type(), COUNTRY_NAME);
    assert_eq!(entry.rule(), X509NameValueComparisonRule::ExactDer);
}
