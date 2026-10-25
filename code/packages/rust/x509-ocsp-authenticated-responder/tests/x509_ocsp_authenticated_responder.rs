use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::RsaPublicOperationLimits;
use sha256::sha256;
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{
    validate_certificate_algorithm_policy, validate_certificate_role_policy,
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile, X509CertificateRole,
};
use x509_ocsp_authenticated_responder::{
    authenticate_x509_ocsp_rsa_sha256_responder, X509OcspAuthenticatedResponderErrorKind,
    X509OcspRsaSha256ResponderPolicy,
};
use x509_ocsp_basic_response::decode_x509_ocsp_basic_response;
use x509_ocsp_responder_certificate_policy::validate_x509_ocsp_responder_certificate_policy;
use x509_ocsp_responder_id_binding::bind_x509_ocsp_responder_id_to_certificate;
use x509_ocsp_rsa_pkcs1_v1_5_sha256_signature::X509OcspRsaSha256SignatureErrorKind;
use x509_rsa_public_key::{RsaPublicKeyErrorKind, RsaPublicKeyPolicy};
use x509_time::{decode_x509_time, X509Time};

const SHA256_WITH_RSA_ENCRYPTION_OID_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x0f];
const EXTENDED_KEY_USAGE_OID_DER: &[u8] = &[0x55, 0x1d, 0x25];
const OCSP_SIGNING_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x09];
const NULL_DER: &[u8] = &[0x05, 0x00];
const ALLOWED_EXPONENTS: &[u64] = &[65_537];

// OpenSSL 3.6.3 generated this one-off interoperability vector from synthetic
// public OCSP data. The private key existed only in process memory and was not
// retained. Audit trace:
//
// ResponseData SHA-256: 15f2736816151b2d3547907f4133d2ff8fd6bf5ae7e645fb8806c5c52c34c1ab
// public SPKI SHA-256: 425c2ae3ec7b195f1befc2c088cedc338d745a82d3bf9560b4f56abdf09d7d36
// signature SHA-256: dc2195f00db37ebdd0bfac2e1d5416d7687466e75a0a019757004d9ec4a9aa94
const RESPONSE_DATA_HEX: &str = "307fa11930173115301306035504030c0c6f6373702e6578616d706c65180f32303236313032343132303030305a3051304f303a300906052b0e03021a050004141111111111111111111111111111111111111111041422222222222222222222222222222222222222220201018000180f32303236313032343132333435365a";
const PUBLIC_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a02820101",
    "00e4ac7c6d5ac04e8fe5b469555e8f4344893851ee1c90b2c4c13d29ac30e4",
    "8af4910a30b7541be7773222c33f92bf37ae425504327f40ede5e569a7feaf90",
    "e21553d36629410b5bf3eb08a6e9fdd0816c107df36787afa5d54d45ee25c80",
    "29750f73a28e3738dd746f51b4ed3243e11ec1546c7eb421ebe1378597a5ee1",
    "7556c0c0df72eeac4ffc24c8190ee79bcc5aea7d55fbd4136be1f18ac499727",
    "d3c62e5debd818a1ae839e3195864054f5d72fae109faa1cb0de28d8124b60d",
    "152126a7023a232614b600d329c4595a2d789ef43f6fe2b6d33383e8c821606",
    "1eefaccf46efea8cccd22ef55b43485dd75b0af68dc831af9ae4a56c636c532",
    "ce0a03d22b0203010001",
);
const SIGNATURE_HEX: &str = concat!(
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
    assert!(hexadecimal.len().is_multiple_of(2));
    hexadecimal
        .as_bytes()
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

fn name(value: &[u8]) -> Vec<u8> {
    let attribute = sequence(&[element(0x06, &[0x55, 0x04, 0x03]), element(0x0c, value)]);
    sequence(&[element(0x31, &attribute)])
}

fn extension(oid: &[u8], critical: bool, value: &[u8]) -> Vec<u8> {
    let mut fields = vec![element(0x06, oid)];
    if critical {
        fields.push(element(0x01, &[0xff]));
    }
    fields.push(element(0x04, value));
    sequence(&fields)
}

fn responder_certificate() -> Vec<u8> {
    let validity = sequence(&[
        element(0x17, b"260101000000Z"),
        element(0x17, b"270101000000Z"),
    ]);
    let extensions = sequence(&[
        extension(KEY_USAGE_OID_DER, true, &element(0x03, &[0x07, 0x80])),
        extension(
            EXTENDED_KEY_USAGE_OID_DER,
            false,
            &sequence(&[element(0x06, OCSP_SIGNING_OID_DER)]),
        ),
    ]);

    let tbs = sequence(&[
        element(0xa0, &element(0x02, &[0x02])),
        element(0x02, &[0x01]),
        algorithm(SHA256_WITH_RSA_ENCRYPTION_OID_DER),
        name(b"issuer.example"),
        validity,
        name(b"ocsp.example"),
        decode_hex(PUBLIC_SPKI_HEX),
        element(0xa3, &extensions),
    ]);

    sequence(&[
        tbs,
        algorithm(SHA256_WITH_RSA_ENCRYPTION_OID_DER),
        element(0x03, &[0x00, 0xde, 0xad]),
    ])
}

fn response_with_signature(signature: &[u8]) -> Vec<u8> {
    let mut signature_bits = vec![0];
    signature_bits.extend_from_slice(signature);
    sequence(&[
        decode_hex(RESPONSE_DATA_HEX),
        algorithm(SHA256_WITH_RSA_ENCRYPTION_OID_DER),
        element(0x03, &signature_bits),
    ])
}

fn decode_time(value: &[u8; 13]) -> X509Time {
    let encoded = element(0x17, value);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decode_x509_time(decoder.decode_exact(&encoded).unwrap()).unwrap()
}

fn algorithm_profiles() -> [X509CertificateAlgorithmProfile<'static>; 1] {
    [X509CertificateAlgorithmProfile::new(
        SHA256_WITH_RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        RSA_ENCRYPTION_OID,
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        1,
        4_096,
        1,
        4_096,
    )
    .unwrap()]
}

fn responder_policy(minimum_modulus_bits: usize) -> X509OcspRsaSha256ResponderPolicy<'static> {
    X509OcspRsaSha256ResponderPolicy::new(
        RsaPublicKeyPolicy::new(minimum_modulus_bits, 4_096, ALLOWED_EXPONENTS).unwrap(),
        Asn1Limits::default(),
        RsaPublicOperationLimits::new(512).unwrap(),
    )
}

