use der_asn1::Asn1Limits;
use rsa_public_operation::{RsaPublicOperationErrorKind, RsaPublicOperationLimits};
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_rsa_pkcs1_v1_5_sha256_signature::{
    verify_x509_rsa_pkcs1_v1_5_sha256_signature, X509RsaSha256SignatureErrorKind,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};

const SHA256_WITH_RSA_ENCRYPTION_OID_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const NULL_DER: &[u8] = &[0x05, 0x00];

// A static OpenSSL-generated 2048-bit self-signed certificate. OpenSSL is not
// invoked by the tests; this DER is only an independent interoperability
// vector for repository-owned parsing, hashing, RSA, and padding code.
const SELF_SIGNED_CERTIFICATE_HEX: &str = concat!(
    "30820302308201eaa003020102020101300d06092a864886f70d01010b0500301a",
    "3118301606035504030c0f666978747572652e6578616d706c65301e170d3236",
    "313031393133323232375a170d3237313031393133323232375a301a31183016",
    "06035504030c0f666978747572652e6578616d706c6530820122300d06092a86",
    "4886f70d01010105000382010f003082010a0282010100a29d015dfef93f6242",
    "eb379927fe740c0f8f318d3d650062255a13be0d22e7ba7561ffd136b0bffc83",
    "aaadfd2e14ac74f83a62b2eeec4ce0c7fab8c043c1b1ccbf382047abfc16f3",
    "e652526f99772e7da7dce76e09ec43c35f5a2dacfb860c90626bbe9704d833c1",
    "14d9390562ff877f5c307389736d0645729581ed5fe51edeb3843c939ff0fe75",
    "f1452574110663e720792445218228d6acae1a96211717d28118826ebe1147021",
    "b2587b8dbec595270418b92802c88a1e6c8b23e724de8ddd7c6dc7efd475608",
    "d6cc1dc5b7ee3769c06243107352c0c2a4bab1adfb93e1abc98158e5c35a5e9",
    "8b7456dae5bd5289a296748aae03c407b205e0501eb4e6bfb0203010001a35330",
    "51301d0603551d0e041604146b0f758af837ee8260699f6eb28dc29885547f96",
    "301f0603551d230418301680146b0f758af837ee8260699f6eb28dc29885547f",
    "96300f0603551d130101ff040530030101ff300d06092a864886f70d01010b05",
    "0003820101003639dec06f08db64836993dc7ea4ab5ae00318b4d4e07bd30644",
    "97f48e087b6408e935a060dc312cdfad1ae6a01843aafaedaea219ddad6e3102",
    "ee9a9c7f9d1d7d8c7ea49d624b77adc4c05b71901d2c2c513a24f583ef091d6",
    "2eb62af8326ef5b25dfd3733b673fc6387556d232154d16f2e2610dc7383e568",
    "d2e81326f7d5728e40ceac1b82a90e1e53e31416c8b2aa2b77b801142b653a",
    "c67fb3b3bfe97f561b91e59659ff20a43686a6ac33bdeba8ef573c2d443b24e",
    "4bd5b2e995322dc6c4b2443d7bca9716e374c217e3c06d94109a781455e225a",
    "66ba6b5e4ccac7f57c21fbeab6780f8791abaa8ff371aeaadb9181bb7b7fb459",
    "7411a5a92e329e8df",
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
    let encoded = &bytes[first..];
    let mut result = vec![0x80 | encoded.len() as u8];
    result.extend_from_slice(encoded);
    result
}

fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = vec![tag];
    encoded.extend(der_length(value.len()));
    encoded.extend_from_slice(value);
    encoded
}

fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
    tlv(0x30, &fields.concat())
}

fn algorithm(oid: &[u8], parameters: Option<&[u8]>) -> Vec<u8> {
    let mut fields = tlv(0x06, oid);
    if let Some(parameters) = parameters {
        fields.extend_from_slice(parameters);
    }
    tlv(0x30, &fields)
}

