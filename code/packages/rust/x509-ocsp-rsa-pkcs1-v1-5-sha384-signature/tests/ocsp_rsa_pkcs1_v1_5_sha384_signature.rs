use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::{RsaPublicOperationErrorKind, RsaPublicOperationLimits};
use sha384::sha384;
use x509_ocsp_basic_response::{decode_x509_ocsp_basic_response, X509OcspBasicResponse};
use x509_ocsp_rsa_pkcs1_v1_5_sha384_signature::{
    verify_x509_ocsp_rsa_pkcs1_v1_5_sha384_signature, X509OcspRsaSha384SignatureErrorKind,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const SHA384_WITH_RSA_ENCRYPTION_OID_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0c];
const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const NULL_DER: &[u8] = &[0x05, 0x00];

// OpenSSL generated this one-off interoperability vector from synthetic public
// OCSP data. Its temporary private key was deleted immediately after fixture
// extraction and is not retained. Public SHA-384 digests bind the fixture back
// to the generation trace.
const RESPONSE_DATA_HEX: &str = concat!(
    "307ca21604144444444444444444444444444444444444444444180f32303236",
    "313032343132303030305a3051304f303a300906052b0e03021a050004141111",
    "1111111111111111111111111111111111110414222222222222222222222222",
    "22222222222222220201018000180f32303236313032343132333435365a",
);
const PUBLIC_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a02820101",
    "00d55c8dc03f3903bace760f486dcf94f5e8c13da191b6daa22ff3e3a1aa02",
    "9c4f69c86de686820a513b9f7d876a76c444a7afd944fb49ececb7c79b21e6",
    "84ccd710b39c59bcc7249a24b0a36a3899cb9d7a299c51dcc820c35396e5ec",
    "bc722b98bcd9e2ee78c7e7ea92ce2f428e994174969514e7bb959e96017372",
    "c3d92fd4575ce9a7d8a8e93710b504bb3e7c2e9d229b92391b487a27f7efd",
    "31ea60e5b0a11e98760bc909d7b058501897dda566f90e5eb5ca6cb44805dd",
    "62a948e02aa13b4f36c42fe150499198309aca94b6d4fbd6d1eeae3a345678",
    "a8af60800bb0da7c7a1f02c1fbbd9b602ec0f26af496b1426082b67b33c02",
    "75d3444a9e06dc64f49d0203010001",
);
const SIGNATURE_HEX: &str = concat!(
    "2c37793c890866bdc75e9e4090d33b4764f7d21fb9e394956ef1f3c0f7c99a79",
    "56b07589d6e6e3279b6409124b8924ed49ee8efbaa9d68040f283adcf7b91bdd",
    "61948a479d006c0e6fa4a7b528f601bb0b16c52a2ea59fd34b838bde7ca18a34",
    "496efeb304de4d14535ff201625ee26c4844d68998728a50fbd0879d31b4ad3d",
    "ccb7d9d026ca417e0376567ab7e699a1eff3ae4f19a2d79903fecefe8394b96a",
    "19f79942b034d16859ac16e1a62af7b37e3cef2acee492cc90f3f7df23d6ea2c",
    "35ab1b7dc0dc1e15a035ab6c9588e0d16bce31e9ecd4f4d49c5d9c0c227495a3",
    "b03e33ca27c004efc328fff3673e56fb8a0f93f801c70ec810ee29e99fe308d9",
);

