use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_serial_number::{
    decode_x509_serial_number, X509SerialNumberErrorKind, MAX_SERIAL_NUMBER_OCTETS,
};

fn decode(
    encoded: &[u8],
) -> Result<x509_serial_number::X509SerialNumber<'_>, x509_serial_number::X509SerialNumberError> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder
        .decode_exact(encoded)
        .expect("valid outer DER framing");
    decode_x509_serial_number(element)
}

#[test]
fn decodes_positive_values_and_borrows_the_magnitude() {
    let encoded = [0x02, 0x03, 0x01, 0x23, 0x45];
    let serial = decode(&encoded).unwrap();
    assert_eq!(serial.encoded_value(), &[0x01, 0x23, 0x45]);
    assert_eq!(serial.magnitude(), &[0x01, 0x23, 0x45]);
    assert_eq!(serial.encoded_len(), 3);
}

#[test]
fn strips_only_the_required_positive_sign_octet_from_the_magnitude() {
    let encoded = [0x02, 0x03, 0x00, 0x80, 0x01];
    let serial = decode(&encoded).unwrap();
    assert_eq!(serial.encoded_value(), &[0x00, 0x80, 0x01]);
    assert_eq!(serial.magnitude(), &[0x80, 0x01]);
}

#[test]
fn accepts_the_exact_twenty_octet_bound_without_integer_conversion() {
    let mut encoded = vec![0x02, MAX_SERIAL_NUMBER_OCTETS as u8];
    encoded.push(0x01);
    encoded.extend(std::iter::repeat_n(0xff, MAX_SERIAL_NUMBER_OCTETS - 1));
    let serial = decode(&encoded).unwrap();
    assert_eq!(serial.encoded_len(), MAX_SERIAL_NUMBER_OCTETS);
    assert_eq!(serial.magnitude().len(), MAX_SERIAL_NUMBER_OCTETS);
}

#[test]
fn accepts_a_twenty_octet_value_with_a_required_sign_octet() {
    let mut encoded = vec![0x02, MAX_SERIAL_NUMBER_OCTETS as u8, 0x00, 0x80];
    encoded.extend(std::iter::repeat_n(0x5a, MAX_SERIAL_NUMBER_OCTETS - 2));
    let serial = decode(&encoded).unwrap();
    assert_eq!(serial.encoded_len(), MAX_SERIAL_NUMBER_OCTETS);
    assert_eq!(serial.magnitude().len(), MAX_SERIAL_NUMBER_OCTETS - 1);
}

#[test]
fn rejects_zero_and_negative_values() {
    for encoded in [[0x02, 0x01, 0x00], [0x02, 0x01, 0xff]] {
        let error = decode(&encoded).unwrap_err();
        assert_eq!(error.kind(), X509SerialNumberErrorKind::NotPositive);
        assert_eq!(error.offset(), 2);
    }
}

#[test]
fn rejects_a_twenty_first_content_octet() {
    let mut encoded = vec![0x02, 21, 0x01];
    encoded.extend(std::iter::repeat_n(0x5a, 20));
    let error = decode(&encoded).unwrap_err();
    assert_eq!(error.kind(), X509SerialNumberErrorKind::TooLong);
    assert_eq!(error.offset(), 22);
}

#[test]
fn rejects_non_integer_and_constructed_integer_tags() {
    for encoded in [[0x04, 0x01, 0x01], [0x22, 0x01, 0x01]] {
        let error = decode(&encoded).unwrap_err();
        assert_eq!(
            error.kind(),
            X509SerialNumberErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
        assert_eq!(error.offset(), 0);
    }
}

#[test]
fn rejects_empty_and_non_minimal_integer_encodings() {
    for (encoded, expected) in [
        (&[0x02, 0x00][..], Asn1ErrorKind::EmptyInteger),
        (
            &[0x02, 0x02, 0x00, 0x7f][..],
            Asn1ErrorKind::NonMinimalInteger,
        ),
        (
            &[0x02, 0x02, 0xff, 0x80][..],
            Asn1ErrorKind::NonMinimalInteger,
        ),
    ] {
        let error = decode(encoded).unwrap_err();
        assert_eq!(error.kind(), X509SerialNumberErrorKind::Structure(expected));
        assert_eq!(error.offset(), 2);
    }
}

#[test]
fn caller_der_value_limit_applies_before_semantic_decoding() {
    let mut limits = Asn1Limits::default();
    limits.der.max_value_len = 2;
    let mut decoder = Asn1Decoder::new(limits);
    let error = decoder
        .decode_exact(&[0x02, 0x03, 0x01, 0x02, 0x03])
        .unwrap_err();
    assert!(matches!(error.kind(), Asn1ErrorKind::Framing(_)));
}

#[test]
fn serial_debug_and_errors_do_not_disclose_value_bytes() {
    let serial = decode(&[0x02, 0x03, 0x01, 0xab, 0xcd]).unwrap();
    let rendered = format!("{serial:?}");
    assert!(!rendered.contains("ab"));
    assert!(!rendered.contains("cd"));
    assert!(rendered.contains("magnitude_len: 3"));

    let error = decode(&[0x02, 0x01, 0xde]).unwrap_err();
    assert!(!error.to_string().contains("de"));
    assert!(error.to_string().contains("byte 2"));
}
