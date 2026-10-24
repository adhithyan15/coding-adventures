use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_revocation_policy::{
    evaluate_x509_revocation_observation, X509RevocationObservation, X509RevocationPolicyErrorKind,
    X509RevocationStatus,
};
use x509_time::{decode_x509_time, X509Time};

fn utc(contents: &[u8; 13]) -> X509Time {
    let mut encoded = vec![0x17, 13];
    encoded.extend_from_slice(contents);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder.decode_exact(&encoded).expect("valid DER framing");
    decode_x509_time(element).expect("valid profile time")
}

fn observation(
    status: X509RevocationStatus,
    this_update: Option<&[u8; 13]>,
    next_update: Option<&[u8; 13]>,
) -> X509RevocationObservation {
    X509RevocationObservation::new(status, this_update.map(utc), next_update.map(utc))
}

fn kind(
    observation: Option<&X509RevocationObservation>,
    observed: &[u8; 13],
) -> X509RevocationPolicyErrorKind {
    evaluate_x509_revocation_observation(observation, utc(observed))
        .unwrap_err()
        .kind()
}

#[test]
fn accepts_good_observations_at_inclusive_freshness_endpoints() {
    let revocation = observation(
        X509RevocationStatus::Good,
        Some(b"241001000000Z"),
        Some(b"241002000000Z"),
    );

    for observed in [b"241001000000Z", b"241002000000Z"] {
        let accepted = evaluate_x509_revocation_observation(Some(&revocation), utc(observed))
            .expect("inclusive fresh good observation");
        assert!(core::ptr::eq(accepted.observation(), &revocation));
        assert_eq!(accepted.observation_time(), utc(observed));
        assert_eq!(format!("{accepted:?}"), "X509RevocationPolicyAcceptance");
        assert_eq!(accepted.observation().status(), X509RevocationStatus::Good);
        assert!(accepted.observation().this_update().is_some());
        assert!(accepted.observation().next_update().is_some());
    }
}

#[test]
fn missing_revoked_and_unknown_observations_fail_closed() {
    let observed = b"241001120000Z";
    assert_eq!(
        kind(None, observed),
        X509RevocationPolicyErrorKind::MissingObservation
    );

    for (status, expected) in [
        (
            X509RevocationStatus::Revoked,
            X509RevocationPolicyErrorKind::Revoked,
        ),
        (
            X509RevocationStatus::Unknown,
            X509RevocationPolicyErrorKind::Unknown,
        ),
    ] {
        let revocation = observation(status, None, None);
        assert_eq!(kind(Some(&revocation), observed), expected);
    }
}

#[test]
fn both_freshness_endpoints_are_mandatory() {
    let observed = b"241001120000Z";
    let missing_this = observation(X509RevocationStatus::Good, None, Some(b"241002000000Z"));
    assert_eq!(
        kind(Some(&missing_this), observed),
        X509RevocationPolicyErrorKind::MissingThisUpdate
    );

    let missing_next = observation(X509RevocationStatus::Good, Some(b"241001000000Z"), None);
    assert_eq!(
        kind(Some(&missing_next), observed),
        X509RevocationPolicyErrorKind::MissingNextUpdate
    );
}

#[test]
fn inverted_future_and_stale_ranges_fail_closed() {
    let inverted = observation(
        X509RevocationStatus::Good,
        Some(b"241002000000Z"),
        Some(b"241001000000Z"),
    );
    assert_eq!(
        kind(Some(&inverted), b"241001120000Z"),
        X509RevocationPolicyErrorKind::InvertedUpdateRange
    );

    let bounded = observation(
        X509RevocationStatus::Good,
        Some(b"241001000000Z"),
        Some(b"241002000000Z"),
    );
    assert_eq!(
        kind(Some(&bounded), b"240930235959Z"),
        X509RevocationPolicyErrorKind::FutureThisUpdate
    );
    assert_eq!(
        kind(Some(&bounded), b"241002000001Z"),
        X509RevocationPolicyErrorKind::StaleNextUpdate
    );
}

#[test]
fn errors_are_stable_and_payload_free() {
    let revocation = observation(
        X509RevocationStatus::Good,
        Some(b"241001000000Z"),
        Some(b"241002000000Z"),
    );
    let error =
        evaluate_x509_revocation_observation(Some(&revocation), utc(b"241003123456Z")).unwrap_err();

    assert_eq!(error.kind(), X509RevocationPolicyErrorKind::StaleNextUpdate);
    assert_eq!(
        error.to_string(),
        "X.509 revocation policy error StaleNextUpdate"
    );
    assert!(!error.to_string().contains("241003123456"));
}
