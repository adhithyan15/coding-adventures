use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_public_operation::{RsaPublicOperationErrorKind, RsaPublicOperationLimits};
use sha512::sum512;
use x509_ocsp_basic_response::{decode_x509_ocsp_basic_response, X509OcspBasicResponse};
use x509_ocsp_rsa_pkcs1_v1_5_sha512_signature::{
    verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature, X509OcspRsaSha512SignatureErrorKind,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const SHA512_WITH_RSA_ENCRYPTION_OID_DER: &[u8] =
    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0d];
const RSA_ENCRYPTION_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const NULL_DER: &[u8] = &[0x05, 0x00];

// OpenSSL 3.6.3 generated this one-off interoperability vector from synthetic
// public OCSP data. Its temporary private key was deleted immediately after
// fixture extraction and is not retained. Re-hashing the checked-in bytes
// below binds them back to the generation trace:
//
// ResponseData SHA-512: 5c5a42147a8620f100254f0d3ee07c504715ef96d1276ef3c7c32000d387100732731d941e1b115389b0a5b909689136f4c5ed9ae2257749c45e77b7f9e984a8
// public SPKI SHA-512: b289bb6dda716a0af1ef37337bd5b805640cc0cd3530fd9b6abf46eb7f599f1c13338946e940fc875d79253b7be1247390d4f410621e327b47e75b0b0594c7ba
// signature SHA-512: 8b4de01c6e1c81c2bc73465a16519d09733180b937a25c80216610fadfbaf3a993f833bdd64beb83ea0ab382bff5565b76ccdca10c492ff4d884af3b3af3b781
const RESPONSE_DATA_HEX: &str = concat!(
    "307ca21604144444444444444444444444444444444444444444180f32303236",
    "313032343132303030305a3051304f303a300906052b0e03021a050004141111",
    "1111111111111111111111111111111111110414222222222222222222222222",
    "22222222222222220201018000180f32303236313032343132333435365a",
);
const PUBLIC_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a02820101",
    "00bf3b7ec4399b92c6cc41bb49a5962a6a439823ccf6d95066624b5e674b5e",
    "324d48407ecfc8cbebf714c0267370569c1a4f4b3c74e454519be2244e4626",
    "895bae3dc2d3c8a8f5b0924717a7de76727750cc2f9437244e76562a385e0c",
    "36bbaf772d506b9b89c86c05f472c0dca7165acc9ba2a6d913618ce62a9711",
    "0b2277db926d385892820f7d852643b583851a93a153d9fbc3fd892d13f5cd",
    "6a1e80811afadff990f7f929b6deef83b37ea46784446165cc8f7e30b530c4",
    "77e986cffb93a7cfcbe0753f7a7bb5ae8470c91336f8e314a10a5348a0a496",
    "acd8ce97404feee5461f5c6d95ec4f779070da2576fce22daffdd90576d152",
    "59308a829d97179fc30203010001",
);
const SIGNATURE_HEX: &str = concat!(
    "4f45613e4185e22339af206eaf888d233ef899669f355d34918ba3b33e99dba8",
    "ff86f4a0a90101ae6586d18ed1c67f10245e7941b939d51c876b9341b1c646a5",
    "d3dd02bae3742f5370e2bb006836eb5541513451f97454e9fff4c73b756defba",
    "a16b5262a03c2f3c850623b35d6c6a6ccfdd0fe358634c376c84ff51445252a0",
    "15b6cd8fa18742c4d6e148a49f697f98fd34f1767b1b2b6e0ba9d16fdd9a7a43",
    "6511c2ae5f9a9770b724ca1b952191413de8bc9b484581c4ab2ac163e4335f8d",
    "69f758773a98a433d45389a4de1a694eda2ad1569604035e35c0961b33bf40e2",
    "b2f012a1c12797ef549b048317bacf91b69aca6244c1da3eda0b26ee149a4d38",
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
        SHA512_WITH_RSA_ENCRYPTION_OID_DER,
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
        sum512(&signed_response_data()).as_slice(),
        decode_hex(concat!(
            "5c5a42147a8620f100254f0d3ee07c504715ef96d1276ef3c7c32000d3871007",
            "32731d941e1b115389b0a5b909689136f4c5ed9ae2257749c45e77b7f9e984a8",
        ))
    );
    assert_eq!(
        sum512(&decode_hex(PUBLIC_SPKI_HEX)).as_slice(),
        decode_hex(concat!(
            "b289bb6dda716a0af1ef37337bd5b805640cc0cd3530fd9b6abf46eb7f599f1c",
            "13338946e940fc875d79253b7be1247390d4f410621e327b47e75b0b0594c7ba",
        ))
    );
    assert_eq!(
        sum512(&decode_hex(SIGNATURE_HEX)).as_slice(),
        decode_hex(concat!(
            "8b4de01c6e1c81c2bc73465a16519d09733180b937a25c80216610fadfbaf3a9",
            "93f833bdd64beb83ea0ab382bff5565b76ccdca10c492ff4d884af3b3af3b781",
        ))
    );
}

