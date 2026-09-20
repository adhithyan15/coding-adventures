use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use x509_name::{
    decode_x509_name, X509AttributeTypeAndValueErrorKind, X509Name, X509NameError,
    X509NameErrorKind, MAX_X509_NAME_ATTRIBUTES, MAX_X509_NAME_RDNS,
};

const COMMON_NAME: &[u8] = &[0x55, 0x04, 0x03];
const COUNTRY_NAME: &[u8] = &[0x55, 0x04, 0x06];

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

fn attribute(oid: &[u8], value_tag: u8, value: &[u8]) -> Vec<u8> {
    let mut fields = der(0x06, oid);
    fields.extend(der(value_tag, value));
    der(0x30, &fields)
}

fn rdn(attributes: &[Vec<u8>]) -> Vec<u8> {
    der(0x31, &attributes.concat())
}

fn name(rdns: &[Vec<u8>]) -> Vec<u8> {
    der(0x30, &rdns.concat())
}

fn decode(encoded: &[u8]) -> Result<X509Name<'_>, X509NameError> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder
        .decode_exact(encoded)
        .expect("valid outer DER framing");
    decode_x509_name(&mut decoder, root)
}

#[test]
fn decodes_empty_and_multivalued_names_without_interpreting_values() {
    let empty_encoding = name(&[]);
    let empty = decode(&empty_encoding).unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.rdn_count(), 0);
    assert_eq!(empty.attribute_count(), 0);
    assert_eq!(empty.encoded(), empty_encoding);

    let common_name = attribute(COMMON_NAME, 0x0c, b"A");
    let country = attribute(COUNTRY_NAME, 0x13, b"US");
    assert!(common_name < country, "fixture must already be DER ordered");
    let encoded = name(&[rdn(&[common_name, country])]);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded.rdn_count(), 1);
    assert_eq!(decoded.attribute_count(), 2);

    let first_rdn = decoded.get_rdn(0).unwrap();
    assert_eq!(first_rdn.len(), 2);
    assert!(!first_rdn.is_empty());
    assert!(first_rdn
        .get(0)
        .unwrap()
        .attribute_type()
        .equals(&[2, 5, 4, 3]));
    assert_eq!(first_rdn.get(0).unwrap().value().value(), b"A");
    assert_eq!(first_rdn.get(0).unwrap().value().tag().number, 12);
    assert!(!first_rdn.get(0).unwrap().value().tag().constructed);
    assert!(first_rdn
        .get(1)
        .unwrap()
        .attribute_type()
        .equals(&[2, 5, 4, 6]));
    assert_eq!(first_rdn.iter().count(), 2);
    assert_eq!(decoded.rdns().count(), 1);
    assert!(decoded.get_rdn(1).is_none());
}

#[test]
fn keeps_rdn_sequence_boundaries() {
    let encoded = name(&[
        rdn(&[attribute(COUNTRY_NAME, 0x13, b"US")]),
        rdn(&[attribute(COMMON_NAME, 0x0c, b"service.example")]),
    ]);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded.rdn_count(), 2);
    assert_eq!(decoded.attribute_count(), 2);
    assert!(decoded
        .get_rdn(0)
        .unwrap()
        .get(0)
        .unwrap()
        .attribute_type()
        .equals(&[2, 5, 4, 6]));
    assert!(decoded
        .get_rdn(1)
        .unwrap()
        .get(0)
        .unwrap()
        .attribute_type()
        .equals(&[2, 5, 4, 3]));
}

#[test]
fn requires_name_sequence_and_rdn_sets() {
    let not_a_sequence = der(0x31, &[]);
    let error = decode(&not_a_sequence).unwrap_err();
    assert_eq!(
        error.kind(),
        X509NameErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
    );
    assert_eq!(error.offset(), 0);

    let attribute = attribute(COMMON_NAME, 0x0c, b"A");
    let encoded = name(&[der(0x30, &attribute)]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        X509NameErrorKind::Structure(Asn1ErrorKind::UnexpectedTag)
    );
    assert_eq!(error.offset(), 2);
}

#[test]
fn rejects_empty_and_unsorted_rdns() {
    let encoded = name(&[rdn(&[])]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        X509NameErrorKind::EmptyRelativeDistinguishedName
    );
    assert_eq!(error.offset(), 4);

    let common_name = attribute(COMMON_NAME, 0x0c, b"A");
    let country = attribute(COUNTRY_NAME, 0x13, b"US");
    assert!(country > common_name, "fixture must start out of order");
    let encoded = name(&[rdn(&[country, common_name])]);
    let error = decode(&encoded).unwrap_err();
    assert_eq!(
        error.kind(),
        X509NameErrorKind::UnsortedRelativeDistinguishedName
    );
    assert!(error.offset() > 4);
}

