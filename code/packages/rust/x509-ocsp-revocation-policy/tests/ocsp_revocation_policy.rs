use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_ocsp_cert_id_binding::{
    bind_x509_ocsp_cert_id_to_certificates, BoundX509OcspCertId, X509OcspCertIdHashInput,
};
use x509_ocsp_revocation_policy::{
    validate_bound_x509_ocsp_revocation, X509OcspRevocationPolicyErrorKind,
};
use x509_ocsp_single_response::{decode_x509_ocsp_single_response, X509OcspSingleResponse};
use x509_revocation_policy::X509RevocationPolicyErrorKind;
use x509_time::{decode_x509_generalized_time, X509Time};

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const HASH_OID: &[u8] = &[0x2b, 0x0e, 0x03, 0x02, 0x1a];
const NAME_HASH: [u8; 4] = [0x11; 4];
const KEY_HASH: [u8; 4] = [0x22; 4];
const ISSUER_NAME: &[u8] = b"issuer-subject";
const ISSUER_KEY: &[u8] = &[0x30, 0x00];

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

fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
    let value: Vec<u8> = fields.iter().flatten().copied().collect();
    element(0x30, &value)
}

fn algorithm(oid: &[u8]) -> Vec<u8> {
    sequence(&[element(0x06, oid), element(0x05, &[])])
}

fn time(value: &[u8]) -> Vec<u8> {
    element(0x18, value)
}

fn name(label: &[u8]) -> Vec<u8> {
    let attribute = sequence(&[element(0x06, &[0x55, 0x04, 0x03]), element(0x0c, label)]);
    sequence(&[element(0x31, &attribute)])
}

fn cert_status(tag: u8) -> Vec<u8> {
    match tag {
        0x80 | 0x82 => vec![tag, 0x00],
        0xa1 => element(tag, &time(b"20261024090000Z")),
        _ => panic!("unsupported test status"),
    }
}

fn single_response(status_tag: u8, this_update: &[u8], next_update: Option<&[u8]>) -> Vec<u8> {
    let cert_id = sequence(&[
        algorithm(HASH_OID),
        element(0x04, &NAME_HASH),
        element(0x04, &KEY_HASH),
        element(0x02, &[2]),
    ]);
    let mut fields = vec![cert_id, cert_status(status_tag), time(this_update)];
    if let Some(next_update) = next_update {
        fields.push(element(0xa0, &time(next_update)));
    }
    sequence(&fields)
}

fn single_response_with_extension() -> Vec<u8> {
    let encoded = single_response(0x80, b"20261024120000Z", Some(b"20261024130000Z"));
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(&encoded).unwrap();
    let mut fields = decoder.sequence(root).unwrap();
    let mut values = Vec::new();
    while let Some(field) = fields.read(&mut decoder).unwrap() {
        values.extend_from_slice(field.encoded());
    }
    let extension = sequence(&[element(0x06, &[0x2a, 0x03]), element(0x04, &[])]);
    values.extend_from_slice(&element(0xa1, &sequence(&[extension])));
    element(0x30, &values)
}

fn certificate(serial: &[u8], issuer_label: &[u8], subject_label: &[u8], key: &[u8]) -> Vec<u8> {
    let validity = sequence(&[
        element(0x17, b"260101000000Z"),
        element(0x17, b"270101000000Z"),
    ]);
    let subject_public_key_info =
        sequence(&[algorithm(KEY_OID), element(0x03, &[&[0x00], key].concat())]);
    let tbs = sequence(&[
        element(0xa0, &element(0x02, &[2])),
        element(0x02, serial),
        algorithm(SIGNATURE_OID),
        name(issuer_label),
        validity,
        name(subject_label),
        subject_public_key_info,
    ]);
    sequence(&[
        tbs,
        algorithm(SIGNATURE_OID),
        element(0x03, &[0x00, 0xca, 0xfe]),
    ])
}

fn decode_response(encoded: &[u8]) -> X509OcspSingleResponse<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    decode_x509_ocsp_single_response(&mut decoder, root).unwrap()
}

fn decode_certificate(encoded: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(encoded, Asn1Limits::default()).unwrap()
}

fn observed_time(value: &[u8]) -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_generalized_time(decoder.decode_exact(&time(value)).unwrap()).unwrap()
}

fn bind<'response, 'response_encoded, 'target, 'target_encoded, 'issuer, 'issuer_encoded>(
    response: &'response X509OcspSingleResponse<'response_encoded>,
    target: &'target X509Certificate<'target_encoded>,
    issuer: &'issuer X509Certificate<'issuer_encoded>,
) -> BoundX509OcspCertId<
    'response,
    'response_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
> {
    bind_x509_ocsp_cert_id_to_certificates(response, target, issuer, |_, input| match input {
        X509OcspCertIdHashInput::TargetIssuerNameDer(_) => Some(NAME_HASH),
        X509OcspCertIdHashInput::IssuerSubjectPublicKey(_) => Some(KEY_HASH),
    })
    .unwrap()
}

