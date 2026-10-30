use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_algorithm_identifier::decode_x509_algorithm_identifier;
use x509_rsassa_pss_parameters::{
    decode_x509_rsassa_pss_parameters, X509RsassaPssDigestAlgorithm, X509RsassaPssParametersError,
    X509RsassaPssParametersErrorKind,
};

const PSS_OID: &[u8] = &[
    0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0a,
];
const MGF1_OID: &[u8] = &[
    0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x08,
];
const SHA1_OID: &[u8] = &[0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a];
const SHA256_OID: &[u8] = &[
    0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01,
];
const SHA384_OID: &[u8] = &[
    0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02,
];
const SHA224_OID: &[u8] = &[
    0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x04,
];
const SHA512_OID: &[u8] = &[
    0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03,
];

fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    assert!(value.len() < 128);
    let mut encoded = vec![tag, value.len() as u8];
    encoded.extend_from_slice(value);
    encoded
}

fn sequence(parts: &[&[u8]]) -> Vec<u8> {
    let value: Vec<u8> = parts.iter().flat_map(|part| part.iter().copied()).collect();
    tlv(0x30, &value)
}

fn hash_identifier(oid: &[u8], with_null: bool) -> Vec<u8> {
    if with_null {
        sequence(&[oid, &[0x05, 0x00]])
    } else {
        sequence(&[oid])
    }
}

fn explicit(tag: u8, child: &[u8]) -> Vec<u8> {
    tlv(0xa0 + tag, child)
}

fn integer(value: u8) -> Vec<u8> {
    vec![0x02, 0x01, value]
}

fn pss_identifier(fields: &[&[u8]]) -> Vec<u8> {
    let parameters = sequence(fields);
    sequence(&[PSS_OID, &parameters])
}

fn decode(
    encoded: &[u8],
) -> Result<x509_rsassa_pss_parameters::X509RsassaPssParameters, X509RsassaPssParametersErrorKind> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    let algorithm = decode_x509_algorithm_identifier(&mut decoder, root).unwrap();
    decode_x509_rsassa_pss_parameters(&mut decoder, algorithm).map_err(|error| error.kind())
}

fn decode_error(encoded: &[u8]) -> X509RsassaPssParametersError {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    let algorithm = decode_x509_algorithm_identifier(&mut decoder, root).unwrap();
    decode_x509_rsassa_pss_parameters(&mut decoder, algorithm).unwrap_err()
}

#[test]
fn decodes_empty_default_parameters() {
    let decoded = decode(&pss_identifier(&[])).unwrap();
    assert_eq!(decoded.message_hash(), X509RsassaPssDigestAlgorithm::Sha1);
    assert_eq!(decoded.mgf1_hash(), X509RsassaPssDigestAlgorithm::Sha1);
    assert_eq!(decoded.salt_length(), 20);
    assert_eq!(decoded.trailer_field(), 1);
}

#[test]
fn decodes_explicit_sha256_mgf1_sha384_and_salt() {
    let hash = explicit(0, &hash_identifier(SHA256_OID, true));
    let mgf_hash = hash_identifier(SHA384_OID, false);
    let mgf = explicit(1, &sequence(&[MGF1_OID, &mgf_hash]));
    let salt = explicit(2, &integer(32));
    let decoded = decode(&pss_identifier(&[&hash, &mgf, &salt])).unwrap();
    assert_eq!(decoded.message_hash(), X509RsassaPssDigestAlgorithm::Sha256);
    assert_eq!(decoded.mgf1_hash(), X509RsassaPssDigestAlgorithm::Sha384);
    assert_eq!(decoded.salt_length(), 32);
    assert_eq!(decoded.trailer_field(), 1);
}

#[test]
fn accepts_rfc4055_explicit_validator_defaults_but_rejects_default_salt() {
    let hash = explicit(0, &hash_identifier(SHA1_OID, true));
    let decoded = decode(&pss_identifier(&[&hash])).unwrap();
    assert_eq!(decoded.message_hash(), X509RsassaPssDigestAlgorithm::Sha1);

    let mgf_hash = hash_identifier(SHA1_OID, true);
    let mgf = explicit(1, &sequence(&[MGF1_OID, &mgf_hash]));
    let decoded = decode(&pss_identifier(&[&mgf])).unwrap();
    assert_eq!(decoded.mgf1_hash(), X509RsassaPssDigestAlgorithm::Sha1);

    let trailer = explicit(3, &integer(1));
    let decoded = decode(&pss_identifier(&[&trailer])).unwrap();
    assert_eq!(decoded.trailer_field(), 1);

    let salt = explicit(2, &integer(20));
    assert_eq!(
        decode(&pss_identifier(&[&salt])).unwrap_err(),
        X509RsassaPssParametersErrorKind::ExplicitDefaultSaltLength
    );
}

