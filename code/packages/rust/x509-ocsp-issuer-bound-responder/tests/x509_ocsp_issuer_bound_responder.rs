use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::RsaPublicOperationLimits;
use sha256::sha256;
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_certificate_policy::{
    validate_certificate_algorithm_policy, validate_certificate_role_policy,
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile, X509CertificateRole,
};
use x509_ocsp_authenticated_responder::{
    authenticate_x509_ocsp_rsa_sha256_responder, AuthenticatedX509OcspRsaSha256Responder,
    X509OcspRsaSha256ResponderPolicy,
};
use x509_ocsp_basic_response::{decode_x509_ocsp_basic_response, X509OcspBasicResponse};
use x509_ocsp_issuer_bound_responder::{
    bind_x509_ocsp_authenticated_responder_to_issuer, X509OcspIssuerBoundResponderErrorKind,
};
use x509_ocsp_responder_certificate_policy::validate_x509_ocsp_responder_certificate_policy;
use x509_ocsp_responder_id_binding::bind_x509_ocsp_responder_id_to_certificate;
use x509_rsa_public_key::RsaPublicKeyPolicy;
use x509_rsa_sha256_issuer_link::{
    verify_x509_rsa_sha256_issuer_link, VerifiedX509RsaSha256IssuerLink,
    X509RsaSha256IssuerLinkPolicy,
};
use x509_time::{decode_x509_time, X509Time};

const SHA256_WITH_RSA_ENCRYPTION_OID_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const NULL_DER: &[u8] = &[0x05, 0x00];
const ALLOWED_EXPONENTS: &[u64] = &[65_537];

const ISSUER_CERTIFICATE_SHA256: &str =
    "3fad12f7deaeb31ad73d9342f823d2c34e7b46c9636c8e25b4745d6777072fa5";
const RESPONDER_CERTIFICATE_SHA256: &str =
    "38a954a1080409d5fd31d50581af31ec5df84f75814468108dd27a172f90f66e";

// The response vector and responder public key are shared with the preceding
// authentication package. OpenSSL 3.6.3 generated the public certificate pair
// once on 2026-10-25. The test-only issuer key was not retained in the repo.
const RESPONSE_DATA_HEX: &str = "307fa11930173115301306035504030c0c6f6373702e6578616d706c65180f32303236313032343132303030305a3051304f303a300906052b0e03021a050004141111111111111111111111111111111111111111041422222222222222222222222222222222222222220201018000180f32303236313032343132333435365a";
const RESPONSE_SIGNATURE_HEX: &str = concat!(
    "6f9afb10383405539fb2d2d55893f70bcc058e77dee4b04d12020b354ca71d83",
    "ed9ee9e3cd0fef51d52df2cdb433031479b82324b1002b10ae919116d8d89016",
    "8e35d13f242eb641031f1ec5757933a8f2a888bf661ca9734b2e236e24e98144",
    "39ee4abeb7b1482e06593b4d5a0fc6104e47036c294d4f86e9993c91d00ae3a",
    "5845db98fc0ca1c04478cb90eb3fabba3dd44d7aebdb1de2884b976333d05705",
    "9c90c06ca7f71ca13b2f053f5825cb70fbb9bac173d9bb033b5b05ae4505593",
    "8d290bc5ef3198f488b086832b5ab312de502240a0dc147ee4cab706ee7030385",
    "cc027acc7de9a38f1038a25353c0c37901c302cb489d900992fa10470240daa1c",
);

fn decode_hex(hexadecimal: &str) -> Vec<u8> {
    let digits = hexadecimal
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    assert!(digits.len().is_multiple_of(2));
    digits
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}

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
    element(0x30, &fields.iter().flatten().copied().collect::<Vec<_>>())
}

fn algorithm(oid: &[u8]) -> Vec<u8> {
    sequence(&[element(0x06, oid), NULL_DER.to_vec()])
}

fn response_with_signature() -> Vec<u8> {
    let mut signature_bits = vec![0];
    signature_bits.extend_from_slice(&decode_hex(RESPONSE_SIGNATURE_HEX));
    sequence(&[
        decode_hex(RESPONSE_DATA_HEX),
        algorithm(SHA256_WITH_RSA_ENCRYPTION_OID_DER),
        element(0x03, &signature_bits),
    ])
}

fn observed_time() -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(b"\x17\x0d261026000000Z").unwrap()).unwrap()
}

fn algorithm_profiles() -> [X509CertificateAlgorithmProfile<'static>; 1] {
    [X509CertificateAlgorithmProfile::new(
        SHA256_WITH_RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        2_048,
        4_096,
        2_048,
        4_096,
    )
    .unwrap()]
}