fn fixture_certificates() -> (Vec<u8>, Vec<u8>) {
    (
        certificate(&[2], ISSUER_NAME, b"target", &[0x01]),
        certificate(&[1], b"root", ISSUER_NAME, ISSUER_KEY),
    )
}

#[test]
fn validates_exact_bound_good_response_and_retains_inputs() {
    let response_der = single_response(0x80, b"20261024120000Z", Some(b"20261024130000Z"));
    let (target_der, issuer_der) = fixture_certificates();
    let response = decode_response(&response_der);
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);
    let binding = bind(&response, &target, &issuer);
    let time = observed_time(b"20261024123000Z");

    let validated = validate_bound_x509_ocsp_revocation(&binding, time).unwrap();

    assert!(std::ptr::eq(validated.cert_id_binding(), &binding));
    assert!(std::ptr::eq(
        validated.cert_id_binding().response(),
        &response
    ));
    assert!(std::ptr::eq(validated.cert_id_binding().target(), &target));
    assert!(std::ptr::eq(validated.cert_id_binding().issuer(), &issuer));
    assert_eq!(validated.observation_time(), time);
    assert_eq!(format!("{validated:?}"), "ValidatedBoundX509OcspRevocation");
}

#[test]
fn accepts_this_update_and_rejects_next_update_endpoint() {
    let response_der = single_response(0x80, b"20261024120000Z", Some(b"20261024130000Z"));
    let (target_der, issuer_der) = fixture_certificates();
    let response = decode_response(&response_der);
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);
    let binding = bind(&response, &target, &issuer);

    validate_bound_x509_ocsp_revocation(&binding, observed_time(b"20261024120000Z")).unwrap();
    let error = validate_bound_x509_ocsp_revocation(&binding, observed_time(b"20261024130000Z"))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        X509OcspRevocationPolicyErrorKind::Policy(X509RevocationPolicyErrorKind::StaleNextUpdate)
    );
}

#[test]
fn rejects_revoked_and_unknown_status_from_the_bound_response() {
    let (target_der, issuer_der) = fixture_certificates();
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);
    for (status_tag, expected) in [
        (0xa1, X509RevocationPolicyErrorKind::Revoked),
        (0x82, X509RevocationPolicyErrorKind::Unknown),
    ] {
        let response_der =
            single_response(status_tag, b"20261024120000Z", Some(b"20261024130000Z"));
        let response = decode_response(&response_der);
        let binding = bind(&response, &target, &issuer);
        let error =
            validate_bound_x509_ocsp_revocation(&binding, observed_time(b"20261024123000Z"))
                .unwrap_err();
        assert_eq!(
            error.kind(),
            X509OcspRevocationPolicyErrorKind::Policy(expected)
        );
    }
}

#[test]
fn requires_next_update_from_the_exact_bound_response() {
    let response_der = single_response(0x80, b"20261024120000Z", None);
    let (target_der, issuer_der) = fixture_certificates();
    let response = decode_response(&response_der);
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);
    let binding = bind(&response, &target, &issuer);

    let error = validate_bound_x509_ocsp_revocation(&binding, observed_time(b"20261024123000Z"))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        X509OcspRevocationPolicyErrorKind::Policy(X509RevocationPolicyErrorKind::MissingNextUpdate)
    );
}

#[test]
fn preserves_shared_freshness_failure_categories() {
    let (target_der, issuer_der) = fixture_certificates();
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);
    for (this_update, next_update, time, expected) in [
        (
            b"20261024130000Z".as_slice(),
            b"20261024120000Z".as_slice(),
            b"20261024123000Z".as_slice(),
            X509RevocationPolicyErrorKind::InvertedUpdateRange,
        ),
        (
            b"20261024120000Z".as_slice(),
            b"20261024130000Z".as_slice(),
            b"20261024115959Z".as_slice(),
            X509RevocationPolicyErrorKind::FutureThisUpdate,
        ),
        (
            b"20261024120000Z".as_slice(),
            b"20261024130000Z".as_slice(),
            b"20261024130001Z".as_slice(),
            X509RevocationPolicyErrorKind::StaleNextUpdate,
        ),
    ] {
        let response_der = single_response(0x80, this_update, Some(next_update));
        let response = decode_response(&response_der);
        let binding = bind(&response, &target, &issuer);
        let error = validate_bound_x509_ocsp_revocation(&binding, observed_time(time)).unwrap_err();
        assert_eq!(
            error.kind(),
            X509OcspRevocationPolicyErrorKind::Policy(expected)
        );
    }
}

#[test]
fn rejects_single_response_extensions_before_status_policy() {
    let response_der = single_response_with_extension();
    let (target_der, issuer_der) = fixture_certificates();
    let response = decode_response(&response_der);
    let target = decode_certificate(&target_der);
    let issuer = decode_certificate(&issuer_der);
    let binding = bind(&response, &target, &issuer);

    let error = validate_bound_x509_ocsp_revocation(&binding, observed_time(b"20261024123000Z"))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        X509OcspRevocationPolicyErrorKind::UnsupportedSingleResponseExtensions
    );
    assert!(!error.to_string().contains("1.2.3"));
}
