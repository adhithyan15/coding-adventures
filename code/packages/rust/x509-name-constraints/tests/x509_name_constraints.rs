use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_name_constraints::{
    decode_name_constraints, decode_name_constraints_contents, NameConstraintsErrorKind,
    MAX_DNS_SUBTREES, MAX_GENERAL_SUBTREES, NAME_CONSTRAINTS_OID,
};

fn der_length(length: usize) -> Vec<u8> {
    if length < 128 {
        return vec![length as u8];
    }
    let bytes = length.to_be_bytes();
    let first = bytes.iter().position(|byte| *byte != 0).unwrap();
    let significant = &bytes[first..];
    let mut encoded = vec![0x80 | significant.len() as u8];
    encoded.extend_from_slice(significant);
    encoded
}

fn element(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = vec![tag];
    encoded.extend_from_slice(&der_length(value.len()));
    encoded.extend_from_slice(value);
    encoded
}

fn general_subtree(base: &[u8], trailing_fields: &[Vec<u8>]) -> Vec<u8> {
    let mut fields = element(0x82, base);
    for field in trailing_fields {
        fields.extend_from_slice(field);
    }
    element(0x30, &fields)
}

fn ip_subtree(base: &[u8], trailing_fields: &[Vec<u8>]) -> Vec<u8> {
    let mut fields = element(0x87, base);
    for field in trailing_fields {
        fields.extend_from_slice(field);
    }
    element(0x30, &fields)
}

fn permitted(subtrees: &[Vec<u8>]) -> Vec<u8> {
    let contents: Vec<u8> = subtrees.iter().flatten().copied().collect();
    element(0xa0, &contents)
}

fn excluded(subtrees: &[Vec<u8>]) -> Vec<u8> {
    let contents: Vec<u8> = subtrees.iter().flatten().copied().collect();
    element(0xa1, &contents)
}

fn decode_with_limits(
    encoded: &[u8],
    limits: Asn1Limits,
) -> Result<x509_name_constraints::NameConstraints<'_>, x509_name_constraints::NameConstraintsError>
{
    let mut decoder = Asn1Decoder::new(limits);
    decode_name_constraints_contents(&mut decoder, encoded)
}

fn decode(
    encoded: &[u8],
) -> Result<x509_name_constraints::NameConstraints<'_>, x509_name_constraints::NameConstraintsError>
{
    decode_with_limits(encoded, Asn1Limits::default())
}

fn decode_der_with_limits(
    encoded: &[u8],
    limits: Asn1Limits,
) -> Result<x509_name_constraints::NameConstraints<'_>, x509_name_constraints::NameConstraintsError>
{
    let mut decoder = Asn1Decoder::new(limits);
    decode_name_constraints(&mut decoder, encoded)
}

#[test]
fn decodes_one_exact_name_constraints_sequence() {
    let encoded = element(0x30, &permitted(&[general_subtree(b".tr", &[])]));
    let constraints = decode_der_with_limits(&encoded, Asn1Limits::default()).unwrap();

    assert_eq!(NAME_CONSTRAINTS_OID, &[2, 5, 29, 30]);
    assert_eq!(constraints.permitted_dns_subtrees(), &[".tr"]);
}

#[test]
fn full_der_decoder_retains_permitted_and_excluded_dns_subtrees() {
    let fields = [
        permitted(&[general_subtree(b"example.test", &[])]),
        excluded(&[general_subtree(b"private.example.test", &[])]),
    ]
    .concat();
    let encoded = element(0x30, &fields);
    let constraints = decode_der_with_limits(&encoded, Asn1Limits::default()).unwrap();

    assert_eq!(constraints.permitted_dns_subtrees(), &["example.test"]);
    assert_eq!(
        constraints.excluded_dns_subtrees(),
        &["private.example.test"]
    );
    assert_eq!(constraints.permitted_len(), 1);
    assert_eq!(constraints.excluded_len(), 1);
}

#[test]
fn full_der_decoder_retains_ipv4_and_ipv6_subtrees_by_constraint_class() {
    let ipv4 = [192, 0, 2, 0, 255, 255, 255, 0];
    let mut ipv6 = [0u8; 32];
    ipv6[..4].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
    ipv6[16..24].fill(0xff);
    let fields = [
        permitted(&[
            general_subtree(b"example.test", &[]),
            ip_subtree(&ipv4, &[]),
        ]),
        excluded(&[
            ip_subtree(&ipv6, &[]),
            general_subtree(b"private.example.test", &[]),
        ]),
    ]
    .concat();
    let encoded = element(0x30, &fields);
    let constraints = decode_der_with_limits(&encoded, Asn1Limits::default()).unwrap();

    assert_eq!(constraints.permitted_dns_subtrees(), &["example.test"]);
    assert_eq!(constraints.permitted_ip_subtrees(), &[ipv4.as_slice()]);
    assert_eq!(
        constraints.excluded_dns_subtrees(),
        &["private.example.test"]
    );
    assert_eq!(constraints.excluded_ip_subtrees(), &[ipv6.as_slice()]);
    assert_eq!(constraints.permitted_len(), 1);
    assert_eq!(constraints.permitted_ip_len(), 1);
    assert_eq!(constraints.excluded_len(), 1);
    assert_eq!(constraints.excluded_ip_len(), 1);
    assert_eq!(constraints.len(), 4);
    assert_eq!(
        format!("{constraints:?}"),
        "NameConstraints { permitted_dns_subtrees: 1, permitted_ip_subtrees: 1, excluded_dns_subtrees: 1, excluded_ip_subtrees: 1 }"
    );
}