fn name() -> Vec<u8> {
    let mut attribute = tlv(0x06, &[0x55, 0x04, 0x03]);
    attribute.extend(tlv(0x0c, b"fixture"));
    sequence(&[tlv(0x31, &sequence(&[attribute]))])
}

fn synthetic_certificate(
    signature_oid: &[u8],
    signature_parameters: Option<&[u8]>,
    signature_bit_string: &[u8],
) -> Vec<u8> {
    let signature_algorithm = algorithm(signature_oid, signature_parameters);
    let validity = sequence(&[tlv(0x17, b"260101000000Z"), tlv(0x17, b"270101000000Z")]);
    let subject_public_key_info = sequence(&[
        algorithm(RSA_ENCRYPTION_OID_DER, Some(NULL_DER)),
        tlv(0x03, &[0, 1]),
    ]);
    let tbs = sequence(&[
        tlv(0x02, &[1]),
        signature_algorithm.clone(),
        name(),
        validity,
        name(),
        subject_public_key_info,
    ]);
    sequence(&[tbs, signature_algorithm, tlv(0x03, signature_bit_string)])
}

fn decode(input: &[u8]) -> X509Certificate<'_> {
    decode_x509_certificate(input, Asn1Limits::default()).unwrap()
}

fn self_signed_fixture() -> Vec<u8> {
    decode_hex(SELF_SIGNED_CERTIFICATE_HEX)
}

fn fixture_key<'a>(certificate: &'a X509Certificate<'a>) -> ValidatedRsaPublicKey<'a> {
    let policy = RsaPublicKeyPolicy::new(2_048, 2_048, &[65_537]).unwrap();
    validate_rsa_public_key(
        certificate.tbs_certificate().subject_public_key_info(),
        policy,
        Asn1Limits::default(),
    )
    .unwrap()
}

