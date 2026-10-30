use der_asn1::{Asn1ErrorKind, Asn1Limits};
use x509_ocsp_cert_id::X509OcspCertIdErrorKind;
use x509_ocsp_request::{
    encode_unsigned_x509_ocsp_request, validate_x509_ocsp_request_cert_id, X509OcspRequestErrorKind,
};

fn der_length(length: usize) -> Vec<u8> {
    if length <= 127 {
        return vec![length as u8];
    }
    let bytes = length.to_be_bytes();
    let first = bytes.iter().position(|byte| *byte != 0).unwrap();
    let significant = &bytes[first..];
    let mut encoded = vec![0x80 | significant.len() as u8];
    encoded.extend_from_slice(significant);
    encoded
}

fn der(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(value.len() + 4);
    encoded.push(tag);
    encoded.extend_from_slice(&der_length(value.len()));
    encoded.extend_from_slice(value);
    encoded
}

fn cert_id(hash_length: usize) -> Vec<u8> {
    let algorithm_identifier = der(
        0x30,
        &[0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00],
    );
    let issuer_name_hash: Vec<u8> = (0..hash_length).map(|index| index as u8).collect();
    let issuer_key_hash: Vec<u8> = (0..hash_length)
        .map(|index| (index as u8).wrapping_add(0x40))
        .collect();
    let mut contents = algorithm_identifier;
    contents.extend(der(0x04, &issuer_name_hash));
    contents.extend(der(0x04, &issuer_key_hash));
    contents.extend([0x02, 0x01, 0x7f]);
    der(0x30, &contents)
}

fn expected_request(cert_id: &[u8]) -> Vec<u8> {
    let request = der(0x30, cert_id);
    let request_list = der(0x30, &request);
    let tbs_request = der(0x30, &request_list);
    der(0x30, &tbs_request)
}

#[test]
fn encodes_one_exact_minimal_unsigned_request() {
    let encoded_cert_id = cert_id(20);
    let validated =
        validate_x509_ocsp_request_cert_id(&encoded_cert_id, Asn1Limits::default()).unwrap();
    let expected = expected_request(&encoded_cert_id);
    let mut output = vec![0xa5; expected.len() + 3];
    let output_pointer = output.as_ptr();

    {
        let encoded = encode_unsigned_x509_ocsp_request(&validated, &mut output).unwrap();

        assert_eq!(encoded.encoded(), expected);
        assert_eq!(encoded.encoded().as_ptr(), output_pointer);
        assert_eq!(validated.encoded_request_len(), expected.len());
        assert_eq!(validated.encoded_cert_id(), encoded_cert_id);
        assert_eq!(validated.cert_id().issuer_name_hash().len(), 20);
        assert!(std::ptr::eq(encoded.cert_id(), &validated));
        assert_eq!(format!("{validated:?}"), "ValidatedX509OcspRequestCertId");
        assert_eq!(format!("{encoded:?}"), "EncodedX509OcspRequest");
    }
    assert_eq!(&output[expected.len()..], &[0xa5; 3]);
}

#[test]
fn encodes_canonical_nested_long_form_lengths() {
    let encoded_cert_id = cert_id(64);
    let validated =
        validate_x509_ocsp_request_cert_id(&encoded_cert_id, Asn1Limits::default()).unwrap();
    let expected = expected_request(&encoded_cert_id);
    let mut output = vec![0; expected.len()];

    let encoded = encode_unsigned_x509_ocsp_request(&validated, &mut output).unwrap();

    assert_eq!(encoded.encoded(), expected);
    assert_eq!(encoded.encoded()[1] & 0x80, 0x80);
}

#[test]
fn rejects_noncanonical_or_invalid_identifier_input() {
    let trailing = [cert_id(20), vec![0x05, 0x00]].concat();
    let trailing_error =
        validate_x509_ocsp_request_cert_id(&trailing, Asn1Limits::default()).unwrap_err();
    assert!(matches!(
        trailing_error.kind(),
        X509OcspRequestErrorKind::CertIdFraming(Asn1ErrorKind::Framing(_))
    ));

    let invalid = der(0x30, &[0x05, 0x00]);
    let invalid_error =
        validate_x509_ocsp_request_cert_id(&invalid, Asn1Limits::default()).unwrap_err();
    assert!(matches!(
        invalid_error.kind(),
        X509OcspRequestErrorKind::InvalidCertId(X509OcspCertIdErrorKind::InvalidHashAlgorithm(_))
    ));
    assert!(!invalid_error.to_string().contains("0500"));
}

#[test]
fn insufficient_output_is_unchanged_and_redacted() {
    let encoded_cert_id = cert_id(20);
    let validated =
        validate_x509_ocsp_request_cert_id(&encoded_cert_id, Asn1Limits::default()).unwrap();
    let mut output = vec![0x5a; validated.encoded_request_len() - 1];

    let error = encode_unsigned_x509_ocsp_request(&validated, &mut output).unwrap_err();

    assert_eq!(error.kind(), X509OcspRequestErrorKind::OutputTooSmall);
    assert!(output.iter().all(|byte| *byte == 0x5a));
    assert_eq!(
        error.to_string(),
        "X.509 OCSP request framing error OutputTooSmall"
    );
}
