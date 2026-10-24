use der_asn1::Asn1Limits;
use std::cell::RefCell;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_issuer_candidate_selection::X509IssuerCandidateSelectionErrorKind;
use x509_certificate_path_construction::{
    construct_unique_x509_certificate_path_by, X509CertificatePathConstructionErrorKind,
    MAX_X509_CONSTRUCTED_PATH_CERTIFICATES,
};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

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

fn algorithm(oid: &[u8]) -> Vec<u8> {
    let mut fields = der(0x06, oid);
    fields.extend(der(0x05, &[]));
    der(0x30, &fields)
}

fn name(common_name: &[u8]) -> Vec<u8> {
    let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
    attribute.extend(der(0x0c, common_name));
    der(0x30, &der(0x31, &der(0x30, &attribute)))
}

fn certificate(serial: u8, issuer: &[u8], subject: &[u8]) -> Vec<u8> {
    let mut validity = der(0x17, b"260101000000Z");
    validity.extend(der(0x17, b"270101000000Z"));

    let mut subject_public_key_info = algorithm(KEY_OID);
    subject_public_key_info.extend(der(0x03, &[0x00, 0x30, 0x00]));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[serial]));
    tbs.extend(algorithm(SIGNATURE_OID));
    tbs.extend(issuer);
    tbs.extend(der(0x30, &validity));
    tbs.extend(subject);
    tbs.extend(der(0x30, &subject_public_key_info));

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(SIGNATURE_OID));
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
}

fn decoded_certificate(serial: u8, issuer: &[u8], subject: &[u8]) -> X509Certificate<'static> {
    let encoded = Box::leak(certificate(serial, issuer, subject).into_boxed_slice());
    decode_x509_certificate(encoded, Asn1Limits::default()).unwrap()
}

fn same_encoded_name(left: &x509_name::X509Name<'_>, right: &x509_name::X509Name<'_>) -> bool {
    left.encoded() == right.encoded()
}

#[test]
fn constructs_one_unique_target_to_terminal_chain_in_order() {
    let root_name = name(b"root");
    let intermediate_name = name(b"intermediate");
    let target = decoded_certificate(1, &intermediate_name, &name(b"service"));
    let candidates = vec![
        decoded_certificate(2, &root_name, &name(b"unrelated")),
        decoded_certificate(3, &root_name, &root_name),
        decoded_certificate(4, &root_name, &intermediate_name),
    ];
    let terminal_encoding = candidates[1].encoded().to_vec();
    let terminal_calls = RefCell::new(Vec::new());
    let name_calls = RefCell::new(Vec::new());

    let path = construct_unique_x509_certificate_path_by(
        &target,
        &candidates,
        3,
        4,
        |certificate| {
            terminal_calls
                .borrow_mut()
                .push(certificate.encoded().to_vec());
            certificate.encoded() == terminal_encoding
        },
        |issuer, subject| {
            name_calls
                .borrow_mut()
                .push((issuer.encoded().to_vec(), subject.encoded().to_vec()));
            same_encoded_name(issuer, subject)
        },
    )
    .unwrap();

    assert_eq!(terminal_calls.borrow().len(), 3);
    assert_eq!(name_calls.borrow().len(), 6);
    assert_eq!(path.certificate_count(), 3);
    assert_eq!(path.issuer_count(), 2);
    assert_eq!(path.candidate_indices().collect::<Vec<_>>(), vec![2, 1]);
    assert_eq!(path.candidate_index(0), Some(2));
    assert_eq!(path.candidate_index(1), Some(1));
    assert_eq!(path.candidate_index(2), None);
    assert!(std::ptr::eq(path.target_certificate(), &target));
    assert!(std::ptr::eq(
        path.candidate_certificates(),
        candidates.as_slice()
    ));
    assert!(std::ptr::eq(
        path.issuer_certificate(0).unwrap(),
        &candidates[2]
    ));
    assert!(std::ptr::eq(
        path.issuer_certificate(1).unwrap(),
        &candidates[1]
    ));
    assert!(!path.target_is_terminal());
    assert_eq!(format!("{path:?}"), "ConstructedX509CertificatePath");
}

#[test]
fn projects_exact_adjacent_links_from_the_retained_path_order() {
    let root_name = name(b"root");
    let intermediate_name = name(b"intermediate");
    let target = decoded_certificate(1, &intermediate_name, &name(b"service"));
    let candidates = vec![
        decoded_certificate(2, &root_name, &name(b"unrelated")),
        decoded_certificate(3, &root_name, &root_name),
        decoded_certificate(4, &root_name, &intermediate_name),
    ];
    let path = construct_unique_x509_certificate_path_by(
        &target,
        &candidates,
        3,
        4,
        |certificate| std::ptr::eq(certificate, &candidates[1]),
        same_encoded_name,
    )
    .unwrap();

    let target_link = path.adjacent_link(0).expect("target link");
    assert_eq!(target_link.issuer_depth(), 0);
    assert!(std::ptr::eq(target_link.constructed_path(), &path));
    assert!(std::ptr::eq(
        target_link.child_target_certificate().unwrap(),
        &target
    ));
    assert!(target_link.child_candidate_certificate().is_none());
    assert!(std::ptr::eq(
        target_link.issuer_certificate(),
        &candidates[2]
    ));
    assert_eq!(
        format!("{target_link:?}"),
        "ConstructedX509CertificatePathAdjacentLink"
    );

    let intermediate_link = path.adjacent_link(1).expect("intermediate link");
    assert_eq!(intermediate_link.issuer_depth(), 1);
    assert!(intermediate_link.child_target_certificate().is_none());
    assert!(std::ptr::eq(
        intermediate_link.child_candidate_certificate().unwrap(),
        &candidates[2]
    ));
    assert!(std::ptr::eq(
        intermediate_link.issuer_certificate(),
        &candidates[1]
    ));

    assert!(path.adjacent_link(2).is_none());
    assert!(path.adjacent_link(usize::MAX).is_none());
}

