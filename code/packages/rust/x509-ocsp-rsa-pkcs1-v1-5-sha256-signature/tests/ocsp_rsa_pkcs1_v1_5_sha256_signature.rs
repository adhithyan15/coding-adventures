use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::{RsaPublicOperationErrorKind, RsaPublicOperationLimits};
use x509_ocsp_basic_response::{decode_x509_ocsp_basic_response, X509OcspBasicResponse};
use x509_ocsp_rsa_pkcs1_v1_5_sha256_signature::{
    verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature, X509OcspRsaSha256SignatureErrorKind,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const SHA256_WITH_RSA_ENCRYPTION_OID_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const NULL_DER: &[u8] = &[0x05, 0x00];

// OpenSSL 3.6.3 generated this one-off interoperability vector from synthetic
// public OCSP data. The private key existed only in process memory and was not
// retained. Audit trace:
//
// ResponseData SHA-256: ca7ddddf9ea431b9bd338804380afd26c8dafc5b4bd651a9d48e2a72cf83684b
// public SPKI SHA-256: 201929b3487147eb38c26b3de772792259849d8355fa3150f239b8a738e218b5
// signature SHA-256: fc71cfd233c50c7f5c2853534cb243cd7f66e9f80a9eccb852c8777bf7783974
const RESPONSE_DATA_HEX: &str = concat!(
    "307ca21604144444444444444444444444444444444444444444180f32303236",
    "313032343132303030305a3051304f303a300906052b0e03021a050004141111",
    "1111111111111111111111111111111111110414222222222222222222222222",
    "2222222222222222220201018000180f32303236313032343132333435365a",
);
const PUBLIC_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a02820101",
    "00e550386fb8134a3a2e3e850d7a3d8ea5d6e601391b7c5c3327684f3d00b8",
    "6e517e4c210888b64d1f84699b74c18546a7dc992e9a4dcbaac6a6d688e653",
    "0d7080645513562c24c1cdb016f5bed4ce6b4b2ec2e07e63fceeec14f62805",
    "9ff62588e6a4f931714b85e1658de0b6ea4b2def535776efe7c0a816e99c3a",
    "a2202250fa12ea46de1a13d23722a0b38d9690a75823c851700ad5f274a9331",
    "c0ce16a78eb36af10291320c43a5685d4aaab91ae8dc8c7450977614c7200c",
    "ddaaff26a05e3435b885254f8477da784a026fed15773341329088b6b8fd729",
    "8a8fe8978ed1c06c37ce01dfe6b181e9378b860018ffc9ac5954448e4bf3f9",
    "bcb688498d282f210203010001",
);
const SIGNATURE_HEX: &str = concat!(
    "9c6c6cceada1c781da228be19c80408d63e311076efa3f88932bedfa7624cdaa",
    "98af8f6e9948483b26e75efab65bfaff0d5b60b5538e3301f8f1f4784460141",
    "63a3a5c6ee19a6b19691945158ab2a9b6258cd9183dd60b330cce640d7258d7",
    "dbf0ab26a4548a9ac7a9a67edd9f54ae50602a15e7cb839601a6a1b840c217",
    "662d8b835ffdbdbd38f55c02d031eb534efc257643439215f33703257219fd9",
    "6c0fbf3fff597f50b9954346195e8643b202ae86d2ca16d5e56f0c27f8dcfa",
    "b80a4df8a6ad10abf07a5b7aa879511d2b25f56663405cd6bf77abedd5a5fa",
    "f906fb5b073c78134731d0d92c8201065a3c2bcebb5464a1d65cbdad4f21fb",
    "850c0bf8a1f",
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

fn algorithm(oid: &[u8], parameters: Option<&[u8]>) -> Vec<u8> {
    let mut fields = vec![element(0x06, oid)];
    if let Some(parameters) = parameters {
        fields.push(parameters.to_vec());
    }
    sequence(&fields)
}

fn basic_response(
    response_data: &[u8],
    signature_oid: &[u8],
    signature_parameters: Option<&[u8]>,
    signature_bit_string: &[u8],
) -> Vec<u8> {
    sequence(&[
        response_data.to_vec(),
        algorithm(signature_oid, signature_parameters),
        element(0x03, signature_bit_string),
    ])
}

fn signed_response_data() -> Vec<u8> {
    decode_hex(RESPONSE_DATA_HEX)
}

fn fixture_signature() -> Vec<u8> {
    let mut bits = vec![0];
    bits.extend(decode_hex(SIGNATURE_HEX));
    bits
}

fn fixture_response() -> Vec<u8> {
    basic_response(
        &signed_response_data(),
        SHA256_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &fixture_signature(),
    )
}

fn decode_response(encoded: &[u8]) -> X509OcspBasicResponse<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    decode_x509_ocsp_basic_response(&mut decoder, root).unwrap()
}

fn validated_key(encoded: &[u8]) -> ValidatedRsaPublicKey<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    let spki = decode_x509_subject_public_key_info(&mut decoder, root).unwrap();
    let policy = RsaPublicKeyPolicy::new(2_048, 2_048, &[65_537]).unwrap();
    validate_rsa_public_key(spki, policy, Asn1Limits::default()).unwrap()
}