#[test]
fn full_der_decoder_rejects_every_non_constraint_ip_width() {
    for length in 0..=33 {
        if length == 8 || length == 32 {
            continue;
        }
        let encoded = element(0x30, &permitted(&[ip_subtree(&vec![0; length], &[])]));
        let error = decode_der_with_limits(&encoded, Asn1Limits::default()).unwrap_err();
        assert_eq!(
            error.kind(),
            NameConstraintsErrorKind::InvalidIpSubtreeLength
        );
    }
}

#[test]
fn reviewed_snapshot_contents_remain_dns_only() {
    let ipv4 = [192, 0, 2, 0, 255, 255, 255, 0];
    let encoded = permitted(&[ip_subtree(&ipv4, &[])]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        NameConstraintsErrorKind::UnsupportedGeneralName
    );
}

#[test]
fn mixed_subtrees_share_one_capacity_per_top_level_field() {
    let ipv4 = [192, 0, 2, 0, 255, 255, 255, 0];
    let mut entries = Vec::with_capacity(MAX_GENERAL_SUBTREES + 1);
    for index in 0..MAX_GENERAL_SUBTREES {
        entries.push(if index % 2 == 0 {
            general_subtree(b"example.test", &[])
        } else {
            ip_subtree(&ipv4, &[])
        });
    }
    let encoded = element(0x30, &permitted(&entries));
    let constraints = decode_der_with_limits(&encoded, Asn1Limits::default()).unwrap();
    assert_eq!(constraints.len(), MAX_GENERAL_SUBTREES);
    assert_eq!(constraints.permitted_len(), MAX_GENERAL_SUBTREES / 2);
    assert_eq!(constraints.permitted_ip_len(), MAX_GENERAL_SUBTREES / 2);

    entries.push(ip_subtree(&ipv4, &[]));
    let encoded = element(0x30, &permitted(&entries));
    assert_eq!(
        decode_der_with_limits(&encoded, Asn1Limits::default())
            .unwrap_err()
            .kind(),
        NameConstraintsErrorKind::TooManySubtrees
    );
}

#[test]
fn full_der_decoder_accepts_excluded_only_but_snapshot_contents_do_not() {
    let field = excluded(&[general_subtree(b"private.example.test", &[])]);
    let encoded = element(0x30, &field);
    let constraints = decode_der_with_limits(&encoded, Asn1Limits::default()).unwrap();

    assert!(constraints.permitted_dns_subtrees().is_empty());
    assert_eq!(
        constraints.excluded_dns_subtrees(),
        &["private.example.test"]
    );
    assert_eq!(
        decode(&field).unwrap_err().kind(),
        NameConstraintsErrorKind::UnsupportedTopLevelField
    );
}

#[test]
fn full_der_decoder_rejects_empty_duplicate_and_reversed_fields() {
    let permitted_field = permitted(&[general_subtree(b"example.test", &[])]);
    let excluded_field = excluded(&[general_subtree(b"private.example.test", &[])]);
    let cases = [
        (
            element(0x30, &excluded(&[])),
            NameConstraintsErrorKind::EmptyExcludedSubtrees,
        ),
        (
            element(
                0x30,
                &[permitted_field.clone(), permitted_field.clone()].concat(),
            ),
            NameConstraintsErrorKind::DuplicateTopLevelField,
        ),
        (
            element(
                0x30,
                &[excluded_field.clone(), excluded_field.clone()].concat(),
            ),
            NameConstraintsErrorKind::DuplicateTopLevelField,
        ),
        (
            element(0x30, &[excluded_field, permitted_field].concat()),
            NameConstraintsErrorKind::UnsupportedTopLevelFieldOrder,
        ),
    ];

    for (encoded, expected) in cases {
        assert_eq!(
            decode_der_with_limits(&encoded, Asn1Limits::default())
                .unwrap_err()
                .kind(),
            expected
        );
    }
}

