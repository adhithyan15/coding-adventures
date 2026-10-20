use der_asn1::Asn1Limits;
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{
    validate_certificate_algorithm_profile, X509AlgorithmParametersPolicy,
    X509CertificateAlgorithmErrorKind, X509CertificateAlgorithmProfile,
    X509CertificateAlgorithmProfileDefinitionErrorKind,
};

const SIGNATURE_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const SIGNATURE_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
const KEY_OID_DER: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const KEY_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
const OTHER_OID_DER: &[u8] = &[0x2a, 0x03];
const NULL_DER: &[u8] = &[0x05, 0x00];

fn der(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(value.len() + 4);
    encoded.push(tag);
    match value.len() {
        length @ 0..=127 => encoded.push(length as u8),
        length @ 128..=255 => encoded.extend([0x81, length as u8]),
        length @ 256..=65_535 => {
            encoded.extend([0x82, (length >> 8) as u8, length as u8]);
        }
        _ => panic!("test value too large"),
    }
    encoded.extend(value);
    encoded
}

fn algorithm(oid: &[u8], parameters: Option<&[u8]>) -> Vec<u8> {
    let mut fields = der(0x06, oid);
    if let Some(parameters) = parameters {
        fields.extend(parameters);
    }
    der(0x30, &fields)
}

fn name() -> Vec<u8> {
    let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
    attribute.extend(der(0x0c, b"example"));
    der(0x30, &der(0x31, &der(0x30, &attribute)))
}

fn certificate(
    signature_oid: &[u8],
    signature_parameters: Option<&[u8]>,
    key_oid: &[u8],
    key_parameters: Option<&[u8]>,
    key_bit_string: &[u8],
    signature_bit_string: &[u8],
) -> Vec<u8> {
    let mut validity = der(0x17, b"260101000000Z");
    validity.extend(der(0x17, b"270101000000Z"));

    let mut subject_public_key_info = algorithm(key_oid, key_parameters);
    subject_public_key_info.extend(der(0x03, key_bit_string));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[0x01]));
    tbs.extend(algorithm(signature_oid, signature_parameters));
    tbs.extend(name());
    tbs.extend(der(0x30, &validity));
    tbs.extend(name());
    tbs.extend(der(0x30, &subject_public_key_info));

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(signature_oid, signature_parameters));
    fields.extend(der(0x03, signature_bit_string));
    der(0x30, &fields)
}

fn profile(
    signature_parameters: X509AlgorithmParametersPolicy<'static>,
    key_parameters: X509AlgorithmParametersPolicy<'static>,
    key_min_bits: usize,
    key_max_bits: usize,
    signature_min_bits: usize,
    signature_max_bits: usize,
) -> X509CertificateAlgorithmProfile<'static> {
    X509CertificateAlgorithmProfile::new(
        SIGNATURE_OID,
        signature_parameters,
        KEY_OID,
        key_parameters,
        key_min_bits,
        key_max_bits,
        signature_min_bits,
        signature_max_bits,
    )
    .unwrap()
}

#[test]
fn rejects_invalid_profile_definitions() {
    let cases = [
        (
            X509CertificateAlgorithmProfile::new(
                &[],
                X509AlgorithmParametersPolicy::Absent,
                KEY_OID,
                X509AlgorithmParametersPolicy::Absent,
                1,
                8,
                1,
                8,
            ),
            X509CertificateAlgorithmProfileDefinitionErrorKind::EmptySignatureAlgorithm,
        ),
        (
            X509CertificateAlgorithmProfile::new(
                SIGNATURE_OID,
                X509AlgorithmParametersPolicy::Absent,
                &[],
                X509AlgorithmParametersPolicy::Absent,
                1,
                8,
                1,
                8,
            ),
            X509CertificateAlgorithmProfileDefinitionErrorKind::EmptySubjectPublicKeyAlgorithm,
        ),
        (
            X509CertificateAlgorithmProfile::new(
                SIGNATURE_OID,
                X509AlgorithmParametersPolicy::Absent,
                KEY_OID,
                X509AlgorithmParametersPolicy::Absent,
                0,
                8,
                1,
                8,
            ),
            X509CertificateAlgorithmProfileDefinitionErrorKind::InvalidSubjectPublicKeyBitLengthRange,
        ),
        (
            X509CertificateAlgorithmProfile::new(
                SIGNATURE_OID,
                X509AlgorithmParametersPolicy::Absent,
                KEY_OID,
                X509AlgorithmParametersPolicy::Absent,
                9,
                8,
                1,
                8,
            ),
            X509CertificateAlgorithmProfileDefinitionErrorKind::InvalidSubjectPublicKeyBitLengthRange,
        ),
        (
            X509CertificateAlgorithmProfile::new(
                SIGNATURE_OID,
                X509AlgorithmParametersPolicy::Absent,
                KEY_OID,
                X509AlgorithmParametersPolicy::Absent,
                1,
                8,
                0,
                8,
            ),
            X509CertificateAlgorithmProfileDefinitionErrorKind::InvalidSignatureBitLengthRange,
        ),
        (
            X509CertificateAlgorithmProfile::new(
                SIGNATURE_OID,
                X509AlgorithmParametersPolicy::Absent,
                KEY_OID,
                X509AlgorithmParametersPolicy::Absent,
                1,
                8,
                9,
                8,
            ),
            X509CertificateAlgorithmProfileDefinitionErrorKind::InvalidSignatureBitLengthRange,
        ),
    ];

    for (result, expected) in cases {
        assert_eq!(result.unwrap_err().kind(), expected);
    }
}