#[test]
fn checked_in_fixture_bytes_match_the_signing_trace() {
    assert_eq!(
        sha256(&decode_hex(RESPONSE_DATA_HEX)),
        decode_hex("15f2736816151b2d3547907f4133d2ff8fd6bf5ae7e645fb8806c5c52c34c1ab").as_slice()
    );
    assert_eq!(
        sha256(&decode_hex(PUBLIC_SPKI_HEX)),
        decode_hex("425c2ae3ec7b195f1befc2c088cedc338d745a82d3bf9560b4f56abdf09d7d36").as_slice()
    );
    assert_eq!(
        sha256(&decode_hex(SIGNATURE_HEX)),
        decode_hex("dc2195f00db37ebdd0bfac2e1d5416d7687466e75a0a019757004d9ec4a9aa94").as_slice()
    );
}

#[test]
fn authenticates_one_exact_policy_approved_responder_candidate_and_response() {
    let certificate_encoded = responder_certificate();
    let certificate = decode_x509_certificate(&certificate_encoded, Asn1Limits::default()).unwrap();
    let response_encoded = response_with_signature(&decode_hex(SIGNATURE_HEX));
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(&response_encoded).unwrap();
    let response = decode_x509_ocsp_basic_response(&mut decoder, root).unwrap();

    let responder_id =
        bind_x509_ocsp_responder_id_to_certificate(&response, &certificate, |left, right| {
            left.encoded() == right.encoded()
        })
        .unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();
    let certificate_policy = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &decode_time(b"261025120000Z"),
    )
    .unwrap();

    let authentication_policy = responder_policy(2_048);
    assert_eq!(
        authentication_policy
            .responder_key_policy()
            .minimum_modulus_bits(),
        2_048
    );
    assert_eq!(
        authentication_policy.responder_key_limits(),
        Asn1Limits::default()
    );
    assert_eq!(
        authentication_policy
            .public_operation_limits()
            .maximum_modulus_bytes(),
        512
    );
    let authenticated = authenticate_x509_ocsp_rsa_sha256_responder(
        responder_id,
        certificate_policy,
        authentication_policy,
    )
    .unwrap();

    assert!(std::ptr::eq(
        authenticated.responder_id_binding().response(),
        authenticated.response_signature().response()
    ));
    assert!(std::ptr::eq(
        authenticated.responder_id_binding().certificate(),
        authenticated.responder_certificate_policy().certificate()
    ));
    assert_eq!(
        authenticated
            .response_signature()
            .responder_public_key()
            .modulus_bits(),
        2_048
    );
    assert_eq!(
        format!("{authenticated:?}"),
        "AuthenticatedX509OcspRsaSha256Responder"
    );
}