#[test]
fn excluded_subtrees_reject_invalid_ip_bases_and_distance_fields() {
    let invalid_ip = element(0x30, &element(0x87, &[192, 0, 2, 1]));
    let error = decode_der_with_limits(
        &element(0x30, &excluded(&[invalid_ip])),
        Asn1Limits::default(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        NameConstraintsErrorKind::InvalidIpSubtreeLength
    );

    let distance = general_subtree(b"private.example.test", &[element(0x80, &[0])]);
    let error = decode_der_with_limits(
        &element(0x30, &excluded(&[distance])),
        Asn1Limits::default(),
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        NameConstraintsErrorKind::UnsupportedGeneralSubtreeField
    );
}

#[test]
fn permitted_and_excluded_capacities_are_independent() {
    let permitted_entries: Vec<_> = (0..MAX_DNS_SUBTREES)
        .map(|_| general_subtree(b"example.test", &[]))
        .collect();
    let excluded_entries: Vec<_> = (0..MAX_DNS_SUBTREES)
        .map(|_| general_subtree(b"private.example.test", &[]))
        .collect();
    let fields = [permitted(&permitted_entries), excluded(&excluded_entries)].concat();
    let encoded = element(0x30, &fields);
    let constraints = decode_der_with_limits(&encoded, Asn1Limits::default()).unwrap();
    assert_eq!(constraints.permitted_len(), MAX_DNS_SUBTREES);
    assert_eq!(constraints.excluded_len(), MAX_DNS_SUBTREES);

    let mut too_many = excluded_entries;
    too_many.push(general_subtree(b"blocked.example.test", &[]));
    let fields = [permitted(&permitted_entries), excluded(&too_many)].concat();
    assert_eq!(
        decode_der_with_limits(&element(0x30, &fields), Asn1Limits::default())
            .unwrap_err()
            .kind(),
        NameConstraintsErrorKind::TooManySubtrees
    );
}

#[test]
fn full_der_decoder_rejects_contents_trailing_bytes_and_tight_limits() {
    let contents = permitted(&[general_subtree(b".tr", &[])]);
    let trailing = [element(0x30, &contents), vec![0x05, 0]].concat();
    let exhausted = Asn1Limits {
        max_total_elements: 0,
        ..Default::default()
    };

    for error in [
        decode_der_with_limits(&contents, Asn1Limits::default()).unwrap_err(),
        decode_der_with_limits(&trailing, Asn1Limits::default()).unwrap_err(),
        decode_der_with_limits(&element(0x30, &contents), exhausted).unwrap_err(),
    ] {
        assert!(matches!(
            error.kind(),
            NameConstraintsErrorKind::Structure(_)
        ));
    }
}

#[test]
fn decodes_the_exact_reviewed_constraint_bytes() {
    let encoded = [0xa0, 0x07, 0x30, 0x05, 0x82, 0x03, b'.', b't', b'r'];
    let constraints = decode(&encoded).unwrap();

    assert_eq!(constraints.permitted_dns_subtrees(), &[".tr"]);
    assert_eq!(constraints.len(), 1);
    assert!(!constraints.is_empty());
    assert_eq!(
        format!("{constraints:?}"),
        "NameConstraints { permitted_dns_subtrees: 1, permitted_ip_subtrees: 0, excluded_dns_subtrees: 0, excluded_ip_subtrees: 0 }"
    );
}

#[test]
fn retains_multiple_data_driven_dns_subtrees_in_wire_order() {
    let encoded = permitted(&[
        general_subtree(b".example", &[]),
        general_subtree(b"service.test", &[]),
    ]);
    let constraints = decode(&encoded).unwrap();
    assert_eq!(
        constraints.permitted_dns_subtrees(),
        &[".example", "service.test"]
    );
}

#[test]
fn requires_one_exact_permitted_subtrees_field() {
    for encoded in [
        vec![],
        vec![0xa0, 0],
        [permitted(&[general_subtree(b"a", &[])]), vec![0x05, 0]].concat(),
    ] {
        let error = decode(&encoded).unwrap_err();
        if encoded == [0xa0, 0] {
            assert_eq!(
                error.kind(),
                NameConstraintsErrorKind::EmptyPermittedSubtrees
            );
            assert_eq!(error.offset(), 2);
        } else {
            assert!(matches!(
                error.kind(),
                NameConstraintsErrorKind::Structure(_)
            ));
        }
    }
}

#[test]
fn rejects_unsupported_top_level_tag_forms() {
    for encoded in [
        element(0x30, &[]),
        element(0x60, &[]),
        element(0xe0, &[]),
        element(0x80, &[]),
        element(0xa1, &[]),
    ] {
        let error = decode(&encoded).unwrap_err();
        assert_eq!(
            error.kind(),
            NameConstraintsErrorKind::UnsupportedTopLevelField
        );
        assert_eq!(error.offset(), 0);
    }
}

#[test]
fn enforces_the_fixed_subtree_capacity() {
    let entries: Vec<_> = (0..MAX_DNS_SUBTREES)
        .map(|_| general_subtree(b"a", &[]))
        .collect();
    let encoded = permitted(&entries);
    assert_eq!(decode(&encoded).unwrap().len(), MAX_DNS_SUBTREES);

    let mut too_many = entries;
    too_many.push(general_subtree(b"b", &[]));
    let encoded = permitted(&too_many);
    let expected_offset = encoded.len() - too_many.last().unwrap().len();
    let error = decode(&encoded).unwrap_err();
    assert_eq!(error.kind(), NameConstraintsErrorKind::TooManySubtrees);
    assert_eq!(error.offset(), expected_offset);
}

#[test]
fn general_subtrees_must_be_exact_sequences() {
    for subtree in [element(0x31, &[]), element(0xa0, &[]), element(0x10, &[])] {
        let error = decode(&permitted(&[subtree])).unwrap_err();
        assert_eq!(
            error.kind(),
            NameConstraintsErrorKind::InvalidGeneralSubtree
        );
        assert_eq!(error.offset(), 2);
    }

    let empty_sequence = element(0x30, &[]);
    let error = decode(&permitted(&[empty_sequence])).unwrap_err();
    assert_eq!(
        error.kind(),
        NameConstraintsErrorKind::UnsupportedGeneralName
    );
    assert_eq!(error.offset(), 4);
}

#[test]
fn only_primitive_context_specific_dns_bases_are_supported() {
    let mut bases = vec![element(0x16, b"a"), element(0xa2, b"a")];
    for tag in [0x80, 0x81, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88] {
        bases.push(element(tag, b"a"));
    }

    for base in bases {
        let subtree = element(0x30, &base);
        let error = decode(&permitted(&[subtree])).unwrap_err();
        assert_eq!(
            error.kind(),
            NameConstraintsErrorKind::UnsupportedGeneralName
        );
        assert_eq!(error.offset(), 4);
    }
}

#[test]
fn dns_subtrees_must_be_non_empty_ascii() {
    let empty = decode(&permitted(&[general_subtree(b"", &[])])).unwrap_err();
    assert_eq!(empty.kind(), NameConstraintsErrorKind::EmptyDnsSubtree);
    assert_eq!(empty.offset(), 6);

    let invalid = decode(&permitted(&[general_subtree(&[b'a', 0x80], &[])])).unwrap_err();
    assert_eq!(
        invalid.kind(),
        NameConstraintsErrorKind::InvalidDnsSubtree(Asn1ErrorKind::NonAsciiIa5String)
    );
    assert_eq!(invalid.offset(), 7);
}

#[test]
fn rejects_minimum_maximum_and_other_trailing_fields() {
    for trailing in [element(0x80, &[0]), element(0x81, &[1]), element(0x05, &[])] {
        let encoded = permitted(&[general_subtree(b"a", &[trailing])]);
        let error = decode(&encoded).unwrap_err();
        assert_eq!(
            error.kind(),
            NameConstraintsErrorKind::UnsupportedGeneralSubtreeField
        );
        assert_eq!(error.offset(), 7);
    }
}

#[test]
fn malformed_nested_values_report_contents_local_offsets() {
    let encoded = [0xa0, 0x06, 0x30, 0x05, 0x82, 0x04, b'a', b'b'];
    let error = decode(&encoded).unwrap_err();
    assert!(matches!(
        error.kind(),
        NameConstraintsErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), encoded.len());

    let malformed_trailing = element(0x30, &[element(0x82, b"a"), vec![0x05]].concat());
    let encoded = permitted(&[malformed_trailing]);
    let error = decode(&encoded).unwrap_err();
    assert!(matches!(
        error.kind(),
        NameConstraintsErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
}

#[test]
fn shared_depth_and_element_limits_are_enforced() {
    let encoded = permitted(&[general_subtree(b"a", &[])]);

    for (max_depth, expected_offset) in [(1, 0), (2, 2)] {
        let error = decode_with_limits(
            &encoded,
            Asn1Limits {
                max_depth,
                ..Asn1Limits::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            error.kind(),
            NameConstraintsErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
        );
        assert_eq!(error.offset(), expected_offset);
    }

    for (max_total_elements, expected_offset) in [(1, 2), (2, 4)] {
        let error = decode_with_limits(
            &encoded,
            Asn1Limits {
                max_total_elements,
                ..Asn1Limits::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            error.kind(),
            NameConstraintsErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
        );
        assert_eq!(error.offset(), expected_offset);
    }
}

#[test]
fn diagnostics_are_redacted() {
    let secret = b"secret.internal";
    let mut invalid = secret.to_vec();
    invalid.push(0x80);
    let error = decode(&permitted(&[general_subtree(&invalid, &[])])).unwrap_err();
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("secret.internal"));
}