#[test]
fn selects_exact_algorithm_and_parameter_profile_with_borrowed_evidence() {
    let encoded = certificate(
        SIGNATURE_OID_DER,
        Some(NULL_DER),
        KEY_OID_DER,
        Some(NULL_DER),
        &[0, 0xaa, 0xbb],
        &[0, 0xcc, 0xdd],
    );
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let profiles = [profile(
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        16,
        16,
        16,
        16,
    )];

    let evidence = validate_certificate_algorithm_profile(&certificate, &profiles).unwrap();

    assert_eq!(profiles[0].signature_algorithm(), SIGNATURE_OID);
    assert_eq!(
        profiles[0].signature_parameters(),
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER)
    );
    assert_eq!(profiles[0].subject_public_key_algorithm(), KEY_OID);
    assert_eq!(
        profiles[0].subject_public_key_parameters(),
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER)
    );
    assert_eq!(
        profiles[0].subject_public_key_bit_length().minimum_bits(),
        16
    );
    assert_eq!(
        profiles[0].subject_public_key_bit_length().maximum_bits(),
        16
    );
    assert_eq!(profiles[0].signature_bit_length().minimum_bits(), 16);
    assert_eq!(profiles[0].signature_bit_length().maximum_bits(), 16);
    assert_eq!(evidence.profile_index(), 0);
    assert!(evidence
        .signature_algorithm()
        .algorithm()
        .equals(SIGNATURE_OID));
    assert!(evidence
        .subject_public_key_algorithm()
        .algorithm()
        .equals(KEY_OID));
    assert_eq!(evidence.subject_public_key().bytes(), &[0xaa, 0xbb]);
    assert_eq!(evidence.signature_value().bytes(), &[0xcc, 0xdd]);
}

#[test]
fn distinguishes_absent_from_exact_parameters() {
    let encoded = certificate(
        SIGNATURE_OID_DER,
        None,
        KEY_OID_DER,
        None,
        &[0, 0xaa],
        &[0, 0xbb],
    );
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let profiles = [
        profile(
            X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
            X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
            8,
            8,
            8,
            8,
        ),
        profile(
            X509AlgorithmParametersPolicy::Absent,
            X509AlgorithmParametersPolicy::Absent,
            8,
            8,
            8,
            8,
        ),
    ];

    let evidence = validate_certificate_algorithm_profile(&certificate, &profiles).unwrap();
    assert_eq!(evidence.profile_index(), 1);
}

#[test]
fn rejects_present_but_nonmatching_parameter_bytes() {
    let encoded = certificate(
        SIGNATURE_OID_DER,
        Some(NULL_DER),
        KEY_OID_DER,
        Some(NULL_DER),
        &[0, 0xaa],
        &[0, 0xbb],
    );
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let profiles = [profile(
        X509AlgorithmParametersPolicy::ExactDer(&[0x02, 0x01, 0x01]),
        X509AlgorithmParametersPolicy::ExactDer(NULL_DER),
        8,
        8,
        8,
        8,
    )];

    let error = validate_certificate_algorithm_profile(&certificate, &profiles).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificateAlgorithmErrorKind::UnsupportedAlgorithmProfile
    );
}

#[test]
fn rejects_unsupported_identifier_pair() {
    let encoded = certificate(
        OTHER_OID_DER,
        None,
        KEY_OID_DER,
        None,
        &[0, 0xaa],
        &[0, 0xbb],
    );
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let profiles = [profile(
        X509AlgorithmParametersPolicy::Absent,
        X509AlgorithmParametersPolicy::Absent,
        1,
        8,
        1,
        8,
    )];

    let error = validate_certificate_algorithm_profile(&certificate, &profiles).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificateAlgorithmErrorKind::UnsupportedAlgorithmProfile
    );
}

