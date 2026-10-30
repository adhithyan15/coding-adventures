use der_asn1::{Asn1Decoder, Asn1Limits};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use x509_name_constraints::{
    decode_name_constraints, evaluate_dns_and_ip_names, evaluate_dns_names, evaluate_ip_addresses,
    DnsConstraintErrorKind, IpConstraintError, IpConstraintErrorKind, IpConstraintSyntaxError,
    NameConstraintEvaluationErrorKind, MAX_PRESENTED_IP_ADDRESSES,
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

fn subtree(tag: u8, value: &[u8]) -> Vec<u8> {
    element(0x30, &element(tag, value))
}

fn constraint_field(tag: u8, dns: &[&[u8]], ip: &[&[u8]]) -> Vec<u8> {
    let mut contents = Vec::new();
    for value in dns {
        contents.extend(subtree(0x82, value));
    }
    for value in ip {
        contents.extend(subtree(0x87, value));
    }
    element(tag, &contents)
}

fn encoded_constraints(
    permitted_dns: &[&[u8]],
    permitted_ip: &[&[u8]],
    excluded_dns: &[&[u8]],
    excluded_ip: &[&[u8]],
) -> Vec<u8> {
    let mut fields = Vec::new();
    if !permitted_dns.is_empty() || !permitted_ip.is_empty() {
        fields.extend(constraint_field(0xa0, permitted_dns, permitted_ip));
    }
    if !excluded_dns.is_empty() || !excluded_ip.is_empty() {
        fields.extend(constraint_field(0xa1, excluded_dns, excluded_ip));
    }
    element(0x30, &fields)
}

fn evaluate_ip(
    permitted_ip: &[&[u8]],
    excluded_ip: &[&[u8]],
    presented: &[IpAddr],
) -> Result<x509_name_constraints::ConstrainedIpAddresses, IpConstraintError> {
    let encoded = encoded_constraints(&[], permitted_ip, &[], excluded_ip);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let constraints = decode_name_constraints(&mut decoder, &encoded).unwrap();
    evaluate_ip_addresses(&constraints, presented)
}

#[test]
fn ipv4_permitted_and_excluded_prefixes_use_exclusion_precedence() {
    let permitted = [192, 0, 2, 0, 255, 255, 255, 0];
    let excluded = [192, 0, 2, 128, 255, 255, 255, 128];

    let accepted = evaluate_ip(
        &[&permitted],
        &[&excluded],
        &[IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1))],
    )
    .unwrap();
    assert_eq!(accepted.count(), 1);
    assert!(!accepted.is_empty());

    let excluded_error = evaluate_ip(
        &[&permitted],
        &[&excluded],
        &[IpAddr::V4(Ipv4Addr::new(192, 0, 2, 200))],
    )
    .unwrap_err();
    assert_eq!(
        excluded_error.kind(),
        IpConstraintErrorKind::PresentedIpAddressExcluded
    );

    let outside_error = evaluate_ip(
        &[&permitted],
        &[&excluded],
        &[IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1))],
    )
    .unwrap_err();
    assert_eq!(
        outside_error.kind(),
        IpConstraintErrorKind::PresentedIpAddressNotPermitted
    );
}

#[test]
fn ipv6_prefixes_match_exactly_within_their_family() {
    let mut permitted = [0u8; 32];
    permitted[..4].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
    permitted[16..20].fill(0xff);

    let mut excluded = [0u8; 32];
    excluded[..6].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0xde, 0xad]);
    excluded[16..22].fill(0xff);

    assert!(evaluate_ip(
        &[&permitted],
        &[&excluded],
        &[IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 1, 0, 0, 0, 0, 1,))],
    )
    .is_ok());
    assert_eq!(
        evaluate_ip(
            &[&permitted],
            &[&excluded],
            &[IpAddr::V6(Ipv6Addr::new(
                0x2001, 0x0db8, 0xdead, 0, 0, 0, 0, 1,
            ))],
        )
        .unwrap_err()
        .kind(),
        IpConstraintErrorKind::PresentedIpAddressExcluded
    );
}

#[test]
fn address_families_never_cross_match() {
    let ipv4 = [192, 0, 2, 0, 255, 255, 255, 0];
    let ipv6 = IpAddr::V6(Ipv6Addr::LOCALHOST);

    assert_eq!(
        evaluate_ip(&[&ipv4], &[], &[ipv6]).unwrap_err().kind(),
        IpConstraintErrorKind::PresentedIpAddressNotPermitted
    );
    assert!(evaluate_ip(&[], &[&ipv4], &[ipv6]).is_ok());
}

