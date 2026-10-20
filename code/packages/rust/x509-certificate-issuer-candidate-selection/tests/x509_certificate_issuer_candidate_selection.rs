use der_asn1::Asn1Limits;
use std::cell::RefCell;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_issuer_candidate_selection::{
    select_unique_x509_issuer_candidate_by, X509IssuerCandidateSelectionErrorKind,
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

#[test]
fn selects_one_candidate_after_scanning_the_complete_pool_once() {
    let wanted = name(b"issuing-ca");
    let child = decoded_certificate(1, &wanted, &name(b"service"));
    let candidates = vec![
        decoded_certificate(2, &name(b"root"), &name(b"other-ca")),
        decoded_certificate(3, &name(b"root"), &wanted),
        decoded_certificate(4, &name(b"root"), &name(b"last-ca")),
    ];
    let calls = RefCell::new(Vec::new());

    let selected =
        select_unique_x509_issuer_candidate_by(&child, &candidates, 3, |issuer, subject| {
            calls
                .borrow_mut()
                .push((issuer.encoded().to_vec(), subject.encoded().to_vec()));
            issuer.encoded() == subject.encoded()
        })
        .unwrap();

    assert_eq!(calls.borrow().len(), 3);
    assert!(calls
        .borrow()
        .iter()
        .all(|(issuer, _)| issuer.as_slice() == wanted));
    let observed_subjects: Vec<Vec<u8>> = calls
        .borrow()
        .iter()
        .map(|(_, subject)| subject.clone())
        .collect();
    assert_eq!(
        observed_subjects,
        vec![name(b"other-ca"), wanted.clone(), name(b"last-ca")]
    );
    assert_eq!(selected.candidate_index(), 1);
    assert!(std::ptr::eq(selected.child_certificate(), &child));
    assert!(std::ptr::eq(
        selected.candidate_certificate(),
        &candidates[1]
    ));
    assert_eq!(format!("{selected:?}"), "SelectedX509IssuerCandidate");
}

#[test]
fn rejects_no_match_with_a_redacted_error() {
    let child = decoded_certificate(1, &name(b"secret-issuer"), &name(b"service"));
    let candidates = vec![decoded_certificate(
        2,
        &name(b"root"),
        &name(b"secret-other"),
    )];

    let error =
        select_unique_x509_issuer_candidate_by(&child, &candidates, 1, |issuer, subject| {
            issuer.encoded() == subject.encoded()
        })
        .unwrap_err();

    assert_eq!(error.kind(), X509IssuerCandidateSelectionErrorKind::NoMatch);
    assert_eq!(
        error.to_string(),
        "X.509 issuer-candidate selection error NoMatch"
    );
    assert!(!error.to_string().contains("secret-issuer"));
    assert!(!error.to_string().contains("secret-other"));
}

#[test]
fn rejects_ambiguity_only_after_comparing_every_candidate_once() {
    let wanted = name(b"issuing-ca");
    let child = decoded_certificate(1, &wanted, &name(b"service"));
    let candidates = vec![
        decoded_certificate(2, &name(b"root"), &wanted),
        decoded_certificate(3, &name(b"root"), &wanted),
        decoded_certificate(4, &name(b"root"), &name(b"last-ca")),
    ];
    let calls = RefCell::new(Vec::new());

    let error =
        select_unique_x509_issuer_candidate_by(&child, &candidates, 3, |issuer, subject| {
            calls.borrow_mut().push(subject.encoded().to_vec());
            issuer.encoded() == subject.encoded()
        })
        .unwrap_err();

    assert_eq!(calls.borrow().len(), 3);
    assert_eq!(
        error.kind(),
        X509IssuerCandidateSelectionErrorKind::AmbiguousMatch
    );
}

#[test]
fn enforces_a_nonzero_limit_before_invoking_name_policy() {
    let child = decoded_certificate(1, &name(b"issuer"), &name(b"service"));
    let candidates = vec![decoded_certificate(2, &name(b"root"), &name(b"issuer"))];

    let zero_limit = select_unique_x509_issuer_candidate_by(&child, &candidates, 0, |_, _| {
        panic!("invalid limit must reject before comparison")
    })
    .unwrap_err();
    assert_eq!(
        zero_limit.kind(),
        X509IssuerCandidateSelectionErrorKind::InvalidCandidateLimit
    );
}

#[test]
fn rejects_pool_larger_than_positive_limit_before_name_policy() {
    let child = decoded_certificate(1, &name(b"issuer"), &name(b"service"));
    let candidates = vec![
        decoded_certificate(2, &name(b"root"), &name(b"issuer")),
        decoded_certificate(3, &name(b"root"), &name(b"issuer")),
    ];

    let error = select_unique_x509_issuer_candidate_by(&child, &candidates, 1, |_, _| {
        panic!("oversized pool must reject before comparison")
    })
    .unwrap_err();
    assert_eq!(
        error.kind(),
        X509IssuerCandidateSelectionErrorKind::CandidateLimitExceeded
    );
}
