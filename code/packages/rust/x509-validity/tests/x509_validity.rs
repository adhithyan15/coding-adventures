use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_time::{decode_x509_time, X509Time, X509TimeErrorKind};
use x509_validity::{decode_x509_validity, X509ValidityErrorKind, X509ValidityStatus};

fn utc(contents: &[u8; 13]) -> Vec<u8> {
    let mut encoded = vec![0x17, 13];
    encoded.extend_from_slice(contents);
    encoded
}

fn generalized(contents: &[u8; 15]) -> Vec<u8> {
    let mut encoded = vec![0x18, 15];
    encoded.extend_from_slice(contents);
    encoded
}

fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
    let value_length: usize = fields.iter().map(Vec::len).sum();
    assert!(value_length < 128);
    let mut encoded = vec![0x30, value_length as u8];
    for field in fields {
        encoded.extend_from_slice(field);
    }
    encoded
}

fn decode(encoded: &[u8]) -> Result<x509_validity::X509Validity, x509_validity::X509ValidityError> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_validity(&mut decoder, element)
}

fn time(encoded: &[u8]) -> X509Time {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_time(element).expect("valid profile time")
}

#[test]
fn decodes_utc_generalized_and_mixed_endpoints() {
    for encoded in [
        sequence(&[utc(b"500101000000Z"), utc(b"491231235959Z")]),
        sequence(&[
            generalized(b"20500101000000Z"),
            generalized(b"24001231235959Z"),
        ]),
        sequence(&[utc(b"491231235959Z"), generalized(b"20500101000000Z")]),
    ] {
        assert!(decode(&encoded).is_ok());
    }
}

#[test]
fn classification_is_inclusive_at_both_endpoints() {
    let validity = decode(&sequence(&[utc(b"240101000000Z"), utc(b"241231235959Z")])).unwrap();

    assert_eq!(validity.not_before(), time(&utc(b"240101000000Z")));
    assert_eq!(validity.not_after(), time(&utc(b"241231235959Z")));

    for observed in [utc(b"240101000000Z"), utc(b"241231235959Z")] {
        assert_eq!(
            validity.classify(&time(&observed)),
            X509ValidityStatus::Valid
        );
    }
    assert_eq!(
        validity.classify(&time(&utc(b"231231235959Z"))),
        X509ValidityStatus::NotYetValid
    );
    assert_eq!(
        validity.classify(&time(&utc(b"250101000000Z"))),
        X509ValidityStatus::Expired
    );
}

#[test]
fn equal_endpoints_form_one_inclusive_instant() {
    let validity = decode(&sequence(&[
        generalized(b"20500101000000Z"),
        generalized(b"20500101000000Z"),
    ]))
    .unwrap();
    assert_eq!(
        validity.classify(&time(&generalized(b"20500101000000Z"))),
        X509ValidityStatus::Valid
    );
}

#[test]
fn inverted_ranges_fail_closed() {
    let encoded = sequence(&[utc(b"250101000000Z"), utc(b"240101000000Z")]);
    assert_eq!(
        decode(&encoded).unwrap_err().kind(),
        X509ValidityErrorKind::InvertedRange
    );
}

#[test]
fn root_must_be_a_constructed_universal_sequence() {
    for encoded in [vec![0x10, 0], vec![0x31, 0]] {
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509ValidityErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
    }
}

#[test]
fn exactly_two_fields_are_required() {
    assert_eq!(
        decode(&sequence(&[])).unwrap_err().kind(),
        X509ValidityErrorKind::MissingNotBefore
    );
    assert_eq!(
        decode(&sequence(&[utc(b"240101000000Z")]))
            .unwrap_err()
            .kind(),
        X509ValidityErrorKind::MissingNotAfter
    );
    assert_eq!(
        decode(&sequence(&[
            utc(b"240101000000Z"),
            utc(b"250101000000Z"),
            utc(b"260101000000Z"),
        ]))
        .unwrap_err()
        .kind(),
        X509ValidityErrorKind::TrailingElement
    );
}

#[test]
fn malformed_times_are_attributed_to_their_field() {
    let invalid_before = sequence(&[utc(b"240001000000Z"), utc(b"250101000000Z")]);
    assert_eq!(
        decode(&invalid_before).unwrap_err().kind(),
        X509ValidityErrorKind::InvalidNotBefore(X509TimeErrorKind::InvalidMonth)
    );

    let invalid_after = sequence(&[utc(b"240101000000Z"), utc(b"250101000060Z")]);
    assert_eq!(
        decode(&invalid_after).unwrap_err().kind(),
        X509ValidityErrorKind::InvalidNotAfter(X509TimeErrorKind::InvalidSecond)
    );
}

#[test]
fn shared_decoder_depth_and_element_limits_are_enforced() {
    let encoded = sequence(&[utc(b"240101000000Z"), utc(b"250101000000Z")]);

    let depth_limits = Asn1Limits {
        max_depth: 1,
        ..Asn1Limits::default()
    };
    let mut depth_decoder = Asn1Decoder::new(depth_limits);
    let root = depth_decoder.decode_exact(&encoded).unwrap();
    assert_eq!(
        decode_x509_validity(&mut depth_decoder, root)
            .unwrap_err()
            .kind(),
        X509ValidityErrorKind::Structure(Asn1ErrorKind::DepthLimitExceeded)
    );

    let element_limits = Asn1Limits {
        max_total_elements: 2,
        ..Asn1Limits::default()
    };
    let mut element_decoder = Asn1Decoder::new(element_limits);
    let root = element_decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_validity(&mut element_decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509ValidityErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
    assert_eq!(error.offset(), 17);
}

#[test]
fn child_framing_offsets_are_local_to_the_whole_validity_element() {
    for encoded in [
        sequence(&[vec![0x17, 13, b'2']]),
        sequence(&[utc(b"240101000000Z"), vec![0x17, 13, b'2']]),
    ] {
        let error = decode(&encoded).unwrap_err();
        assert!(matches!(
            error.kind(),
            X509ValidityErrorKind::Structure(Asn1ErrorKind::Framing(_))
        ));
        assert_eq!(error.offset(), encoded.len());
    }
}

#[test]
fn errors_are_redacted_and_offsets_are_local_to_the_validity_element() {
    let encoded = sequence(&[utc(b"240101000000Z"), utc(b"25X101000000Z")]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        X509ValidityErrorKind::InvalidNotAfter(X509TimeErrorKind::NonDigit)
    );
    assert_eq!(error.offset(), 21);
    assert!(!error.to_string().contains("25X101"));
}
