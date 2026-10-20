use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_directory_string::{
    compare_x509_directory_strings_by_scalar, decode_x509_directory_string, X509DirectoryString,
    X509DirectoryStringComparisonError, X509DirectoryStringEncoding, X509DirectoryStringError,
    X509DirectoryStringErrorKind,
};

fn der(tag: u8, value: &[u8]) -> Vec<u8> {
    assert!(value.len() < 128);
    let mut encoded = vec![tag, value.len() as u8];
    encoded.extend(value);
    encoded
}

fn decode(encoded: &[u8]) -> Result<X509DirectoryString<'_>, X509DirectoryStringError> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder.decode_exact(encoded).unwrap();
    decode_x509_directory_string(element)
}

#[test]
fn decodes_all_five_directory_string_alternatives() {
    let cases = [
        (
            der(0x0c, "café".as_bytes()),
            X509DirectoryStringEncoding::Utf8,
        ),
        (
            der(0x13, b"Alpha 42"),
            X509DirectoryStringEncoding::Printable,
        ),
        (
            der(0x14, &[0x80, 0x41]),
            X509DirectoryStringEncoding::Teletex,
        ),
        (
            der(0x1c, &[0x00, 0x00, 0x00, 0x41, 0x00, 0x01, 0xf6, 0x00]),
            X509DirectoryStringEncoding::Universal,
        ),
        (
            der(0x1e, &[0x00, 0x41, 0x00, 0xe9]),
            X509DirectoryStringEncoding::Bmp,
        ),
    ];

    for (encoded, expected) in &cases {
        let decoded = decode(encoded).unwrap();
        assert_eq!(decoded.encoding(), *expected);
        assert_eq!(decoded.contents(), &encoded[2..]);
    }
}

#[test]
fn rejects_non_directory_constructed_and_empty_values() {
    assert_eq!(
        decode(&der(0x16, b"not-directory")).unwrap_err().kind(),
        X509DirectoryStringErrorKind::UnexpectedTag
    );
    assert_eq!(
        decode(&der(0x2c, b"constructed")).unwrap_err().kind(),
        X509DirectoryStringErrorKind::UnexpectedTag
    );
    assert_eq!(
        decode(&der(0x8c, b"context-specific")).unwrap_err().kind(),
        X509DirectoryStringErrorKind::UnexpectedTag
    );

    for tag in [0x0c, 0x13, 0x14, 0x1c, 0x1e] {
        let error = decode(&der(tag, b"")).unwrap_err();
        assert_eq!(error.kind(), X509DirectoryStringErrorKind::EmptyValue);
        assert_eq!(error.offset(), 2);
    }
}

#[test]
fn validates_utf8_and_printable_string_contents_with_redacted_offsets() {
    let utf8 = decode(&der(0x0c, &[b'A', 0xff, b'B'])).unwrap_err();
    assert_eq!(utf8.kind(), X509DirectoryStringErrorKind::InvalidUtf8);
    assert_eq!(utf8.offset(), 3);

    let printable = decode(&der(0x13, b"OK@NO")).unwrap_err();
    assert_eq!(
        printable.kind(),
        X509DirectoryStringErrorKind::InvalidPrintableCharacter
    );
    assert_eq!(printable.offset(), 4);
    assert!(!printable.to_string().contains("OK@NO"));

    let allowed = b"AZaz09 '()+,-./:=?";
    assert_eq!(decode(&der(0x13, allowed)).unwrap().contents(), allowed);
}

#[test]
fn validates_universal_string_width_and_unicode_scalars() {
    let length = decode(&der(0x1c, &[0x00, 0x00, 0x41])).unwrap_err();
    assert_eq!(
        length.kind(),
        X509DirectoryStringErrorKind::InvalidUniversalLength
    );
    assert_eq!(length.offset(), 5);

    for invalid in [[0x00, 0x00, 0xd8, 0x00], [0x00, 0x11, 0x00, 0x00]] {
        let error = decode(&der(0x1c, &invalid)).unwrap_err();
        assert_eq!(
            error.kind(),
            X509DirectoryStringErrorKind::InvalidUniversalScalar
        );
        assert_eq!(error.offset(), 2);
    }

    let second_invalid = decode(&der(
        0x1c,
        &[0x00, 0x00, 0x00, 0x41, 0x00, 0x00, 0xd8, 0x00],
    ))
    .unwrap_err();
    assert_eq!(second_invalid.offset(), 6);
}

