use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_name_constraints::{
    decode_name_constraints, decode_name_constraints_contents, evaluate_dns_name_subtrees,
    evaluate_dns_names, ConstrainedDnsNames, DnsConstraintErrorKind, DnsNameSyntaxError,
    MAX_DNS_SUBTREES, MAX_PRESENTED_DNS_NAMES,
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

fn excluded(subtrees: &[&[u8]]) -> Vec<u8> {
    let mut contents = Vec::new();
    for subtree in subtrees {
        contents.extend_from_slice(&element(0x30, &element(0x82, subtree)));
    }
    element(0xa1, &contents)
}

fn evaluate(
    permitted_subtrees: &[&[u8]],
    presented_dns_names: &[&str],
) -> Result<ConstrainedDnsNames, x509_name_constraints::DnsConstraintError> {
    let encoded = permitted(permitted_subtrees);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let constraints = decode_name_constraints_contents(&mut decoder, &encoded).unwrap();
    evaluate_dns_names(&constraints, presented_dns_names)
}

fn evaluate_full(
    permitted_subtrees: &[&[u8]],
    excluded_subtrees: &[&[u8]],
    presented_dns_names: &[&str],
) -> Result<ConstrainedDnsNames, x509_name_constraints::DnsConstraintError> {
    let mut fields = Vec::new();
    if !permitted_subtrees.is_empty() {
        fields.extend(permitted(permitted_subtrees));
    }
    if !excluded_subtrees.is_empty() {
        fields.extend(excluded(excluded_subtrees));
    }
    let encoded = element(0x30, &fields);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let constraints = decode_name_constraints(&mut decoder, &encoded).unwrap();
    evaluate_dns_names(&constraints, presented_dns_names)
}

#[test]
fn dns_only_evaluation_fails_closed_when_ip_subtrees_are_retained() {
    let ipv4 = [192, 0, 2, 0, 255, 255, 255, 0];
    for field_tag in [0xa0, 0xa1] {
        // One field may contain both GeneralName forms.
        let mixed_contents = [
            element(0x30, &element(0x82, b"example.test")),
            element(0x30, &element(0x87, &ipv4)),
        ]
        .concat();
        let encoded = element(0x30, &element(field_tag, &mixed_contents));
        let mut decoder = Asn1Decoder::new(Asn1Limits::default());
        let constraints = decode_name_constraints(&mut decoder, &encoded).unwrap();

        let error = evaluate_dns_names(&constraints, &["example.test"]).unwrap_err();
        assert_eq!(error.kind(), DnsConstraintErrorKind::UnevaluatedIpSubtrees);
        assert_eq!(error.item_index(), 0);
    }
}

#[test]
fn explicit_dns_component_can_be_composed_with_separate_ip_policy() {
    let ipv4 = [192, 0, 2, 0, 255, 255, 255, 0];
    let mixed_contents = [
        element(0x30, &element(0x82, b"example.test")),
        element(0x30, &element(0x87, &ipv4)),
    ]
    .concat();
    let encoded = element(0x30, &element(0xa0, &mixed_contents));
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let constraints = decode_name_constraints(&mut decoder, &encoded).unwrap();

    assert_eq!(
        evaluate_dns_names(&constraints, &["api.example.test"])
            .unwrap_err()
            .kind(),
        DnsConstraintErrorKind::UnevaluatedIpSubtrees
    );
    let evaluated = evaluate_dns_name_subtrees(
        constraints.permitted_dns_subtrees(),
        constraints.excluded_dns_subtrees(),
        &["api.example.test"],
    )
    .unwrap();
    assert_eq!(evaluated.count(), 1);
    assert_eq!(constraints.permitted_ip_subtrees(), &[&ipv4[..]]);
}

#[test]
fn explicit_dns_component_enforces_all_collection_ceilings() {
    let oversized = vec!["example.test"; MAX_DNS_SUBTREES + 1];

    assert_eq!(
        evaluate_dns_name_subtrees(&oversized, &[], &[])
            .unwrap_err()
            .kind(),
        DnsConstraintErrorKind::TooManyPermittedSubtrees
    );
    assert_eq!(
        evaluate_dns_name_subtrees(&[], &oversized, &[])
            .unwrap_err()
            .kind(),
        DnsConstraintErrorKind::TooManyExcludedSubtrees
    );
    assert_eq!(
        evaluate_dns_name_subtrees(&[], &[], &oversized)
            .unwrap_err()
            .kind(),
        DnsConstraintErrorKind::TooManyPresentedDnsNames
    );
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
    assert_eq!(format!("{evaluated:?}"), "ConstrainedDnsNames { count: 3 }");

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
    let evaluated = evaluate_dns_names(
        &constraints,
        &["example.tr", "deep.example.tr", "*.example.tr"],
    )
    .unwrap();
    assert_eq!(evaluated.count(), 3);

    for name in ["tr", "eviltr", ".tr"] {
        assert!(evaluate_dns_names(&constraints, &[name]).is_err());
    }
}

#[test]
fn excluded_subtrees_reject_exact_and_descendant_names() {
    let evaluated =
        evaluate_full(&[], &[b"private.example.test"], &["public.example.test"]).unwrap();
    assert_eq!(evaluated.count(), 1);

    for name in ["private.example.test", "api.private.example.test"] {
        let error = evaluate_full(&[], &[b"private.example.test"], &[name]).unwrap_err();
        assert_eq!(
            error.kind(),
            DnsConstraintErrorKind::PresentedDnsNameExcluded
        );
        assert_eq!(error.item_index(), 0);
    }
}

#[test]
fn excluded_leading_dot_requires_a_left_hand_label() {
    assert!(evaluate_full(&[], &[b".tr"], &["tr"]).is_ok());
    for name in ["example.tr", "deep.example.tr", "*.example.tr"] {
        assert_eq!(
            evaluate_full(&[], &[b".tr"], &[name]).unwrap_err().kind(),
            DnsConstraintErrorKind::PresentedDnsNameExcluded
        );
    }
}

#[test]
fn exclusion_wins_when_a_name_is_also_permitted() {
    let error = evaluate_full(
        &[b"example.test"],
        &[b"private.example.test"],
        &["api.private.example.test"],
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        DnsConstraintErrorKind::PresentedDnsNameExcluded
    );
}

#[test]
fn excluded_subtree_rejects_a_broader_overlapping_presented_wildcard() {
    let error = evaluate_full(
        &[b"example.test"],
        &[b"private.example.test"],
        &["*.example.test"],
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        DnsConstraintErrorKind::PresentedDnsNameExcluded
    );

    assert!(evaluate_full(
        &[b"example.test"],
        &[b"deeper.private.example.test", b".private.example.test"],
        &["*.example.test"],
    )
    .is_ok());
    assert_eq!(
        evaluate_full(
            &[b"example.test"],
            &[b"PRIVATE.EXAMPLE.TEST"],
            &["*.example.test"],
        )
        .unwrap_err()
        .kind(),
        DnsConstraintErrorKind::PresentedDnsNameExcluded
    );
}

#[test]
fn every_excluded_constraint_is_validated_before_any_name_is_accepted() {
    let error = evaluate_full(
        &[b"example.test"],
        &[b"private.example.test", b"*.invalid.test"],
        &["api.example.test"],
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        DnsConstraintErrorKind::InvalidExcludedSubtree(DnsNameSyntaxError::WildcardNotAllowed)
    );
    assert_eq!(error.item_index(), 1);
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