#[test]
fn rejects_duplicate_out_of_order_and_unknown_fields() {
    let salt = explicit(2, &integer(32));
    assert_eq!(
        decode(&pss_identifier(&[&salt, &salt])).unwrap_err(),
        X509RsassaPssParametersErrorKind::DuplicateParameter
    );
    let hash = explicit(0, &hash_identifier(SHA256_OID, true));
    assert_eq!(
        decode(&pss_identifier(&[&salt, &hash])).unwrap_err(),
        X509RsassaPssParametersErrorKind::OutOfOrderParameter
    );
    let unknown = explicit(4, &integer(2));
    assert_eq!(
        decode(&pss_identifier(&[&unknown])).unwrap_err(),
        X509RsassaPssParametersErrorKind::UnexpectedParameterTag
    );
}

#[test]
fn rejects_unsupported_hash_and_mask_algorithms() {
    let unknown_hash_oid = [0x06, 0x02, 0x2a, 0x03];
    let hash = explicit(0, &hash_identifier(&unknown_hash_oid, true));
    assert_eq!(
        decode(&pss_identifier(&[&hash])).unwrap_err(),
        X509RsassaPssParametersErrorKind::UnsupportedHashAlgorithm
    );

    let mgf_hash = hash_identifier(SHA256_OID, true);
    let wrong_mgf_oid = [0x06, 0x02, 0x2a, 0x03];
    let mgf = explicit(1, &sequence(&[&wrong_mgf_oid, &mgf_hash]));
    assert_eq!(
        decode(&pss_identifier(&[&mgf])).unwrap_err(),
        X509RsassaPssParametersErrorKind::UnsupportedMaskGenAlgorithm
    );
}

#[test]
fn decodes_remaining_digest_ids_and_nondefault_trailer() {
    let hash = explicit(0, &hash_identifier(SHA224_OID, false));
    let mgf_hash = hash_identifier(SHA512_OID, true);
    let mgf = explicit(1, &sequence(&[MGF1_OID, &mgf_hash]));
    let trailer = explicit(3, &integer(2));
    let decoded = decode(&pss_identifier(&[&hash, &mgf, &trailer])).unwrap();
    assert_eq!(decoded.message_hash(), X509RsassaPssDigestAlgorithm::Sha224);
    assert_eq!(decoded.mgf1_hash(), X509RsassaPssDigestAlgorithm::Sha512);
    assert_eq!(decoded.trailer_field(), 2);
}

#[test]
fn rejects_wrong_outer_algorithm_missing_or_nonsequence_parameters() {
    let wrong = sequence(&[SHA256_OID]);
    assert_eq!(
        decode(&wrong).unwrap_err(),
        X509RsassaPssParametersErrorKind::WrongAlgorithm
    );

    let missing = sequence(&[PSS_OID]);
    assert_eq!(
        decode(&missing).unwrap_err(),
        X509RsassaPssParametersErrorKind::MissingParameters
    );

    let nonsequence = sequence(&[PSS_OID, &[0x05, 0x00]]);
    assert!(matches!(
        decode(&nonsequence).unwrap_err(),
        X509RsassaPssParametersErrorKind::Structure(_)
    ));
}