fn decode_hex(hexadecimal: &str) -> Vec<u8> {
    assert!(hexadecimal.len().is_multiple_of(2));
    hexadecimal
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
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
        SHA384_WITH_RSA_ENCRYPTION_OID_DER,
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
fn checked_in_fixture_bytes_match_the_signing_trace() {
    assert_eq!(
        sha384(&signed_response_data()).as_slice(),
        decode_hex("6bcdb82b50dc7ae8555a2e6f47a9a4600b84976c02e426976a252f513f5923e2e3477db41d02746bd692ebb12552bb95")
    );
    assert_eq!(
        sha384(&decode_hex(PUBLIC_SPKI_HEX)).as_slice(),
        decode_hex("3975316f6aa77e2e5dca9bcc14068291e74853cbb42a48a8672b9a3286082219683c8d8bf3363a1dacb1685b84c86af5")
    );
    assert_eq!(
        sha384(&decode_hex(SIGNATURE_HEX)).as_slice(),
        decode_hex("f36d5c5461c87dc7cc6a2abc2f8bf77d382bd64e9570956efa850f3edeedf7dd675793f6110cf1faf9fc4c47f7eff8c3")
    );
}

#[test]
fn verifies_independent_2048_bit_fixture_and_binds_exact_inputs() {
    let encoded = fixture_response();
    let response = decode_response(&encoded);
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let verified = verify_x509_ocsp_rsa_pkcs1_v1_5_sha384_signature(
        &response,
        key,
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap();

    assert_eq!(
        format!("{verified:?}"),
        "VerifiedX509OcspRsaSha384Signature"
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
    let offset = tampered_data
        .windows(b"20261024123456Z".len())
        .position(|window| window == b"20261024123456Z")
        .unwrap();
    tampered_data[offset + 13] = b'5';
    let tampered_encoded = basic_response(
        &tampered_data,
        SHA384_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &fixture_signature(),
    );
    let tampered = decode_response(&tampered_encoded);
    assert!(matches!(
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha384_signature(&tampered, key, limits)
            .unwrap_err()
            .kind(),
        X509OcspRsaSha384SignatureErrorKind::EncodedMessage(_)
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
    assert!(matches!(
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha384_signature(&response, different_key, limits)
            .unwrap_err()
            .kind(),
        X509OcspRsaSha384SignatureErrorKind::EncodedMessage(_)
            | X509OcspRsaSha384SignatureErrorKind::PublicOperation(_)
    ));
}

#[test]
fn rejects_unsupported_missing_and_invalid_algorithm_identifiers() {
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let limits = RsaPublicOperationLimits::new(256).unwrap();
    let data = signed_response_data();

    let unsupported = basic_response(&data, RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &[0, 0]);
    let missing = basic_response(&data, SHA384_WITH_RSA_ENCRYPTION_OID_DER, None, &[0, 0]);
    let invalid = basic_response(
        &data,
        SHA384_WITH_RSA_ENCRYPTION_OID_DER,
        Some(&[0x04, 0x00]),
        &[0, 0],
    );

    assert_eq!(
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha384_signature(
            &decode_response(&unsupported),
            key,
            limits,
        )
        .unwrap_err()
        .kind(),
        X509OcspRsaSha384SignatureErrorKind::UnsupportedSignatureAlgorithm
    );
    assert_eq!(
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha384_signature(&decode_response(&missing), key, limits,)
            .unwrap_err()
            .kind(),
        X509OcspRsaSha384SignatureErrorKind::MissingSignatureAlgorithmParameters
    );
    let invalid_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha384_signature(&decode_response(&invalid), key, limits)
            .unwrap_err();
    assert_eq!(
        invalid_error.kind(),
        X509OcspRsaSha384SignatureErrorKind::InvalidSignatureAlgorithmParameters
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
        SHA384_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &[1, 0],
    );
    assert_eq!(
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha384_signature(
            &decode_response(&non_byte_aligned),
            key,
            limits,
        )
        .unwrap_err()
        .kind(),
        X509OcspRsaSha384SignatureErrorKind::NonByteAlignedSignature
    );

    let wrong_width = basic_response(
        &data,
        SHA384_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &[0, 0],
    );
    assert_eq!(
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha384_signature(
            &decode_response(&wrong_width),
            key,
            limits,
        )
        .unwrap_err()
        .kind(),
        X509OcspRsaSha384SignatureErrorKind::PublicOperation(
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
    assert_eq!(
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha384_signature(
            &response,
            key,
            RsaPublicOperationLimits::new(255).unwrap(),
        )
        .unwrap_err()
        .kind(),
        X509OcspRsaSha384SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::ModulusTooLong
        )
    );

    let mut corrupted_signature = fixture_signature();
    *corrupted_signature.last_mut().unwrap() ^= 1;
    let corrupted_encoded = basic_response(
        &signed_response_data(),
        SHA384_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &corrupted_signature,
    );
    let signature_error = verify_x509_ocsp_rsa_pkcs1_v1_5_sha384_signature(
        &decode_response(&corrupted_encoded),
        key,
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        signature_error.kind(),
        X509OcspRsaSha384SignatureErrorKind::EncodedMessage(_)
            | X509OcspRsaSha384SignatureErrorKind::PublicOperation(_)
    ));
    assert!(!signature_error.to_string().contains("2c37793c"));
    assert!(!format!("{signature_error:?}").contains("2c37793c"));
}