#[test]
fn attribute_requires_exact_type_and_value_fields() {
    let missing_type = name(&[rdn(&[der(0x30, &[])])]);
    assert_eq!(
        decode(&missing_type).unwrap_err().kind(),
        X509NameErrorKind::InvalidAttribute(X509AttributeTypeAndValueErrorKind::MissingType)
    );

    let invalid_type = name(&[rdn(&[der(0x30, &der(0x05, &[]))])]);
    assert_eq!(
        decode(&invalid_type).unwrap_err().kind(),
        X509NameErrorKind::InvalidAttribute(X509AttributeTypeAndValueErrorKind::InvalidType(
            Asn1ErrorKind::UnexpectedTag
        ))
    );

    let mut non_minimal_oid_fields = der(0x06, &[0x80]);
    non_minimal_oid_fields.extend(der(0x0c, b"A"));
    let non_minimal_oid = name(&[rdn(&[der(0x30, &non_minimal_oid_fields)])]);
    assert_eq!(
        decode(&non_minimal_oid).unwrap_err().kind(),
        X509NameErrorKind::InvalidAttribute(X509AttributeTypeAndValueErrorKind::InvalidType(
            Asn1ErrorKind::NonMinimalObjectIdentifier
        ))
    );

    let missing_value = name(&[rdn(&[der(0x30, &der(0x06, COMMON_NAME))])]);
    assert_eq!(
        decode(&missing_value).unwrap_err().kind(),
        X509NameErrorKind::InvalidAttribute(X509AttributeTypeAndValueErrorKind::MissingValue)
    );

    let mut fields = der(0x06, COMMON_NAME);
    fields.extend(der(0x0c, b"A"));
    fields.extend(der(0x05, &[]));
    let trailing = name(&[rdn(&[der(0x30, &fields)])]);
    assert_eq!(
        decode(&trailing).unwrap_err().kind(),
        X509NameErrorKind::InvalidAttribute(X509AttributeTypeAndValueErrorKind::TrailingElement)
    );
}

#[test]
fn child_framing_failures_are_redacted_and_located() {
    let malformed_rdn = [0x30, 0x03, 0x31, 0x02, 0x30];
    let error = decode(&malformed_rdn).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509NameErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));
    assert!(error.offset() >= 2);

    let malformed_attribute = [0x30, 0x05, 0x31, 0x03, 0x30, 0x02, 0x06];
    let error = decode(&malformed_attribute).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509NameErrorKind::Structure(Asn1ErrorKind::Framing(_))
    ));

    let malformed_type = name(&[rdn(&[der(0x30, &[0x06, 0x02, 0x55])])]);
    let error = decode(&malformed_type).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509NameErrorKind::InvalidAttribute(X509AttributeTypeAndValueErrorKind::Structure(
            Asn1ErrorKind::Framing(_)
        ))
    ));

    let mut malformed_trailing_fields = der(0x06, COMMON_NAME);
    malformed_trailing_fields.extend(der(0x0c, b"A"));
    malformed_trailing_fields.extend([0x05, 0x01]);
    let malformed_trailing = name(&[rdn(&[der(0x30, &malformed_trailing_fields)])]);
    let error = decode(&malformed_trailing).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509NameErrorKind::InvalidAttribute(X509AttributeTypeAndValueErrorKind::Structure(
            Asn1ErrorKind::Framing(_)
        ))
    ));

    let rendered = error.to_string();
    assert!(!rendered.contains("secret"));
    assert!(rendered.contains("byte"));
}

#[test]
fn explicit_rdn_and_total_attribute_bounds_are_enforced() {
    let one_attribute_rdn = rdn(&[attribute(COMMON_NAME, 0x0c, b"A")]);
    let encoded = name(&vec![one_attribute_rdn; MAX_X509_NAME_RDNS + 1]);
    assert_eq!(
        decode(&encoded).unwrap_err().kind(),
        X509NameErrorKind::TooManyRelativeDistinguishedNames
    );

    let attribute = attribute(COMMON_NAME, 0x0c, b"A");
    let encoded = name(&[rdn(&vec![attribute; MAX_X509_NAME_ATTRIBUTES + 1])]);
    assert_eq!(
        decode(&encoded).unwrap_err().kind(),
        X509NameErrorKind::TooManyAttributes
    );
}

#[test]
fn shared_depth_and_element_limits_are_enforced() {
    let encoded = name(&[rdn(&[attribute(COMMON_NAME, 0x0c, b"A")])]);

    let limits = Asn1Limits {
        max_depth: 3,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&encoded).unwrap();
    assert_eq!(
        decode_x509_name(&mut decoder, root).unwrap_err().kind(),
        X509NameErrorKind::InvalidAttribute(X509AttributeTypeAndValueErrorKind::Structure(
            Asn1ErrorKind::DepthLimitExceeded
        ))
    );

    for (max_total_elements, expected) in [
        (
            1,
            X509NameErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded),
        ),
        (
            2,
            X509NameErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded),
        ),
        (
            3,
            X509NameErrorKind::InvalidAttribute(X509AttributeTypeAndValueErrorKind::Structure(
                Asn1ErrorKind::ElementLimitExceeded,
            )),
        ),
        (
            4,
            X509NameErrorKind::InvalidAttribute(X509AttributeTypeAndValueErrorKind::Structure(
                Asn1ErrorKind::ElementLimitExceeded,
            )),
        ),
    ] {
        let limits = Asn1Limits {
            max_total_elements,
            ..Asn1Limits::default()
        };
        let mut decoder = Asn1Decoder::new(limits);
        let root = decoder.decode_exact(&encoded).unwrap();
        assert_eq!(
            decode_x509_name(&mut decoder, root).unwrap_err().kind(),
            expected
        );
    }
}

#[test]
fn errors_never_render_attribute_bytes() {
    let mut fields = der(0x06, COMMON_NAME);
    fields.extend(der(0x0c, b"secret-name"));
    fields.extend(der(0x05, &[]));
    let encoded = name(&[rdn(&[der(0x30, &fields)])]);
    let rendered = decode(&encoded).unwrap_err().to_string();
    assert!(!rendered.contains("secret-name"));
    assert!(!rendered.contains("55, 4, 3"));
}
