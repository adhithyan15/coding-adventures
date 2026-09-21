use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_extension::X509ExtensionErrorKind;
use x509_extensions::{decode_x509_extensions, X509ExtensionsErrorKind, MAX_X509_EXTENSIONS};

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

fn extension(oid_last_arc: u8, critical: bool, value: &[u8]) -> Vec<u8> {
    let mut fields = element(0x06, &[0x55, 0x1d, oid_last_arc]);
    if critical {
        fields.extend_from_slice(&[0x01, 0x01, 0xff]);
    }
    fields.extend_from_slice(&element(0x04, value));
    element(0x30, &fields)
}

fn extensions(values: &[Vec<u8>]) -> Vec<u8> {
    let contents: Vec<u8> = values.iter().flatten().copied().collect();
    element(0x30, &contents)
}

fn decode(
    encoded: &[u8],
) -> Result<x509_extensions::X509Extensions<'_>, x509_extensions::X509ExtensionsError> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).expect("valid DER framing");
    decode_x509_extensions(&mut decoder, root)
}

#[test]
fn decodes_non_empty_extensions_in_wire_order() {
    let encoded = extensions(&[
        extension(15, true, &[0x03, 0x02, 0x05, 0xa0]),
        extension(17, false, &[0x30, 0x00]),
    ]);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded.len(), 2);
    assert!(!decoded.is_empty());

    let values: Vec<_> = decoded.iter().collect();
    assert!(values[0].extension_id().equals(&[2, 5, 29, 15]));
    assert!(values[0].critical());
    assert_eq!(values[0].extension_value(), [0x03, 0x02, 0x05, 0xa0]);
    assert!(values[1].extension_id().equals(&[2, 5, 29, 17]));
    assert!(!values[1].critical());
    assert_eq!(values[1].extension_value(), [0x30, 0x00]);
    assert_eq!(decoded.get(2), None);
}

#[test]
fn accepts_exact_capacity() {
    let values: Vec<_> = (0..MAX_X509_EXTENSIONS)
        .map(|index| extension(index as u8, false, &[]))
        .collect();
    let encoded = extensions(&values);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded.len(), MAX_X509_EXTENSIONS);
    assert!(decoded
        .get(MAX_X509_EXTENSIONS - 1)
        .unwrap()
        .extension_id()
        .equals(&[2, 5, 29, (MAX_X509_EXTENSIONS - 1) as u64]));
}

#[test]
fn rejects_empty_sequence() {
    let error = decode(&extensions(&[])).unwrap_err();
    assert_eq!(error.kind(), X509ExtensionsErrorKind::Empty);
    assert_eq!(error.offset(), 2);
}

#[test]
fn root_must_be_a_constructed_universal_sequence() {
    for encoded in [vec![0x10, 0], vec![0x31, 0]] {
        assert_eq!(
            decode(&encoded).unwrap_err().kind(),
            X509ExtensionsErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
        );
    }
}

#[test]
fn rejects_more_than_the_fixed_capacity() {
    let values: Vec<_> = (0..=MAX_X509_EXTENSIONS)
        .map(|index| extension(index as u8, false, &[]))
        .collect();
    let encoded = extensions(&values);
    let last_offset = encoded.len() - values.last().unwrap().len();
    let error = decode(&encoded).unwrap_err();
    assert_eq!(error.kind(), X509ExtensionsErrorKind::TooManyExtensions);
    assert_eq!(error.offset(), last_offset);
}

#[test]
fn rejects_duplicate_extension_identifiers() {
    let first = extension(17, false, &[0x30, 0x00]);
    let second = extension(17, true, &[0x30, 0x01, 0x00]);
    let second_offset = 2 + first.len();
    let encoded = extensions(&[first, second]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(error.kind(), X509ExtensionsErrorKind::DuplicateExtensionId);
    assert_eq!(error.offset(), second_offset);
}

#[test]
fn nested_extension_failures_are_translated_to_list_offsets() {
    let first = extension(15, false, &[]);
    let invalid = element(0x30, &element(0x05, &[]));
    let invalid_id_offset = 2 + first.len() + 2;
    let encoded = extensions(&[first, invalid]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        X509ExtensionsErrorKind::InvalidExtension(X509ExtensionErrorKind::InvalidExtensionId(
            Asn1ErrorKind::UnexpectedTag
        ))
    );
    assert_eq!(error.offset(), invalid_id_offset);
}

#[test]
fn malformed_child_reports_whole_list_offset() {
    let first = extension(15, false, &[]);
    let mut contents = first.clone();
    contents.extend_from_slice(&[0x30, 0x02, 0x06]);
    let encoded = element(0x30, &contents);
    let error = decode(&encoded).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509ExtensionsErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert_eq!(error.offset(), encoded.len());
}

#[test]
fn shared_depth_and_element_limits_are_enforced() {
    let encoded = extensions(&[extension(17, false, &[])]);

    let depth_limits = Asn1Limits {
        max_depth: 2,
        ..Asn1Limits::default()
    };
    let mut depth_decoder = Asn1Decoder::new(depth_limits);
    let root = depth_decoder.decode_exact(&encoded).unwrap();
    assert_eq!(
        decode_x509_extensions(&mut depth_decoder, root)
            .unwrap_err()
            .kind(),
        X509ExtensionsErrorKind::InvalidExtension(X509ExtensionErrorKind::Structure(
            Asn1ErrorKind::DepthLimitExceeded
        ))
    );

    let element_limits = Asn1Limits {
        max_total_elements: 3,
        ..Asn1Limits::default()
    };
    let mut element_decoder = Asn1Decoder::new(element_limits);
    let root = element_decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_extensions(&mut element_decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509ExtensionsErrorKind::InvalidExtension(X509ExtensionErrorKind::Structure(
            Asn1ErrorKind::ElementLimitExceeded
        ))
    );
    assert_eq!(error.offset(), 9);

    let outer_element_limits = Asn1Limits {
        max_total_elements: 1,
        ..Asn1Limits::default()
    };
    let mut outer_decoder = Asn1Decoder::new(outer_element_limits);
    let root = outer_decoder.decode_exact(&encoded).unwrap();
    let error = decode_x509_extensions(&mut outer_decoder, root).unwrap_err();
    assert_eq!(
        error.kind(),
        X509ExtensionsErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
    );
    assert_eq!(error.offset(), 2);
}

#[test]
fn errors_are_redacted() {
    let encoded = extensions(&[
        extension(17, false, &[0xde, 0xad]),
        extension(17, false, &[]),
    ]);
    let error = decode(&encoded).unwrap_err();
    assert!(!error.to_string().contains("dead"));
    assert!(!error.to_string().contains("551d11"));
    assert!(error.to_string().contains("byte"));
}