#[test]
fn masks_must_be_contiguous_and_host_bits_are_masked() {
    let valid = [192, 0, 2, 0, 255, 255, 255, 0];
    let non_contiguous_mask = [192, 0, 2, 0, 255, 0, 255, 0];
    let host_bits_set = [192, 0, 2, 1, 255, 255, 255, 0];

    let error = evaluate_ip(&[&valid, &non_contiguous_mask], &[], &[]).unwrap_err();
    assert_eq!(
        error.kind(),
        IpConstraintErrorKind::InvalidPermittedSubtree(IpConstraintSyntaxError::NonContiguousMask)
    );
    assert_eq!(error.item_index(), 1);

    let error = evaluate_ip(&[], &[&valid, &non_contiguous_mask], &[]).unwrap_err();
    assert_eq!(
        error.kind(),
        IpConstraintErrorKind::InvalidExcludedSubtree(IpConstraintSyntaxError::NonContiguousMask)
    );
    assert_eq!(error.item_index(), 1);

    assert!(evaluate_ip(
        &[&host_bits_set],
        &[],
        &[IpAddr::V4(Ipv4Addr::new(192, 0, 2, 42))],
    )
    .is_ok());
}

#[test]
fn mixed_constraints_require_the_combined_entry_point() {
    let ipv4 = [192, 0, 2, 0, 255, 255, 255, 0];
    let encoded = encoded_constraints(&[b"example.test"], &[&ipv4], &[], &[]);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let constraints = decode_name_constraints(&mut decoder, &encoded).unwrap();
    let addresses = [IpAddr::V4(Ipv4Addr::new(192, 0, 2, 9))];

    assert_eq!(
        evaluate_dns_names(&constraints, &["api.example.test"])
            .unwrap_err()
            .kind(),
        DnsConstraintErrorKind::UnevaluatedIpSubtrees
    );
    assert_eq!(
        evaluate_ip_addresses(&constraints, &addresses)
            .unwrap_err()
            .kind(),
        IpConstraintErrorKind::UnevaluatedDnsSubtrees
    );

    let evaluated =
        evaluate_dns_and_ip_names(&constraints, &["api.example.test"], &addresses).unwrap();
    assert_eq!(evaluated.dns_count(), 1);
    assert_eq!(evaluated.ip_count(), 1);
    assert_eq!(evaluated.total_count(), 2);

    let outside = [IpAddr::V4(Ipv4Addr::new(198, 51, 100, 9))];
    let error =
        evaluate_dns_and_ip_names(&constraints, &["api.example.test"], &outside).unwrap_err();
    assert_eq!(
        error.kind(),
        NameConstraintEvaluationErrorKind::Ip(
            IpConstraintErrorKind::PresentedIpAddressNotPermitted
        )
    );
    assert_eq!(error.item_index(), 0);
}

#[test]
fn combined_evaluation_validates_every_constraint_before_presented_names() {
    let invalid_ip = [192, 0, 2, 0, 255, 0, 255, 0];
    let encoded = encoded_constraints(&[b"example.test"], &[&invalid_ip], &[], &[]);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let constraints = decode_name_constraints(&mut decoder, &encoded).unwrap();

    let error = evaluate_dns_and_ip_names(
        &constraints,
        &["invalid_name"],
        &[IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1))],
    )
    .unwrap_err();
    assert_eq!(
        error.kind(),
        NameConstraintEvaluationErrorKind::Ip(IpConstraintErrorKind::InvalidPermittedSubtree(
            IpConstraintSyntaxError::NonContiguousMask,
        ),)
    );
    assert_eq!(error.item_index(), 0);
}

#[test]
fn presented_ip_capacity_is_exact_and_checked_before_matching() {
    let permitted = [0, 0, 0, 0, 0, 0, 0, 0];
    let addresses = vec![IpAddr::V4(Ipv4Addr::LOCALHOST); MAX_PRESENTED_IP_ADDRESSES];
    assert_eq!(
        evaluate_ip(&[&permitted], &[], &addresses).unwrap().count(),
        MAX_PRESENTED_IP_ADDRESSES
    );

    let addresses = vec![IpAddr::V4(Ipv4Addr::LOCALHOST); MAX_PRESENTED_IP_ADDRESSES + 1];
    let error = evaluate_ip(&[&permitted], &[], &addresses).unwrap_err();
    assert_eq!(
        error.kind(),
        IpConstraintErrorKind::TooManyPresentedIpAddresses
    );
    assert_eq!(error.item_index(), MAX_PRESENTED_IP_ADDRESSES);
}

#[test]
fn diagnostics_retain_no_constraint_or_presented_address() {
    let permitted = [192, 0, 2, 0, 255, 255, 255, 0];
    let error = evaluate_ip(
        &[&permitted],
        &[],
        &[IpAddr::V4(Ipv4Addr::new(203, 0, 113, 42))],
    )
    .unwrap_err();
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("192.0.2"));
    assert!(!diagnostic.contains("203.0.113.42"));
}
