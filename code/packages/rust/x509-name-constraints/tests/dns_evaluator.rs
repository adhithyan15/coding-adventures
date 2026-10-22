use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_name_constraints::{
    decode_name_constraints_contents, evaluate_permitted_dns_names, DnsConstraintErrorKind,
    DnsNameSyntaxError, MAX_PRESENTED_DNS_NAMES,
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

fn permitted(subtrees: &[&[u8]]) -> Vec<u8> {
    let mut contents = Vec::new();
    for subtree in subtrees {
        contents.extend_from_slice(&element(0x30, &element(0x82, subtree)));
    }
    element(0xa0, &contents)
}

fn evaluate(
    permitted_subtrees: &[&[u8]],
    presented_dns_names: &[&str],
) -> Result<x509_name_constraints::PermittedDnsNames, x509_name_constraints::DnsConstraintError> {
    let encoded = permitted(permitted_subtrees);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let constraints = decode_name_constraints_contents(&mut decoder, &encoded).unwrap();
    evaluate_permitted_dns_names(&constraints, presented_dns_names)
}

#[test]
fn exact_and_subdomain_matches_are_case_insensitive_and_label_based() {
    let evaluated = evaluate(
        &[b"Example.COM"],
        &["example.com", "WWW.example.COM", "deep.www.example.com"],
    )
    .unwrap();
    assert_eq!(evaluated.count(), 3);
    assert!(!evaluated.is_empty());
    assert_eq!(format!("{evaluated:?}"), "PermittedDnsNames { count: 3 }");

    let error = evaluate(&[b"example.com"], &["badexample.com"]).unwrap_err();
    assert_eq!(
        error.kind(),
        DnsConstraintErrorKind::PresentedDnsNameNotPermitted
    );
    assert_eq!(error.item_index(), 0);
}

#[test]
fn reviewed_leading_dot_requires_a_left_hand_label() {
    let encoded = [0xa0, 0x07, 0x30, 0x05, 0x82, 0x03, b'.', b't', b'r'];
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let constraints = decode_name_constraints_contents(&mut decoder, &encoded).unwrap();
    let evaluated = evaluate_permitted_dns_names(
        &constraints,
        &["example.tr", "deep.example.tr", "*.example.tr"],
    )
    .unwrap();
    assert_eq!(evaluated.count(), 3);

    for name in ["tr", "eviltr", ".tr"] {
        assert!(evaluate_permitted_dns_names(&constraints, &[name]).is_err());
    }
}

#[test]
fn every_presented_name_must_match_one_of_the_data_driven_subtrees() {
    let evaluated = evaluate(
        &[b"example.com", b".tr"],
        &["api.example.com", "service.tr"],
    )
    .unwrap();
    assert_eq!(evaluated.count(), 2);

    let error = evaluate(
        &[b"example.com", b".tr"],
        &["api.example.com", "outside.test"],
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        DnsConstraintErrorKind::PresentedDnsNameNotPermitted
    );
    assert_eq!(error.item_index(), 1);
}

#[test]
fn empty_presented_collection_is_permitted_for_an_absent_name_form() {
    let evaluated = evaluate(&[b".tr"], &[]).unwrap();
    assert_eq!(evaluated.count(), 0);
    assert!(evaluated.is_empty());
}

#[test]
fn presented_wildcard_must_be_the_complete_leftmost_label() {
    assert!(evaluate(&[b"example.tr"], &["*.example.tr"]).is_ok());

    for name in ["*", "foo.*.example.tr", "f*.example.tr", "**.example.tr"] {
        let error = evaluate(&[b"example.tr"], &[name]).unwrap_err();
        assert_eq!(
            error.kind(),
            DnsConstraintErrorKind::InvalidPresentedDnsName(DnsNameSyntaxError::InvalidWildcard)
        );
        assert_eq!(error.item_index(), 0);
    }
}

#[test]
fn every_constraint_is_validated_before_any_name_is_accepted() {
    let error = evaluate(&[b"example.com", b"*.tr"], &["api.example.com"]).unwrap_err();
    assert_eq!(
        error.kind(),
        DnsConstraintErrorKind::InvalidPermittedSubtree(DnsNameSyntaxError::WildcardNotAllowed)
    );
    assert_eq!(error.item_index(), 1);
}

#[test]
fn constraint_syntax_failures_are_stable_and_indexed() {
    let long_label = vec![b'a'; 64];
    let mut long_name = Vec::new();
    for index in 0..128 {
        if index != 0 {
            long_name.push(b'.');
        }
        long_name.push(b'a');
    }
    assert!(long_name.len() > 253);

    let cases: Vec<(&[u8], DnsNameSyntaxError)> = vec![
        (b".", DnsNameSyntaxError::Empty),
        (&long_name, DnsNameSyntaxError::TooLong),
        (b"example..com", DnsNameSyntaxError::EmptyLabel),
        (&long_label, DnsNameSyntaxError::LabelTooLong),
        (b"bad_name", DnsNameSyntaxError::InvalidCharacter),
        (b"-bad.example", DnsNameSyntaxError::LeadingHyphen),
        (b"bad-.example", DnsNameSyntaxError::TrailingHyphen),
        (b"*.example", DnsNameSyntaxError::WildcardNotAllowed),
    ];

    for (constraint, reason) in cases {
        let error = evaluate(&[b"valid.example", constraint], &[]).unwrap_err();
        assert_eq!(
            error.kind(),
            DnsConstraintErrorKind::InvalidPermittedSubtree(reason)
        );
        assert_eq!(error.item_index(), 1);
    }
}

#[test]
fn presented_name_syntax_failures_are_stable_and_indexed() {
    let long_label = "a".repeat(64);
    let long_name = (0..128).map(|_| "a").collect::<Vec<_>>().join(".");
    assert!(long_name.len() > 253);

    let cases = [
        ("", DnsNameSyntaxError::Empty),
        (&long_name, DnsNameSyntaxError::TooLong),
        ("é.example", DnsNameSyntaxError::NonAscii),
        ("example..com", DnsNameSyntaxError::EmptyLabel),
        (&long_label, DnsNameSyntaxError::LabelTooLong),
        ("bad_name", DnsNameSyntaxError::InvalidCharacter),
        ("-bad.example", DnsNameSyntaxError::LeadingHyphen),
        ("bad-.example", DnsNameSyntaxError::TrailingHyphen),
        ("example.com.", DnsNameSyntaxError::EmptyLabel),
    ];

    for (name, reason) in cases {
        let error = evaluate(&[b"example.com"], &["ok.example.com", name]).unwrap_err();
        assert_eq!(
            error.kind(),
            DnsConstraintErrorKind::InvalidPresentedDnsName(reason)
        );
        assert_eq!(error.item_index(), 1);
    }
}

#[test]
fn presented_name_capacity_is_exact_and_checked_before_matching() {
    let names = vec!["a.example.com"; MAX_PRESENTED_DNS_NAMES];
    assert_eq!(evaluate(&[b"example.com"], &names).unwrap().count(), 256);

    let names = vec!["a.example.com"; MAX_PRESENTED_DNS_NAMES + 1];
    let error = evaluate(&[b"example.com"], &names).unwrap_err();
    assert_eq!(
        error.kind(),
        DnsConstraintErrorKind::TooManyPresentedDnsNames
    );
    assert_eq!(error.item_index(), MAX_PRESENTED_DNS_NAMES);
}

#[test]
fn diagnostics_are_redacted() {
    let error = evaluate(&[b"allowed.example"], &["secret.internal"]).unwrap_err();
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("secret.internal"));
    assert!(!diagnostic.contains("allowed.example"));
}