#[test]
fn rejects_malformed_hash_and_mgf_identifiers() {
    let invalid_hash = explicit(0, &integer(2));
    assert!(matches!(
        decode(&pss_identifier(&[&invalid_hash])).unwrap_err(),
        X509RsassaPssParametersErrorKind::InvalidHashAlgorithmIdentifier(_)
    ));

    let hash_bad_parameters = explicit(0, &sequence(&[SHA256_OID, &integer(2)]));
    assert!(matches!(
        decode(&pss_identifier(&[&hash_bad_parameters])).unwrap_err(),
        X509RsassaPssParametersErrorKind::InvalidHashAlgorithmParameters(_)
    ));

    let invalid_mgf = explicit(1, &integer(2));
    assert!(matches!(
        decode(&pss_identifier(&[&invalid_mgf])).unwrap_err(),
        X509RsassaPssParametersErrorKind::InvalidMaskGenAlgorithmIdentifier(_)
    ));

    let missing_hash = explicit(1, &sequence(&[MGF1_OID]));
    assert_eq!(
        decode(&pss_identifier(&[&missing_hash])).unwrap_err(),
        X509RsassaPssParametersErrorKind::MissingMaskGenHashAlgorithm
    );
}

#[test]
fn rejects_malformed_mgf_hash_and_integer_fields() {
    let invalid_nested = explicit(1, &sequence(&[MGF1_OID, &integer(2)]));
    assert!(matches!(
        decode(&pss_identifier(&[&invalid_nested])).unwrap_err(),
        X509RsassaPssParametersErrorKind::InvalidMaskGenHashAlgorithmIdentifier(_)
    ));

    let unknown_hash_oid = [0x06, 0x02, 0x2a, 0x03];
    let unknown_hash = sequence(&[&unknown_hash_oid]);
    let unsupported_nested = explicit(1, &sequence(&[MGF1_OID, &unknown_hash]));
    assert_eq!(
        decode(&pss_identifier(&[&unsupported_nested])).unwrap_err(),
        X509RsassaPssParametersErrorKind::UnsupportedMaskGenHashAlgorithm
    );

    let bad_hash_parameters = sequence(&[SHA256_OID, &integer(2)]);
    let invalid_parameters = explicit(1, &sequence(&[MGF1_OID, &bad_hash_parameters]));
    assert!(matches!(
        decode(&pss_identifier(&[&invalid_parameters])).unwrap_err(),
        X509RsassaPssParametersErrorKind::InvalidMaskGenHashParameters(_)
    ));

    let invalid_salt = explicit(2, &[0x05, 0x00]);
    assert!(matches!(
        decode(&pss_identifier(&[&invalid_salt])).unwrap_err(),
        X509RsassaPssParametersErrorKind::InvalidSaltLength(_)
    ));
    let invalid_trailer = explicit(3, &[0x05, 0x00]);
    assert!(matches!(
        decode(&pss_identifier(&[&invalid_trailer])).unwrap_err(),
        X509RsassaPssParametersErrorKind::InvalidTrailerField(_)
    ));
}

#[test]
fn reports_offsets_from_the_parameters_sequence_through_nested_wrappers() {
    let invalid_hash = explicit(0, &integer(2));
    assert_eq!(decode_error(&pss_identifier(&[&invalid_hash])).offset(), 4);

    let invalid_mgf = explicit(1, &integer(2));
    assert_eq!(decode_error(&pss_identifier(&[&invalid_mgf])).offset(), 4);

    let invalid_nested = explicit(1, &sequence(&[MGF1_OID, &integer(2)]));
    assert_eq!(
        decode_error(&pss_identifier(&[&invalid_nested])).offset(),
        17
    );

    let invalid_salt = explicit(2, &[0x05, 0x00]);
    assert_eq!(decode_error(&pss_identifier(&[&invalid_salt])).offset(), 4);

    let invalid_trailer = explicit(3, &[0x05, 0x00]);
    assert_eq!(
        decode_error(&pss_identifier(&[&invalid_trailer])).offset(),
        4
    );

    let truncated_explicit_child = explicit(0, &[0x30]);
    assert_eq!(
        decode_error(&pss_identifier(&[&truncated_explicit_child])).offset(),
        5
    );
}

#[test]
fn errors_are_redacted() {
    let salt = explicit(2, &integer(20));
    let encoded = pss_identifier(&[&salt]);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(&encoded).unwrap();
    let algorithm = decode_x509_algorithm_identifier(&mut decoder, root).unwrap();
    let error = decode_x509_rsassa_pss_parameters(&mut decoder, algorithm).unwrap_err();
    assert_eq!(
        error.kind(),
        X509RsassaPssParametersErrorKind::ExplicitDefaultSaltLength
    );
    assert!(error.offset() > 0);
    assert!(!format!("{error:?}").contains("140114"));
    assert!(!error.to_string().contains("140114"));
}