#[test]
fn verifies_independent_2048_bit_ocsp_fixture_and_binds_exact_inputs() {
    let encoded = fixture_response();
    let response = decode_response(&encoded);
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let verified = verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(
        &response,
        key,
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap();

    assert_eq!(
        format!("{verified:?}"),
        "VerifiedX509OcspRsaSha512Signature"
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
        SHA512_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &fixture_signature(),
    );
    let tampered = decode_response(&tampered_encoded);
    let tampered_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(&tampered, key, limits).unwrap_err();
    assert!(matches!(
        tampered_error.kind(),
        X509OcspRsaSha512SignatureErrorKind::EncodedMessage(_)
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
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(&response, different_key, limits)
            .unwrap_err();
    assert!(matches!(
        wrong_key_error.kind(),
        X509OcspRsaSha512SignatureErrorKind::EncodedMessage(_)
            | X509OcspRsaSha512SignatureErrorKind::PublicOperation(_)
    ));
}

#[test]
fn rejects_unsupported_missing_and_invalid_algorithm_identifiers() {
    let spki = decode_hex(PUBLIC_SPKI_HEX);
    let key = validated_key(&spki);
    let limits = RsaPublicOperationLimits::new(256).unwrap();
    let data = signed_response_data();

    let unsupported = basic_response(&data, RSA_ENCRYPTION_OID_DER, Some(NULL_DER), &[0, 0]);
    let missing = basic_response(&data, SHA512_WITH_RSA_ENCRYPTION_OID_DER, None, &[0, 0]);
    let invalid = basic_response(
        &data,
        SHA512_WITH_RSA_ENCRYPTION_OID_DER,
        Some(&[0x04, 0x00]),
        &[0, 0],
    );

    let unsupported_response = decode_response(&unsupported);
    let unsupported_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(&unsupported_response, key, limits)
            .unwrap_err();
    assert_eq!(
        unsupported_error.kind(),
        X509OcspRsaSha512SignatureErrorKind::UnsupportedSignatureAlgorithm
    );

    let missing_response = decode_response(&missing);
    let missing_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(&missing_response, key, limits)
            .unwrap_err();
    assert_eq!(
        missing_error.kind(),
        X509OcspRsaSha512SignatureErrorKind::MissingSignatureAlgorithmParameters
    );

    let invalid_response = decode_response(&invalid);
    let invalid_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(&invalid_response, key, limits)
            .unwrap_err();
    assert_eq!(
        invalid_error.kind(),
        X509OcspRsaSha512SignatureErrorKind::InvalidSignatureAlgorithmParameters
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
        SHA512_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &[1, 0],
    );
    let alignment_response = decode_response(&non_byte_aligned);
    let alignment_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(&alignment_response, key, limits)
            .unwrap_err();
    assert_eq!(
        alignment_error.kind(),
        X509OcspRsaSha512SignatureErrorKind::NonByteAlignedSignature
    );

    let wrong_width = basic_response(
        &data,
        SHA512_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &[0, 0],
    );
    let width_response = decode_response(&wrong_width);
    let width_error =
        verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(&width_response, key, limits).unwrap_err();
    assert_eq!(
        width_error.kind(),
        X509OcspRsaSha512SignatureErrorKind::PublicOperation(
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
    let limit_error = verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(
        &response,
        key,
        RsaPublicOperationLimits::new(255).unwrap(),
    )
    .unwrap_err();
    assert_eq!(
        limit_error.kind(),
        X509OcspRsaSha512SignatureErrorKind::PublicOperation(
            RsaPublicOperationErrorKind::ModulusTooLong
        )
    );

    let mut corrupted_signature = fixture_signature();
    *corrupted_signature.last_mut().unwrap() ^= 1;
    let corrupted_encoded = basic_response(
        &signed_response_data(),
        SHA512_WITH_RSA_ENCRYPTION_OID_DER,
        Some(NULL_DER),
        &corrupted_signature,
    );
    let corrupted = decode_response(&corrupted_encoded);
    let signature_error = verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(
        &corrupted,
        key,
        RsaPublicOperationLimits::new(256).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        signature_error.kind(),
        X509OcspRsaSha512SignatureErrorKind::EncodedMessage(_)
            | X509OcspRsaSha512SignatureErrorKind::PublicOperation(_)
    ));
    assert!(!signature_error.to_string().contains("4f45613e"));
    assert!(!format!("{signature_error:?}").contains("4f45613e"));
}