#[test]
fn accepts_a_terminal_target_without_invoking_name_policy() {
    let target = decoded_certificate(1, &name(b"root"), &name(b"root"));
    let candidates = Vec::new();

    let path = construct_unique_x509_certificate_path_by(
        &target,
        &candidates,
        1,
        1,
        |_| true,
        |_, _| panic!("a terminal target must not select an issuer"),
    )
    .unwrap();

    assert_eq!(path.certificate_count(), 1);
    assert_eq!(path.issuer_count(), 0);
    assert!(path.target_is_terminal());
    assert!(path.adjacent_link(0).is_none());
}

#[test]
fn rejects_all_invalid_bounds_before_either_policy_callback() {
    let target = decoded_certificate(1, &name(b"issuer"), &name(b"service"));
    let candidates = vec![decoded_certificate(2, &name(b"root"), &name(b"issuer"))];

    for (candidate_limit, path_limit, expected) in [
        (
            0,
            1,
            X509CertificatePathConstructionErrorKind::InvalidCandidateLimit,
        ),
        (
            1,
            0,
            X509CertificatePathConstructionErrorKind::InvalidPathCertificateLimit,
        ),
        (
            1,
            MAX_X509_CONSTRUCTED_PATH_CERTIFICATES + 1,
            X509CertificatePathConstructionErrorKind::UnsupportedPathCertificateLimit,
        ),
    ] {
        let error = construct_unique_x509_certificate_path_by(
            &target,
            &candidates,
            candidate_limit,
            path_limit,
            |_| panic!("invalid bounds must reject before terminal policy"),
            |_, _| panic!("invalid bounds must reject before name policy"),
        )
        .unwrap_err();
        assert_eq!(error.kind(), expected);
    }

    let oversized_candidates = vec![
        decoded_certificate(3, &name(b"root-a"), &name(b"issuer")),
        decoded_certificate(4, &name(b"root-b"), &name(b"issuer")),
    ];
    let oversized_pool = construct_unique_x509_certificate_path_by(
        &target,
        &oversized_candidates,
        1,
        2,
        |_| panic!("an oversized pool must reject before terminal policy"),
        |_, _| panic!("an oversized pool must reject before name policy"),
    )
    .unwrap_err();
    assert_eq!(
        oversized_pool.kind(),
        X509CertificatePathConstructionErrorKind::CandidateLimitExceeded
    );
}

#[test]
fn maps_no_match_and_ambiguity_to_payload_free_selection_errors() {
    let wanted = name(b"issuer");
    let target = decoded_certificate(1, &wanted, &name(b"secret-service"));
    let no_match_candidates = vec![decoded_certificate(
        2,
        &name(b"root"),
        &name(b"secret-other"),
    )];
    let no_match = construct_unique_x509_certificate_path_by(
        &target,
        &no_match_candidates,
        1,
        2,
        |_| false,
        same_encoded_name,
    )
    .unwrap_err();
    assert_eq!(
        no_match.kind(),
        X509CertificatePathConstructionErrorKind::IssuerSelection(
            X509IssuerCandidateSelectionErrorKind::NoMatch
        )
    );
    assert!(!no_match.to_string().contains("secret"));

    let ambiguous_candidates = vec![
        decoded_certificate(3, &name(b"root-a"), &wanted),
        decoded_certificate(4, &name(b"root-b"), &wanted),
    ];
    let ambiguous = construct_unique_x509_certificate_path_by(
        &target,
        &ambiguous_candidates,
        2,
        2,
        |_| false,
        same_encoded_name,
    )
    .unwrap_err();
    assert_eq!(
        ambiguous.kind(),
        X509CertificatePathConstructionErrorKind::IssuerSelection(
            X509IssuerCandidateSelectionErrorKind::AmbiguousMatch
        )
    );
}

#[test]
fn rejects_an_exact_certificate_cycle_before_rechecking_terminal_policy() {
    let name_a = name(b"a");
    let name_b = name(b"b");
    let target = decoded_certificate(1, &name_b, &name_a);
    let candidates = vec![decoded_certificate(2, &name_a, &name_b), target.clone()];
    let terminal_calls = RefCell::new(Vec::new());

    let error = construct_unique_x509_certificate_path_by(
        &target,
        &candidates,
        2,
        4,
        |certificate| {
            terminal_calls
                .borrow_mut()
                .push(certificate.encoded().to_vec());
            false
        },
        same_encoded_name,
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificatePathConstructionErrorKind::CertificateCycle
    );
    assert_eq!(terminal_calls.borrow().len(), 2);
}

#[test]
fn rejects_depth_exhaustion_before_selecting_another_issuer() {
    let root_name = name(b"root");
    let intermediate_name = name(b"intermediate");
    let target = decoded_certificate(1, &intermediate_name, &name(b"service"));
    let candidates = vec![
        decoded_certificate(2, &root_name, &root_name),
        decoded_certificate(3, &root_name, &intermediate_name),
    ];
    let terminal_calls = RefCell::new(0usize);
    let name_calls = RefCell::new(0usize);

    let error = construct_unique_x509_certificate_path_by(
        &target,
        &candidates,
        2,
        2,
        |_| {
            *terminal_calls.borrow_mut() += 1;
            false
        },
        |issuer, subject| {
            *name_calls.borrow_mut() += 1;
            same_encoded_name(issuer, subject)
        },
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509CertificatePathConstructionErrorKind::PathCertificateLimitExceeded
    );
    assert_eq!(*terminal_calls.borrow(), 2);
    assert_eq!(*name_calls.borrow(), 2);
}