fn authenticate<'response, 'response_encoded, 'responder, 'responder_encoded>(
    response: &'response X509OcspBasicResponse<'response_encoded>,
    responder: &'responder X509Certificate<'responder_encoded>,
) -> AuthenticatedX509OcspRsaSha256Responder<
    'response,
    'response_encoded,
    'responder,
    'responder_encoded,
> {
    let responder_id =
        bind_x509_ocsp_responder_id_to_certificate(response, responder, |left, right| {
            left.encoded() == right.encoded()
        })
        .unwrap();
    let role_policy = validate_certificate_role_policy(
        responder,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 19]) || oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(responder, &algorithm_profiles()).unwrap();
    let certificate_policy = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &observed_time(),
    )
    .unwrap();
    authenticate_x509_ocsp_rsa_sha256_responder(
        responder_id,
        certificate_policy,
        X509OcspRsaSha256ResponderPolicy::new(
            RsaPublicKeyPolicy::new(2_048, 4_096, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            RsaPublicOperationLimits::new(512).unwrap(),
        ),
    )
    .unwrap()
}

fn issuer_link<'responder, 'responder_encoded, 'issuer, 'issuer_encoded>(
    responder: &'responder X509Certificate<'responder_encoded>,
    issuer: &'issuer X509Certificate<'issuer_encoded>,
) -> VerifiedX509RsaSha256IssuerLink<'responder, 'responder_encoded, 'issuer, 'issuer_encoded> {
    verify_x509_rsa_sha256_issuer_link(
        responder,
        issuer,
        |child_issuer, candidate_subject| child_issuer.encoded() == candidate_subject.encoded(),
        X509RsaSha256IssuerLinkPolicy::new(
            Asn1Limits::default(),
            RsaPublicKeyPolicy::new(2_048, 4_096, ALLOWED_EXPONENTS).unwrap(),
            Asn1Limits::default(),
            RsaPublicOperationLimits::new(512).unwrap(),
        ),
    )
    .unwrap()
}

#[test]
fn checked_in_public_certificates_match_the_generation_trace() {
    let issuer = decode_hex(include_str!("fixtures/issuer.der.hex"));
    let responder = decode_hex(include_str!("fixtures/responder.der.hex"));
    assert_eq!(
        sha256(&issuer).as_slice(),
        decode_hex(ISSUER_CERTIFICATE_SHA256).as_slice()
    );
    assert_eq!(
        sha256(&responder).as_slice(),
        decode_hex(RESPONDER_CERTIFICATE_SHA256).as_slice()
    );
}

#[test]
fn binds_one_exact_authenticated_responder_to_its_candidate_issuer() {
    let responder_encoded = decode_hex(include_str!("fixtures/responder.der.hex"));
    let issuer_encoded = decode_hex(include_str!("fixtures/issuer.der.hex"));
    let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
    let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
    let response_encoded = response_with_signature();
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(&response_encoded).unwrap();
    let response = decode_x509_ocsp_basic_response(&mut decoder, root).unwrap();

    let bound = bind_x509_ocsp_authenticated_responder_to_issuer(
        authenticate(&response, &responder),
        issuer_link(&responder, &issuer),
    )
    .unwrap();

    assert!(std::ptr::eq(
        bound
            .authenticated_responder()
            .responder_certificate_policy()
            .certificate(),
        bound.responder_issuer_link().child()
    ));
    assert!(std::ptr::eq(
        bound.responder_issuer_link().candidate_issuer(),
        &issuer
    ));
    assert_eq!(
        format!("{bound:?}"),
        "IssuerBoundX509OcspRsaSha256Responder"
    );
}

#[test]
fn rejects_a_byte_equal_distinct_responder_object() {
    let responder_encoded = decode_hex(include_str!("fixtures/responder.der.hex"));
    let issuer_encoded = decode_hex(include_str!("fixtures/issuer.der.hex"));
    let responder = decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
    let distinct_responder =
        decode_x509_certificate(&responder_encoded, Asn1Limits::default()).unwrap();
    let issuer = decode_x509_certificate(&issuer_encoded, Asn1Limits::default()).unwrap();
    let response_encoded = response_with_signature();
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(&response_encoded).unwrap();
    let response = decode_x509_ocsp_basic_response(&mut decoder, root).unwrap();

    let error = bind_x509_ocsp_authenticated_responder_to_issuer(
        authenticate(&response, &responder),
        issuer_link(&distinct_responder, &issuer),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspIssuerBoundResponderErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP issuer-bound responder error EvidenceMismatch"
    );
}