#[test]
fn verifies_independent_2048_bit_self_signed_fixture() {
    let encoded = self_signed_fixture();
    let certificate = decode(&encoded);
    let key = fixture_key(&certificate);
    let verified = verify_x509_rsa_pkcs1_v1_5_sha256_signature(
        &certificate,
        key,
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap();

    assert_eq!(format!("{verified:?}"), "VerifiedX509RsaSha256Signature");
    assert!(std::ptr::eq(verified.certificate(), &certificate));
    assert_eq!(verified.certificate().encoded(), certificate.encoded());
    assert_eq!(verified.issuer_public_key(), key);
}

#[test]
fn rejects_tampered_tbs_and_a_different_validated_key() {
    let encoded = self_signed_fixture();
    let certificate = decode(&encoded);
    let key = fixture_key(&certificate);
    let limits = RsaPublicOperationLimits::new(256).unwrap();

    let mut tampered_tbs = encoded.clone();
    let common_name_offset = tampered_tbs
        .windows(b"fixture.example".len())
        .position(|window| window == b"fixture.example")
        .unwrap();
    tampered_tbs[common_name_offset] ^= 1;
    let tampered_certificate = decode(&tampered_tbs);
    let tampered_error =
        verify_x509_rsa_pkcs1_v1_5_sha256_signature(&tampered_certificate, key, limits)
            .unwrap_err();
    assert!(matches!(
        tampered_error.kind(),
        X509RsaSha256SignatureErrorKind::EncodedMessage(_)
    ));

    let mut different_key_certificate_der = encoded.clone();
    let modulus_offset = different_key_certificate_der
        .windows(key.modulus().len())
        .position(|window| window == key.modulus())
        .unwrap();
    let modulus_last = modulus_offset + key.modulus().len() - 1;
    different_key_certificate_der[modulus_last] ^= 2;
    let different_key_certificate = decode(&different_key_certificate_der);
    let different_key = fixture_key(&different_key_certificate);
    assert_ne!(different_key, key);

    let wrong_key_error =
        verify_x509_rsa_pkcs1_v1_5_sha256_signature(&certificate, different_key, limits)
            .unwrap_err();
    assert!(matches!(
        wrong_key_error.kind(),
        X509RsaSha256SignatureErrorKind::EncodedMessage(_)
            | X509RsaSha256SignatureErrorKind::PublicOperation(_)
    ));
}

#[test]
fn rejects_unsupported_missing_and_invalid_algorithm_identifiers() {
    let fixture = self_signed_fixture();
    let fixture_certificate = decode(&fixture);
    let key = fixture_key(&fixture_certificate);
    let limits = RsaPublicOperationLimits::new(256).unwrap();

    let unsupported = synthetic_certificate(RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &[0, 0]);
    let missing = synthetic_certificate(SHA256_WITH_RSA_ENCRYPTION_OID_DER, None, &[0, 0]);
    let invalid = synthetic_certificate(
        SHA256_WITH_RSA_ENCRYPTION_OID_DER,
        Some(&[0x04, 0x00]),
        &[0, 0],
    );

    let unsupported_error =
        verify_x509_rsa_pkcs1_v1_5_sha256_signature(&decode(&unsupported), key, limits)
            .unwrap_err();
    assert_eq!(
        unsupported_error.kind(),
        X509RsaSha256SignatureErrorKind::UnsupportedSignatureAlgorithm
    );

    let missing_error =
        verify_x509_rsa_pkcs1_v1_5_sha256_signature(&decode(&missing), key, limits).unwrap_err();
    assert_eq!(
        missing_error.kind(),
        X509RsaSha256SignatureErrorKind::MissingSignatureAlgorithmParameters
    );

    let invalid_error =
        verify_x509_rsa_pkcs1_v1_5_sha256_signature(&decode(&invalid), key, limits).unwrap_err();
    assert!(matches!(
        invalid_error.kind(),
        X509RsaSha256SignatureErrorKind::InvalidSignatureAlgorithmParameters
    ));
    assert!(!invalid_error.to_string().contains("fixture"));
}

#[test]
fn rejects_non_byte_aligned_or_wrong_width_signatures_before_hash_acceptance() {
    let fixture = self_signed_fixture();
    let fixture_certificate = decode(&fixture);
    let key = fixture_key(&fixture_certificate);
    let limits = RsaPublicOperationLimits::new(256).unwrap();

    let non_byte_aligned =
        synthetic_certificate(SHA256_WITH_RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &[1, 0]);
    let alignment_error =
        verify_x509_rsa_pkcs1_v1_5_sha256_signature(&decode(&non_byte_aligned), key, limits)
            .unwrap_err();
    assert_eq!(
        alignment_error.kind(),
        X509RsaSha256SignatureErrorKind::NonByteAlignedSignature
    );

    let wrong_width =
        synthetic_certificate(SHA256_WITH_RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &[0, 0]);
    let width_error =
        verify_x509_rsa_pkcs1_v1_5_sha256_signature(&decode(&wrong_width), key, limits)
            .unwrap_err();
    assert_eq!(
        width_error.kind(),
        X509RsaSha256SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::SignatureLengthMismatch
        )
    );
}

#[test]
fn maps_operational_limits_and_corrupted_signatures_to_redacted_categories() {
    let mut encoded = self_signed_fixture();
    let certificate = decode(&encoded);
    let key = fixture_key(&certificate);
    let limit_error = verify_x509_rsa_pkcs1_v1_5_sha256_signature(
        &certificate,
        key,
        RsaPublicOperationLimits::new(255).unwrap(),
    )
    .unwrap_err();
    assert_eq!(
        limit_error.kind(),
        X509RsaSha256SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::ModulusTooLong
        )
    );

    *encoded.last_mut().unwrap() ^= 1;
    let corrupted_certificate = decode(&encoded);
    let corrupted_key = fixture_key(&corrupted_certificate);
    let signature_error = verify_x509_rsa_pkcs1_v1_5_sha256_signature(
        &corrupted_certificate,
        corrupted_key,
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        signature_error.kind(),
        X509RsaSha256SignatureErrorKind::EncodedMessage(_)
    ));
    assert!(!signature_error.to_string().contains("3639dec0"));
}