#[test]
fn verifies_independent_2048_bit_ocsp_fixture_and_binds_exact_inputs() {
    let encoded = fixture_response();
    let response = decode_response(&encoded);
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let verified = verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(
        &response,
        key,
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap();

    assert_eq!(
        format!("{verified:?}"),
        "VerifiedX509OcspRsaSha256Signature"
    );
    assert!(std::ptr::eq(verified.response(), &response));
    assert_eq!(verified.response().encoded(), encoded);
    assert_eq!(verified.responder_public_key(), key);
}

#[test]
fn rejects_tampered_response_data_and_a_different_validated_key() {
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let limits = RsaPublicOperationLimits::new(256).unwrap();

    let mut tampered_data = signed_response_data();
    let time_offset = tampered_data
        .windows(b"20261024123456Z".len())
        .position(|window| window == b"20261024123456Z")
        .unwrap();
    tampered_data[time_offset + 13] = b'5';
    let tampered_encoded = basic_response(
        &tampered_data,
        SHA256_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &fixture_signature(),
    );
    let tampered = decode_response(&tampered_encoded);
    let tampered_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(&tampered, key, limits).unwrap_err();
    assert!(matches!(
        tampered_error.kind(),
        X509OcspRsaSha256SignatureErrorKind::EncodedMessage(_)
    ));

    let encoded = fixture_response();
    let response = decode_response(&encoded);
    let mut different_spki = spki.clone();
    let modulus_offset = different_spki
        .windows(key.modulus().len())
        .position(|window| window == key.modulus())
        .unwrap();
    different_spki[modulus_offset + 64] ^= 2;
    let different_key = validated_key(&different_spki);
    assert_ne!(different_key, key);
    let wrong_key_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(&response, different_key, limits)
            .unwrap_err();
    assert!(matches!(
        wrong_key_error.kind(),
        X509OcspRsaSha256SignatureErrorKind::EncodedMessage(_)
            | X509OcspRsaSha256SignatureErrorKind::PublicOperation(_)
    ));
}

#[test]
fn rejects_unsupported_missing_and_invalid_algorithm_identifiers() {
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let limits = RsaPublicOperationLimits::new(256).unwrap();
    let data = signed_response_data();

    let unsupported = basic_response(&data, RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &[0, 0]);
    let missing = basic_response(&data, SHA256_WITH_RSA_ENCRYPTION_OID_DER, None, &[0, 0]);
    let invalid = basic_response(
        &data,
        SHA256_WITH_RSA_ENCRYPTION_OID_DER,
        Some(&[0x04, 0x00]),
        &[0, 0],
    );

    let unsupported_response = decode_response(&unsupported);
    let unsupported_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(&unsupported_response, key, limits)
            .unwrap_err();
    assert_eq!(
        unsupported_error.kind(),
        X509OcspRsaSha256SignatureErrorKind::UnsupportedSignatureAlgorithm
    );

    let missing_response = decode_response(&missing);
    let missing_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(&missing_response, key, limits)
            .unwrap_err();
    assert_eq!(
        missing_error.kind(),
        X509OcspRsaSha256SignatureErrorKind::MissingSignatureAlgorithmParameters
    );

    let invalid_response = decode_response(&invalid);
    let invalid_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(&invalid_response, key, limits)
            .unwrap_err();
    assert_eq!(
        invalid_error.kind(),
        X509OcspRsaSha256SignatureErrorKind::InvalidSignatureAlgorithmParameters
    );
    assert!(!invalid_error.to_string().contains("44444444"));
}

#[test]
fn rejects_non_byte_aligned_and_wrong_width_signatures() {
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let limits = RsaPublicOperationLimits::new(256).unwrap();
    let data = signed_response_data();

    let non_byte_aligned = basic_response(
        &data,
        SHA256_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &[1, 0],
    );
    let alignment_response = decode_response(&non_byte_aligned);
    let alignment_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(&alignment_response, key, limits)
            .unwrap_err();
    assert_eq!(
        alignment_error.kind(),
        X509OcspRsaSha256SignatureErrorKind::NonByteAlignedSignature
    );

    let wrong_width = basic_response(
        &data,
        SHA256_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &[0, 0],
    );
    let width_response = decode_response(&wrong_width);
    let width_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(&width_response, key, limits).unwrap_err();
    assert_eq!(
        width_error.kind(),
        X509OcspRsaSha256SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::SignatureLengthMismatch
        )
    );
}

#[test]
fn maps_limits_and_corrupted_signatures_to_payload_free_categories() {
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let encoded = fixture_response();
    let response = decode_response(&encoded);
    let limit_error = verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(
        &response,
        key,
        RsaPublicOperationLimits::new(255).unwrap(),
    )
    .unwrap_err();
    assert_eq!(
        limit_error.kind(),
        X509OcspRsaSha256SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::ModulusTooLong
        )
    );

    let mut corrupted_signature = fixture_signature();
    *corrupted_signature.last_mut().unwrap() ^= 1;
    let corrupted_encoded = basic_response(
        &signed_response_data(),
        SHA256_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &corrupted_signature,
    );
    let corrupted = decode_response(&corrupted_encoded);
    let signature_error = verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(
        &corrupted,
        key,
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        signature_error.kind(),
        X509OcspRsaSha256SignatureErrorKind::EncodedMessage(_)
            | X509OcspRsaSha256SignatureErrorKind::PublicOperation(_)
    ));
    assert!(!signature_error.to_string().contains("9c6c6cce"));
    assert!(!format!("{signature_error:?}").contains("9c6c6cce"));
}
