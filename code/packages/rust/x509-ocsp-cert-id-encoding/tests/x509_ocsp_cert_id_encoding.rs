use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_ocsp_cert_id::decode_x509_ocsp_cert_id;
use x509_ocsp_cert_id_encoding::{
    encode_x509_ocsp_cert_id, validate_x509_ocsp_cert_id_fields, X509OcspCertIdEncodingErrorKind,
};
use x509_serial_number::X509SerialNumberErrorKind;

const SHA1_ALGORITHM_IDENTIFIER: [u8; 11] = [
    0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00,
];
const SERIAL_ONE: [u8; 3] = [0x02, 0x01, 0x01];

#[test]
fn encodes_one_exact_short_form_identifier() {
    let name_hash = [0x11; 20];
    let key_hash = [0x22; 20];
    let fields = validate_x509_ocsp_cert_id_fields(
        &SHA1_ALGORITHM_IDENTIFIER,
        &name_hash,
        &key_hash,
        &SERIAL_ONE,
        Asn1Limits::default(),
    )
    .expect("valid fields");

    assert_eq!(fields.encoded_len(), 60);
    assert_eq!(
        fields.algorithm_identifier_der(),
        &SHA1_ALGORITHM_IDENTIFIER
    );
    assert_eq!(fields.issuer_name_hash(), &name_hash);
    assert_eq!(fields.issuer_key_hash(), &key_hash);
    assert_eq!(fields.serial_number_der(), &SERIAL_ONE);
    assert_eq!(fields.serial_number().magnitude(), [1]);

    let mut output = [0xa5; 64];
    let encoded = encode_x509_ocsp_cert_id(&fields, &mut output).expect("encode");

    let mut expected = vec![0x30, 0x3a];
    expected.extend_from_slice(&SHA1_ALGORITHM_IDENTIFIER);
    expected.extend_from_slice(&[0x04, 0x14]);
    expected.extend_from_slice(&name_hash);
    expected.extend_from_slice(&[0x04, 0x14]);
    expected.extend_from_slice(&key_hash);
    expected.extend_from_slice(&SERIAL_ONE);

    assert!(core::ptr::eq(encoded.fields(), &fields));
    assert_eq!(encoded.encoded(), expected);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder
        .decode_exact(encoded.encoded())
        .expect("encoded identifier is canonical DER");
    let decoded = decode_x509_ocsp_cert_id(&mut decoder, element)
        .expect("encoded identifier has the RFC 6960 shape");
    assert_eq!(decoded.issuer_name_hash(), name_hash);
    assert_eq!(decoded.issuer_key_hash(), key_hash);
    assert_eq!(decoded.serial_number().magnitude(), [1]);
    assert_eq!(&output[60..], &[0xa5; 4]);
}

#[test]
fn encodes_canonical_nested_long_form_lengths() {
    let name_hash = [0x31; 128];
    let key_hash = [0x42; 128];
    let fields = validate_x509_ocsp_cert_id_fields(
        &SHA1_ALGORITHM_IDENTIFIER,
        &name_hash,
        &key_hash,
        &SERIAL_ONE,
        Asn1Limits::default(),
    )
    .expect("valid fields");
    assert_eq!(fields.encoded_len(), 280);

    let mut output = [0u8; 280];
    let encoded = encode_x509_ocsp_cert_id(&fields, &mut output).expect("encode");
    assert_eq!(&encoded.encoded()[..4], &[0x30, 0x82, 0x01, 0x14]);
    assert_eq!(&encoded.encoded()[4..15], &SHA1_ALGORITHM_IDENTIFIER);
    assert_eq!(&encoded.encoded()[15..18], &[0x04, 0x81, 0x80]);
    assert_eq!(&encoded.encoded()[146..149], &[0x04, 0x81, 0x80]);
    assert_eq!(&encoded.encoded()[277..], &SERIAL_ONE);
}

#[test]
fn rejects_noncanonical_algorithm_or_serial_inputs() {
    let name_hash = [0x11; 20];
    let key_hash = [0x22; 20];
    let mut algorithm_with_trailing = SHA1_ALGORITHM_IDENTIFIER.to_vec();
    algorithm_with_trailing.push(0x00);
    let error = validate_x509_ocsp_cert_id_fields(
        &algorithm_with_trailing,
        &name_hash,
        &key_hash,
        &SERIAL_ONE,
        Asn1Limits::default(),
    )
    .expect_err("trailing algorithm input must fail");
    assert!(matches!(
        error.kind(),
        X509OcspCertIdEncodingErrorKind::AlgorithmIdentifierFraming(_)
    ));

    let zero_serial = [0x02, 0x01, 0x00];
    let error = validate_x509_ocsp_cert_id_fields(
        &SHA1_ALGORITHM_IDENTIFIER,
        &name_hash,
        &key_hash,
        &zero_serial,
        Asn1Limits::default(),
    )
    .expect_err("zero serial must fail");
    assert_eq!(
        error.kind(),
        X509OcspCertIdEncodingErrorKind::InvalidSerialNumber(
            X509SerialNumberErrorKind::NotPositive
        )
    );

    let serial_with_trailing = [0x02, 0x01, 0x01, 0x00];
    let error = validate_x509_ocsp_cert_id_fields(
        &SHA1_ALGORITHM_IDENTIFIER,
        &name_hash,
        &key_hash,
        &serial_with_trailing,
        Asn1Limits::default(),
    )
    .expect_err("trailing serial input must fail");
    assert!(matches!(
        error.kind(),
        X509OcspCertIdEncodingErrorKind::SerialNumberFraming(_)
    ));
}

#[test]
fn insufficient_output_is_unchanged_and_failures_are_redacted() {
    let name_hash = [0x11; 20];
    let key_hash = [0x22; 20];
    let fields = validate_x509_ocsp_cert_id_fields(
        &SHA1_ALGORITHM_IDENTIFIER,
        &name_hash,
        &key_hash,
        &SERIAL_ONE,
        Asn1Limits::default(),
    )
    .expect("valid fields");
    let mut output = [0x7c; 59];
    let error =
        encode_x509_ocsp_cert_id(&fields, &mut output).expect_err("undersized output must fail");
    assert_eq!(
        error.kind(),
        X509OcspCertIdEncodingErrorKind::OutputTooSmall
    );
    assert_eq!(output, [0x7c; 59]);
    assert_eq!(format!("{fields:?}"), "ValidatedX509OcspCertIdFields");
    assert!(!format!("{error:?}").contains("17"));
    assert!(!error.to_string().contains("17"));
}