#[test]
fn rejects_nonmatching_subject_public_key_identifier_fields() {
    for (key_oid, key_parameters) in [(OTHER_OID_DER, None), (KEY_OID_DER, Some(NULL_DER))] {
        let encoded = certificate(
            SIGNATURE_OID_DER,
            None,
            key_oid,
            key_parameters,
            &[0, 0xaa],
            &[0, 0xbb],
        );
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let profiles = [profile(
            X509AlgorithmParametersPolicy::Absent,
            X509AlgorithmParametersPolicy::Absent,
            1,
            8,
            1,
            8,
        )];

        let error = validate_certificate_algorithm_profile(&certificate, &profiles).unwrap_err();
        assert_eq!(
            error.kind(),
            X509CertificateAlgorithmErrorKind::UnsupportedAlgorithmProfile
        );
    }
}

#[test]
fn rejects_ambiguous_identifier_matches_before_length_policy() {
    let encoded = certificate(
        SIGNATURE_OID_DER,
        None,
        KEY_OID_DER,
        None,
        &[0, 0xaa],
        &[0, 0xbb],
    );
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let profiles = [
        profile(
            X509AlgorithmParametersPolicy::Absent,
            X509AlgorithmParametersPolicy::Absent,
            100,
            200,
            100,
            200,
        ),
        profile(
            X509AlgorithmParametersPolicy::Absent,
            X509AlgorithmParametersPolicy::Absent,
            1,
            8,
            1,
            8,
        ),
    ];

    let error = validate_certificate_algorithm_profile(&certificate, &profiles).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificateAlgorithmErrorKind::AmbiguousAlgorithmProfile
    );
}

#[test]
fn applies_inclusive_public_key_length_bounds() {
    for (minimum, maximum, expected) in [
        (13, 13, Ok(())),
        (
            14,
            20,
            Err(X509CertificateAlgorithmErrorKind::SubjectPublicKeyLengthOutOfPolicy),
        ),
        (
            1,
            12,
            Err(X509CertificateAlgorithmErrorKind::SubjectPublicKeyLengthOutOfPolicy),
        ),
    ] {
        let encoded = certificate(
            SIGNATURE_OID_DER,
            None,
            KEY_OID_DER,
            None,
            &[3, 0xaa, 0xb8],
            &[0, 0xbb],
        );
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let profiles = [profile(
            X509AlgorithmParametersPolicy::Absent,
            X509AlgorithmParametersPolicy::Absent,
            minimum,
            maximum,
            8,
            8,
        )];
        let result = validate_certificate_algorithm_profile(&certificate, &profiles)
            .map(|_| ())
            .map_err(|error| error.kind());

        assert_eq!(result, expected);
    }
}

#[test]
fn applies_inclusive_signature_length_bounds() {
    for (minimum, maximum, expected) in [
        (13, 13, Ok(())),
        (
            14,
            20,
            Err(X509CertificateAlgorithmErrorKind::SignatureLengthOutOfPolicy),
        ),
        (
            1,
            12,
            Err(X509CertificateAlgorithmErrorKind::SignatureLengthOutOfPolicy),
        ),
    ] {
        let encoded = certificate(
            SIGNATURE_OID_DER,
            None,
            KEY_OID_DER,
            None,
            &[0, 0xaa],
            &[3, 0xbb, 0xb8],
        );
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let profiles = [profile(
            X509AlgorithmParametersPolicy::Absent,
            X509AlgorithmParametersPolicy::Absent,
            8,
            8,
            minimum,
            maximum,
        )];
        let result = validate_certificate_algorithm_profile(&certificate, &profiles)
            .map(|_| ())
            .map_err(|error| error.kind());

        assert_eq!(result, expected);
    }
}

#[test]
fn errors_disclose_only_stable_categories() {
    let definition = X509CertificateAlgorithmProfile::new(
        &[],
        X509AlgorithmParametersPolicy::ExactDer(b"secret-parameters"),
        KEY_OID,
        X509AlgorithmParametersPolicy::Absent,
        1,
        8,
        1,
        8,
    )
    .unwrap_err();
    assert!(!definition.to_string().contains("secret"));

    let encoded = certificate(
        OTHER_OID_DER,
        None,
        KEY_OID_DER,
        None,
        &[0, 0xde, 0xad],
        &[0, 0xbe, 0xef],
    );
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let error = validate_certificate_algorithm_profile(&certificate, &[]).unwrap_err();
    let display = error.to_string();
    assert!(!display.contains("1.2.3"));
    assert!(!display.contains("dead"));
    assert!(!display.contains("beef"));
}