#[test]
fn validates_bmp_string_width_and_rejects_surrogates() {
    let length = decode(&der(0x1e, &[0x00])).unwrap_err();
    assert_eq!(
        length.kind(),
        X509DirectoryStringErrorKind::InvalidBmpLength
    );
    assert_eq!(length.offset(), 3);

    let surrogate = decode(&der(0x1e, &[0xd8, 0x00])).unwrap_err();
    assert_eq!(
        surrogate.kind(),
        X509DirectoryStringErrorKind::InvalidBmpScalar
    );
    assert_eq!(surrogate.offset(), 2);
}

#[test]
fn iterates_and_compares_exact_scalars_across_supported_encodings() {
    let utf8_encoding = der(0x0c, "Aé".as_bytes());
    let universal_encoding = der(0x1c, &[0x00, 0x00, 0x00, 0x41, 0x00, 0x00, 0x00, 0xe9]);
    let bmp_encoding = der(0x1e, &[0x00, 0x41, 0x00, 0xe9]);
    let utf8 = decode(&utf8_encoding).unwrap();
    let universal = decode(&universal_encoding).unwrap();
    let bmp = decode(&bmp_encoding).unwrap();

    assert_eq!(utf8.unicode_scalars().unwrap().collect::<String>(), "Aé");
    assert_eq!(
        compare_x509_directory_strings_by_scalar(utf8, universal),
        Ok(true)
    );
    assert_eq!(
        compare_x509_directory_strings_by_scalar(universal, bmp),
        Ok(true)
    );

    let printable_encoding = der(0x13, b"Alpha");
    let utf8_alpha_encoding = der(0x0c, b"Alpha");
    assert_eq!(
        compare_x509_directory_strings_by_scalar(
            decode(&printable_encoding).unwrap(),
            decode(&utf8_alpha_encoding).unwrap(),
        ),
        Ok(true)
    );
}

#[test]
fn exact_scalar_comparison_is_case_and_space_sensitive() {
    let upper_encoding = der(0x0c, b"Alpha  Beta");
    let lower_encoding = der(0x0c, b"alpha beta");
    assert_eq!(
        compare_x509_directory_strings_by_scalar(
            decode(&upper_encoding).unwrap(),
            decode(&lower_encoding).unwrap(),
        ),
        Ok(false)
    );

    let short_encoding = der(0x13, b"A");
    let long_encoding = der(0x0c, b"AB");
    assert_eq!(
        compare_x509_directory_strings_by_scalar(
            decode(&short_encoding).unwrap(),
            decode(&long_encoding).unwrap(),
        ),
        Ok(false)
    );
}

#[test]
fn refuses_teletex_comparison_without_inventing_a_mapping() {
    let teletex_encoding = der(0x14, b"Alpha");
    let utf8_encoding = der(0x0c, b"Alpha");
    let teletex = decode(&teletex_encoding).unwrap();
    assert!(teletex.unicode_scalars().is_none());
    assert_eq!(
        compare_x509_directory_strings_by_scalar(teletex, decode(&utf8_encoding).unwrap()),
        Err(X509DirectoryStringComparisonError::UnsupportedTeletexString)
    );
    assert_eq!(
        compare_x509_directory_strings_by_scalar(teletex, teletex),
        Err(X509DirectoryStringComparisonError::UnsupportedTeletexString)
    );
    assert_eq!(
        X509DirectoryStringComparisonError::UnsupportedTeletexString.to_string(),
        "TeletexString has no configured character mapping"
    );
}

#[test]
fn handles_supplementary_universal_scalars_without_allocation() {
    let universal_encoding = der(0x1c, &[0x00, 0x01, 0xf6, 0x00]);
    let utf8_encoding = der(0x0c, "😀".as_bytes());
    assert_eq!(
        compare_x509_directory_strings_by_scalar(
            decode(&universal_encoding).unwrap(),
            decode(&utf8_encoding).unwrap(),
        ),
        Ok(true)
    );
}
