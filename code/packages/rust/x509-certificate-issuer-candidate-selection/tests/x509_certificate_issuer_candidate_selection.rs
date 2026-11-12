use der_asn1::Asn1Limits;
use std::cell::RefCell;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_issuer_candidate_selection::{
    select_unique_x509_issuer_candidate_by, select_unique_x509_issuer_candidate_with_name_policy,
    X509IssuerCandidateSelectionErrorKind, X509NamePolicyIssuerCandidateSelectionErrorKind,
};
use x509_name_value_policy::{
    X509NameValueComparisonErrorKind, X509NameValueComparisonRule, X509NameValuePolicy,
    X509NameValuePolicyEntry,
};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const COMMON_NAME: &[u64] = &[2, 5, 4, 3];
const COMMON_NAME_OID: &[u8] = &[0x55, 0x04, 0x03];
const COUNTRY_NAME_OID: &[u8] = &[0x55, 0x04, 0x06];
const EXACT_POLICY_ENTRIES: [X509NameValuePolicyEntry<'static>; 1] =
    [X509NameValuePolicyEntry::new(
        COMMON_NAME,
        X509NameValueComparisonRule::ExactDer,
    )];
const DIRECTORY_STRING_POLICY_ENTRIES: [X509NameValuePolicyEntry<'static>; 1] =
    [X509NameValuePolicyEntry::new(
        COMMON_NAME,
        X509NameValueComparisonRule::DirectoryStringScalar,
    )];

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
    name_value(COMMON_NAME_OID, 0x0c, common_name)
}

fn name_value(oid: &[u8], value_tag: u8, value: &[u8]) -> Vec<u8> {
    let mut attribute = der(0x06, oid);
    attribute.extend(der(value_tag, value));
    der(0x30, &der(0x31, &der(0x30, &attribute)))
}

fn name_policy(rule: X509NameValueComparisonRule) -> X509NameValuePolicy<'static> {
    let entries = match rule {
        X509NameValueComparisonRule::ExactDer => &EXACT_POLICY_ENTRIES,
        X509NameValueComparisonRule::DirectoryStringScalar => &DIRECTORY_STRING_POLICY_ENTRIES,
    };
    X509NameValuePolicy::new(entries).unwrap()
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

#[test]
fn policy_backed_selection_retains_exact_unique_candidate() {
    let wanted = name(b"issuing-ca");
    let child = decoded_certificate(1, &wanted, &name(b"service"));
    let candidates = vec![
        decoded_certificate(2, &name(b"root"), &name(b"other-ca")),
        decoded_certificate(3, &name(b"root"), &wanted),
        decoded_certificate(4, &name(b"root"), &name(b"last-ca")),
    ];

    let selected = select_unique_x509_issuer_candidate_with_name_policy(
        &child,
        &candidates,
        3,
        name_policy(X509NameValueComparisonRule::ExactDer),
    )
    .unwrap();

    assert_eq!(selected.candidate_index(), 1);
    assert!(std::ptr::eq(selected.child_certificate(), &child));
    assert!(std::ptr::eq(
        selected.candidate_certificate(),
        &candidates[1]
    ));
    assert_eq!(
        selected.name_policy(),
        name_policy(X509NameValueComparisonRule::ExactDer)
    );
    assert_eq!(
        format!("{selected:?}"),
        "SelectedX509IssuerCandidateWithNamePolicy"
    );
}

#[test]
fn policy_backed_selection_uses_directory_string_scalar_rule() {
    let utf8 = name_value(COMMON_NAME_OID, 0x0c, "Aé".as_bytes());
    let bmp = name_value(COMMON_NAME_OID, 0x1e, &[0x00, 0x41, 0x00, 0xe9]);
    let child = decoded_certificate(1, &utf8, &name(b"service"));
    let candidates = vec![
        decoded_certificate(2, &name(b"root"), &name(b"other")),
        decoded_certificate(3, &name(b"root"), &bmp),
    ];

    let selected = select_unique_x509_issuer_candidate_with_name_policy(
        &child,
        &candidates,
        2,
        name_policy(X509NameValueComparisonRule::DirectoryStringScalar),
    )
    .unwrap();

    assert_eq!(selected.candidate_index(), 1);
}

#[test]
fn policy_backed_selection_maps_absence_and_ambiguity() {
    let wanted = name(b"issuing-ca");
    let child = decoded_certificate(1, &wanted, &name(b"service"));
    let policy = name_policy(X509NameValueComparisonRule::ExactDer);

    let no_match = vec![decoded_certificate(2, &name(b"root"), &name(b"other"))];
    assert_eq!(
        select_unique_x509_issuer_candidate_with_name_policy(&child, &no_match, 1, policy)
            .unwrap_err()
            .kind(),
        X509NamePolicyIssuerCandidateSelectionErrorKind::Selection(
            X509IssuerCandidateSelectionErrorKind::NoMatch
        )
    );

    let ambiguous = vec![
        decoded_certificate(3, &name(b"root"), &wanted),
        decoded_certificate(4, &name(b"root"), &wanted),
    ];
    assert_eq!(
        select_unique_x509_issuer_candidate_with_name_policy(&child, &ambiguous, 2, policy)
            .unwrap_err()
            .kind(),
        X509NamePolicyIssuerCandidateSelectionErrorKind::Selection(
            X509IssuerCandidateSelectionErrorKind::AmbiguousMatch
        )
    );
}

#[test]
fn policy_backed_limits_win_before_name_policy_evaluation() {
    let child = decoded_certificate(1, &name(b"issuer"), &name(b"service"));
    let candidates = vec![
        decoded_certificate(2, &name(b"root"), &name(b"issuer")),
        decoded_certificate(3, &name(b"root"), &name(b"issuer")),
    ];
    let empty_policy = X509NameValuePolicy::new(&[]).unwrap();

    assert_eq!(
        select_unique_x509_issuer_candidate_with_name_policy(
            &child,
            &candidates[..1],
            0,
            empty_policy,
        )
        .unwrap_err()
        .kind(),
        X509NamePolicyIssuerCandidateSelectionErrorKind::Selection(
            X509IssuerCandidateSelectionErrorKind::InvalidCandidateLimit
        )
    );
    assert_eq!(
        select_unique_x509_issuer_candidate_with_name_policy(&child, &candidates, 1, empty_policy,)
            .unwrap_err()
            .kind(),
        X509NamePolicyIssuerCandidateSelectionErrorKind::Selection(
            X509IssuerCandidateSelectionErrorKind::CandidateLimitExceeded
        )
    );
}

#[test]
fn policy_backed_selection_propagates_first_name_failure_without_payloads() {
    let child = decoded_certificate(1, &name(b"secret-issuer"), &name(b"service"));
    let candidates = vec![
        decoded_certificate(
            2,
            &name(b"root"),
            &name_value(COUNTRY_NAME_OID, 0x13, b"SECRET-COUNTRY"),
        ),
        decoded_certificate(
            3,
            &name(b"root"),
            &name_value(COMMON_NAME_OID, 0x16, b"SECRET-MALFORMED"),
        ),
    ];

    let error = select_unique_x509_issuer_candidate_with_name_policy(
        &child,
        &candidates,
        2,
        name_policy(X509NameValueComparisonRule::DirectoryStringScalar),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509NamePolicyIssuerCandidateSelectionErrorKind::NameComparison(
            X509NameValueComparisonErrorKind::UnconfiguredAttributeType
        )
    );
    assert_eq!(
        error.to_string(),
        "X.509 Name-policy issuer-candidate selection error NameComparison(UnconfiguredAttributeType)"
    );
    assert!(!error.to_string().contains("secret-issuer"));
    assert!(!error.to_string().contains("SECRET"));
}

#[test]
fn policy_backed_selection_propagates_directory_string_failures() {
    let child = decoded_certificate(1, &name(b"issuer"), &name(b"service"));
    let policy = name_policy(X509NameValueComparisonRule::DirectoryStringScalar);

    for (tag, expected) in [
        (
            0x16,
            X509NameValueComparisonErrorKind::InvalidDirectoryString,
        ),
        (
            0x14,
            X509NameValueComparisonErrorKind::UnsupportedTeletexString,
        ),
    ] {
        let candidates = vec![decoded_certificate(
            2,
            &name(b"root"),
            &name_value(COMMON_NAME_OID, tag, b"issuer"),
        )];
        assert_eq!(
            select_unique_x509_issuer_candidate_with_name_policy(&child, &candidates, 1, policy,)
                .unwrap_err()
                .kind(),
            X509NamePolicyIssuerCandidateSelectionErrorKind::NameComparison(expected)
        );
    }
}
