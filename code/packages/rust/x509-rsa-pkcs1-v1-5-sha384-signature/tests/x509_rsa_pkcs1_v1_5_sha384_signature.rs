use der_asn1::Asn1Limits;
use rsa_public_operation::{RsaPublicOperationErrorKind, RsaPublicOperationLimits};
use x509_certificate::{decode_x509_certificate, X509Certificate};
use x509_rsa_pkcs1_v1_5_sha384_signature::{
    verify_x509_rsa_pkcs1_v1_5_sha384_signature, X509RsaSha384SignatureErrorKind,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};

const SHA384_WITH_RSA_ENCRYPTION_OID_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0c];
const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const NULL_DER: &[u8] = &[0x05, 0x00];

// A static OpenSSL-generated 2048-bit self-signed certificate. OpenSSL is not
// invoked by the tests; this DER is only an independent interoperability
// vector for repository-owned parsing, hashing, RSA, and padding code.
const SELF_SIGNED_CERTIFICATE_HEX: &str = concat!(
    "30820310308201f8a003020102020101300d06092a864886f70d01010c05003021311f301d06035504030c167368613338342e666978747572652e6578616d70",
    "6c65301e170d3236313033303032323832355a170d3237313033303032323832355a3021311f301d06035504030c167368613338342e666978747572652e6578",
    "616d706c6530820122300d06092a864886f70d01010105000382010f003082010a0282010100e5c26b4da0d68213228c242721a88219ab5cf64ca8067c27b789",
    "b36abaff76bdb64d57c37c2439f0ae19dd32eb41d280f68ee6d5e0ce816d9e2b385a73c3628b5ca9010b9582b937518c39d0c526a2c01328edd369e04c43ac72",
    "9308654fb519dac3304776f57db33e631343e152217198147708472871a4e25e8ecbbccb82879c47a92de08d03a869f5ed8a08473e596492157edb58ce513dea",
    "bd7721b967ea110bf511227bec9065539754fb7af7463fcd7df73bfd833e35eebc73ca4430077eced3fd162e7724295946087f3f40cf5549c3f9ffc1612d29f5",
    "4d2c3f78e6ad105d457158d476a6fd10722b1b7779768e21f77d5867b8a126c9e444198132b10203010001a3533051301d0603551d0e0416041416e34b32fc1c",
    "2724645c75555724c44dfc5dcf24301f0603551d2304183016801416e34b32fc1c2724645c75555724c44dfc5dcf24300f0603551d130101ff040530030101ff",
    "300d06092a864886f70d01010c05000382010100ba857105c96ad671b513d5c4e59fa68d741b63f751ffbdcbc0040c0859b3972422b693c83e81070abf690d61",
    "a2eba9b15d1e381a9d6af63ae87535e593ea641ccd5bc14fd592fd7152d8422d0de1dd9c3b67c680ac8910e883e17315faa3b9df1c036d62102f98ad5e1a1f06",
    "b76237dbd77af451a250812693f28b3f75b09a074d80fba4fb9c47286c2d7f9432657c3f1db95dfda27a0782ea429e770695033de21031d9a6496830098e89e9",
    "8d0a67d34c72f5cc83363b3ef88a77e18bac0e68d75bd4b3f326010cbc4b82d141ac3176f987bbd621d473aa042b7b5fab05c773c979a71a88154d1a9455bd74",
    "120ba2b39c2a63a36a070bde359f22cad2f46386",
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
    let verified = verify_x509_rsa_pkcs1_v1_5_sha384_signature(
        &certificate,
        key,
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap();

    assert_eq!(format!("{verified:?}"), "VerifiedX509RsaSha384Signature");
    assert!(std::ptr::eq(verified.certificate(), &certificate));
    assert_eq!(verified.issuer_public_key(), key);
}

#[test]
fn rejects_tampered_tbs_and_a_different_validated_key() {
    let encoded = self_signed_fixture();
    let certificate = decode(&encoded);
    let key = fixture_key(&certificate);
    let limits = RsaPublicOperationLimits::new(256).unwrap();

    let mut tampered_tbs = encoded.clone();
    let offset = tampered_tbs
        .windows(b"sha384.fixture.example".len())
        .position(|window| window == b"sha384.fixture.example")
        .unwrap();
    tampered_tbs[offset] ^= 1;
    let tampered_error =
        verify_x509_rsa_pkcs1_v1_5_sha384_signature(&decode(&tampered_tbs), key, limits)
            .unwrap_err();
    assert!(matches!(
        tampered_error.kind(),
        X509RsaSha384SignatureErrorKind::EncodedMessage(_)
    ));

    let mut different_key_der = encoded.clone();
    let modulus_offset = different_key_der
        .windows(key.modulus().len())
        .position(|window| window == key.modulus())
        .unwrap();
    different_key_der[modulus_offset + key.modulus().len() - 1] ^= 2;
    let different_key_certificate = decode(&different_key_der);
    let different_key = fixture_key(&different_key_certificate);
    let wrong_key_error =
        verify_x509_rsa_pkcs1_v1_5_sha384_signature(&certificate, different_key, limits)
            .unwrap_err();
    assert!(matches!(
        wrong_key_error.kind(),
        X509RsaSha384SignatureErrorKind::EncodedMessage(_)
            | X509RsaSha384SignatureErrorKind::PublicOperation(_)
    ));
}

#[test]
fn rejects_unsupported_missing_and_invalid_algorithm_identifiers() {
    let fixture = self_signed_fixture();
    let fixture_certificate = decode(&fixture);
    let key = fixture_key(&fixture_certificate);
    let limits = RsaPublicOperationLimits::new(256).unwrap();

    let unsupported = synthetic_certificate(RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &[0, 0]);
    let missing = synthetic_certificate(SHA384_WITH_RSA_ENCRYPTION_OID_DER, None, &[0, 0]);
    let invalid = synthetic_certificate(
        SHA384_WITH_RSA_ENCRYPTION_OID_DER,
        Some(&[0x04, 0x00]),
        &[0, 0],
    );

    assert_eq!(
        verify_x509_rsa_pkcs1_v1_5_sha384_signature(&decode(&unsupported), key, limits)
            .unwrap_err()
            .kind(),
        X509RsaSha384SignatureErrorKind::UnsupportedSignatureAlgorithm
    );
    assert_eq!(
        verify_x509_rsa_pkcs1_v1_5_sha384_signature(&decode(&missing), key, limits)
            .unwrap_err()
            .kind(),
        X509RsaSha384SignatureErrorKind::MissingSignatureAlgorithmParameters
    );
    let invalid_error =
        verify_x509_rsa_pkcs1_v1_5_sha384_signature(&decode(&invalid), key, limits).unwrap_err();
    assert_eq!(
        invalid_error.kind(),
        X509RsaSha384SignatureErrorKind::InvalidSignatureAlgorithmParameters
    );
    assert!(!invalid_error.to_string().contains("fixture"));
}

#[test]
fn rejects_non_byte_aligned_or_wrong_width_signatures() {
    let fixture = self_signed_fixture();
    let fixture_certificate = decode(&fixture);
    let key = fixture_key(&fixture_certificate);
    let limits = RsaPublicOperationLimits::new(256).unwrap();

    let non_byte_aligned =
        synthetic_certificate(SHA384_WITH_RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &[1, 0]);
    assert_eq!(
        verify_x509_rsa_pkcs1_v1_5_sha384_signature(&decode(&non_byte_aligned), key, limits)
            .unwrap_err()
            .kind(),
        X509RsaSha384SignatureErrorKind::NonByteAlignedSignature
    );

    let wrong_width =
        synthetic_certificate(SHA384_WITH_RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &[0, 0]);
    assert_eq!(
        verify_x509_rsa_pkcs1_v1_5_sha384_signature(&decode(&wrong_width), key, limits)
            .unwrap_err()
            .kind(),
        X509RsaSha384SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::SignatureLengthMismatch
        )
    );
}

#[test]
fn maps_limits_and_corrupted_signatures_to_redacted_categories() {
    let mut encoded = self_signed_fixture();
    let certificate = decode(&encoded);
    let key = fixture_key(&certificate);
    assert_eq!(
        verify_x509_rsa_pkcs1_v1_5_sha384_signature(
            &certificate,
            key,
            RsaPublicOperationLimits::new(255).unwrap(),
        )
        .unwrap_err()
        .kind(),
        X509RsaSha384SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::ModulusTooLong
        )
    );

    *encoded.last_mut().unwrap() ^= 1;
    let corrupted_certificate = decode(&encoded);
    let corrupted_key = fixture_key(&corrupted_certificate);
    let signature_error = verify_x509_rsa_pkcs1_v1_5_sha384_signature(
        &corrupted_certificate,
        corrupted_key,
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        signature_error.kind(),
        X509RsaSha384SignatureErrorKind::EncodedMessage(_)
    ));
    assert!(!signature_error.to_string().contains("ba857105"));
}
