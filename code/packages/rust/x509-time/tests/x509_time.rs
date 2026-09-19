use core::cmp::Ordering;

use der_asn1::{Asn1Decoder, Asn1Element, Asn1Limits};
use x509_time::{decode_x509_time, X509TimeEncoding, X509TimeErrorKind};

fn decode_element(input: &[u8]) -> Asn1Element<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    decoder.decode_exact(input).expect("valid DER framing")
}

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

#[test]
fn utc_time_resolves_the_fixed_rfc_5280_century() {
    for (contents, expected_year) in [
        (b"500101000000Z", 1950),
        (b"991231235959Z", 1999),
        (b"000101000000Z", 2000),
        (b"491231235959Z", 2049),
    ] {
        let encoded = utc(contents);
        let time = decode_x509_time(decode_element(&encoded)).unwrap();
        assert_eq!(time.encoding(), X509TimeEncoding::UtcTime);
        assert_eq!(time.year(), expected_year);
    }
}

#[test]
fn generalized_time_starts_at_2050_and_reaches_9999() {
    for (contents, expected_year) in [(b"20500101000000Z", 2050), (b"99991231235959Z", 9999)] {
        let encoded = generalized(contents);
        let time = decode_x509_time(decode_element(&encoded)).unwrap();
        assert_eq!(time.encoding(), X509TimeEncoding::GeneralizedTime);
        assert_eq!(time.year(), expected_year);
    }

    let encoded = generalized(b"20491231235959Z");
    assert_eq!(
        decode_x509_time(decode_element(&encoded))
            .unwrap_err()
            .kind(),
        X509TimeErrorKind::GeneralizedTimeBefore2050
    );
}

#[test]
fn calendar_validation_uses_gregorian_leap_rules() {
    for encoded in [
        utc(b"960229120000Z"),
        utc(b"000229120000Z"),
        generalized(b"24000229120000Z"),
    ] {
        assert!(decode_x509_time(decode_element(&encoded)).is_ok());
    }

    for encoded in [utc(b"990229120000Z"), generalized(b"21000229120000Z")] {
        assert_eq!(
            decode_x509_time(decode_element(&encoded))
                .unwrap_err()
                .kind(),
            X509TimeErrorKind::InvalidDay
        );
    }
}

#[test]
fn calendar_fields_are_exposed_without_a_clock_or_epoch() {
    let encoded = generalized(b"24000229123456Z");
    let time = decode_x509_time(decode_element(&encoded)).unwrap();
    assert_eq!(time.year(), 2400);
    assert_eq!(time.month(), 2);
    assert_eq!(time.day(), 29);
    assert_eq!(time.hour(), 12);
    assert_eq!(time.minute(), 34);
    assert_eq!(time.second(), 56);
}

#[test]
fn every_calendar_field_rejects_its_closed_upper_or_lower_boundary() {
    for (encoded, expected) in [
        (utc(b"500001000000Z"), X509TimeErrorKind::InvalidMonth),
        (utc(b"501301000000Z"), X509TimeErrorKind::InvalidMonth),
        (utc(b"500100000000Z"), X509TimeErrorKind::InvalidDay),
        (utc(b"500431000000Z"), X509TimeErrorKind::InvalidDay),
        (utc(b"500101240000Z"), X509TimeErrorKind::InvalidHour),
        (utc(b"500101006000Z"), X509TimeErrorKind::InvalidMinute),
        (utc(b"500101000060Z"), X509TimeErrorKind::InvalidSecond),
    ] {
        assert_eq!(
            decode_x509_time(decode_element(&encoded))
                .unwrap_err()
                .kind(),
            expected
        );
    }
}

#[test]
fn exact_tags_and_primitive_form_are_required() {
    for encoded in [
        [&[0x16, 13][..], b"500101000000Z"].concat(),
        [&[0x37, 13][..], b"500101000000Z"].concat(),
        [&[0x38, 15][..], b"20500101000000Z"].concat(),
    ] {
        assert_eq!(
            decode_x509_time(decode_element(&encoded))
                .unwrap_err()
                .kind(),
            X509TimeErrorKind::UnexpectedTag
        );
    }
}

#[test]
fn alternate_ber_and_non_profile_spellings_are_rejected() {
    let cases: Vec<(Vec<u8>, X509TimeErrorKind)> = vec![
        (
            [&[0x17, 11][..], b"5001010000Z"].concat(),
            X509TimeErrorKind::InvalidLength,
        ),
        (
            [&[0x17, 13][..], b"500101000000z"].concat(),
            X509TimeErrorKind::MissingZulu,
        ),
        (
            [&[0x17, 13][..], b"500101000000+"].concat(),
            X509TimeErrorKind::MissingZulu,
        ),
        (
            [&[0x18, 17][..], b"20500101000000.1Z"].concat(),
            X509TimeErrorKind::InvalidLength,
        ),
        (
            [&[0x18, 15][..], b"2050010100000AZ"].concat(),
            X509TimeErrorKind::NonDigit,
        ),
    ];

    for (encoded, expected) in cases {
        assert_eq!(
            decode_x509_time(decode_element(&encoded))
                .unwrap_err()
                .kind(),
            expected
        );
    }
}

#[test]
fn ordering_compares_only_validated_calendar_fields() {
    let utc_encoded = utc(b"491231235959Z");
    let generalized_encoded = generalized(b"20500101000000Z");
    let before = decode_x509_time(decode_element(&utc_encoded)).unwrap();
    let after = decode_x509_time(decode_element(&generalized_encoded)).unwrap();
    assert_eq!(before.cmp_fields(&after), Ordering::Less);
    assert_eq!(after.cmp_fields(&before), Ordering::Greater);
    assert_eq!(before.cmp_fields(&before), Ordering::Equal);
}

#[test]
fn errors_are_redacted_and_point_to_the_local_field() {
    let encoded = utc(b"50X101000000Z");
    let error = decode_x509_time(decode_element(&encoded)).unwrap_err();
    assert_eq!(error.kind(), X509TimeErrorKind::NonDigit);
    assert_eq!(error.offset(), 4);
    assert!(!error.to_string().contains("50X101"));
}

#[test]
fn the_first_invalid_field_wins() {
    let encoded = utc(b"50130000000XZ");
    let error = decode_x509_time(decode_element(&encoded)).unwrap_err();
    assert_eq!(error.kind(), X509TimeErrorKind::InvalidMonth);
    assert_eq!(error.offset(), 4);
}