#[test]
fn rejects_byte_equal_policy_for_a_distinct_decoded_certificate_before_crypto() {
    let certificate_encoded = responder_certificate();
    let certificate = decode_x509_certificate(&certificate_encoded, Asn1Limits::default()).unwrap();
    let other_certificate =
        decode_x509_certificate(&certificate_encoded, Asn1Limits::default()).unwrap();
    let response_encoded = response_with_signature(&decode_hex(SIGNATURE_HEX));
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(&response_encoded).unwrap();
    let response = decode_x509_ocsp_basic_response(&mut decoder, root).unwrap();

    let responder_id =
        bind_x509_ocsp_responder_id_to_certificate(&response, &certificate, |left, right| {
            left.encoded() == right.encoded()
        })
        .unwrap();
    let role_policy = validate_certificate_role_policy(
        &other_certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&other_certificate, &algorithm_profiles()).unwrap();
    let certificate_policy = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &decode_time(b"261025120000Z"),
    )
    .unwrap();

    let error = authenticate_x509_ocsp_rsa_sha256_responder(
        responder_id,
        certificate_policy,
        responder_policy(3_072),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspAuthenticatedResponderErrorKind::EvidenceMismatch
    );
    assert_eq!(
        error.to_string(),
        "X.509 OCSP authenticated responder error EvidenceMismatch"
    );
}

#[test]
fn maps_responder_key_policy_failure_without_disclosing_key_bytes() {
    let certificate_encoded = responder_certificate();
    let certificate = decode_x509_certificate(&certificate_encoded, Asn1Limits::default()).unwrap();
    let response_encoded = response_with_signature(&decode_hex(SIGNATURE_HEX));
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(&response_encoded).unwrap();
    let response = decode_x509_ocsp_basic_response(&mut decoder, root).unwrap();
    let responder_id =
        bind_x509_ocsp_responder_id_to_certificate(&response, &certificate, |left, right| {
            left.encoded() == right.encoded()
        })
        .unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();
    let certificate_policy = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &decode_time(b"261025120000Z"),
    )
    .unwrap();

    let error = authenticate_x509_ocsp_rsa_sha256_responder(
        responder_id,
        certificate_policy,
        responder_policy(3_072),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        X509OcspAuthenticatedResponderErrorKind::ResponderPublicKey(
            RsaPublicKeyErrorKind::ModulusBitLengthOutOfRange
        )
    );
    assert!(!error.to_string().contains("e4ac7c6d"));
}

#[test]
fn maps_tampered_response_signature_without_disclosing_signature_bytes() {
    let certificate_encoded = responder_certificate();
    let certificate = decode_x509_certificate(&certificate_encoded, Asn1Limits::default()).unwrap();
    let mut signature = decode_hex(SIGNATURE_HEX);
    *signature.last_mut().unwrap() ^= 1;
    let response_encoded = response_with_signature(&signature);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(&response_encoded).unwrap();
    let response = decode_x509_ocsp_basic_response(&mut decoder, root).unwrap();
    let responder_id =
        bind_x509_ocsp_responder_id_to_certificate(&response, &certificate, |left, right| {
            left.encoded() == right.encoded()
        })
        .unwrap();
    let role_policy = validate_certificate_role_policy(
        &certificate,
        X509CertificateRole::EndEntity,
        Asn1Limits::default(),
        |oid| oid.equals(&[2, 5, 29, 15]),
    )
    .unwrap();
    let algorithm_policy =
        validate_certificate_algorithm_policy(&certificate, &algorithm_profiles()).unwrap();
    let certificate_policy = validate_x509_ocsp_responder_certificate_policy(
        role_policy,
        algorithm_policy,
        &decode_time(b"261025120000Z"),
    )
    .unwrap();

    let error = authenticate_x509_ocsp_rsa_sha256_responder(
        responder_id,
        certificate_policy,
        responder_policy(2_048),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        X509OcspAuthenticatedResponderErrorKind::ResponseSignature(
            X509OcspRsaSha256SignatureErrorKind::EncodedMessage(_)
                | X509OcspRsaSha256SignatureErrorKind::PublicOperation(_)
        )
    ));
    assert!(!error.to_string().contains("6f9afb10"));
}
