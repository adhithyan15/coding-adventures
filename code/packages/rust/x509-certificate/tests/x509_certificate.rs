use der_asn1::{Asn1Decoder, Asn1ErrorKind, Asn1Limits};
use der_tlv::DerErrorKind;
use x509_certificate::{
    decode_x509_certificate, decode_x509_certificate_element, X509CertificateErrorKind,
    X509TbsCertificateErrorKind,
};
use x509_certificate_version::X509CertificateVersion;

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];

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

fn algorithm(oid: &[u8]) -> Vec<u8> {
    let mut fields = der(0x06, oid);
    fields.extend(der(0x05, &[]));
    der(0x30, &fields)
}

fn validity() -> Vec<u8> {
    let mut fields = der(0x17, b"260101000000Z");
    fields.extend(der(0x17, b"270101000000Z"));
    der(0x30, &fields)
}

fn subject_public_key_info() -> Vec<u8> {
    let mut fields = algorithm(KEY_OID);
    fields.extend(der(0x03, &[0x00, 0x30, 0x00]));
    der(0x30, &fields)
}

fn name() -> Vec<u8> {
    let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
    attribute.extend(der(0x0c, b"example"));
    der(0x30, &der(0x31, &der(0x30, &attribute)))
}

fn extensions() -> Vec<u8> {
    let mut extension_fields = der(0x06, &[0x55, 0x1d, 0x13]);
    extension_fields.extend(der(0x04, &[0x30, 0x00]));
    let extension = der(0x30, &extension_fields);
    der(0xa3, &der(0x30, &extension))
}

fn tbs(version: Option<u8>, optional_fields: &[Vec<u8>]) -> Vec<u8> {
    tbs_with_issuer(version, optional_fields, name())
}

fn tbs_with_issuer(version: Option<u8>, optional_fields: &[Vec<u8>], issuer: Vec<u8>) -> Vec<u8> {
    let mut fields = Vec::new();
    if let Some(version) = version {
        fields.extend(der(0xa0, &der(0x02, &[version])));
    }
    fields.extend(der(0x02, &[0x01]));
    fields.extend(algorithm(SIGNATURE_OID));
    fields.extend(issuer);
    fields.extend(validity());
    fields.extend(name());
    fields.extend(subject_public_key_info());
    for field in optional_fields {
        fields.extend(field);
    }
    der(0x30, &fields)
}

fn certificate_with(tbs: Vec<u8>, outer_algorithm: Vec<u8>) -> Vec<u8> {
    let mut fields = tbs;
    fields.extend(outer_algorithm);
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
}

fn certificate(version: Option<u8>, optional_fields: &[Vec<u8>]) -> Vec<u8> {
    certificate_with(tbs(version, optional_fields), algorithm(SIGNATURE_OID))
}

#[test]
fn decodes_complete_v3_certificate_and_preserves_signed_bytes() {
    let issuer_id = der(0x81, &[0x00, 0xaa]);
    let subject_id = der(0x82, &[0x04, 0xb0]);
    let encoded = certificate(Some(2), &[issuer_id, subject_id, extensions()]);
    let decoded = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();

    assert_eq!(decoded.encoded(), encoded);
    assert_eq!(
        decoded.tbs_certificate().version(),
        X509CertificateVersion::V3
    );
    assert_eq!(decoded.tbs_certificate().serial_number().magnitude(), &[1]);
    assert!(decoded
        .tbs_certificate()
        .signature_algorithm()
        .algorithm()
        .equals(&[1, 2, 840, 113549, 1, 1, 11]));
    assert_eq!(decoded.tbs_certificate().issuer().rdn_count(), 1);
    assert_eq!(decoded.tbs_certificate().subject().rdn_count(), 1);
    assert_eq!(
        decoded
            .tbs_certificate()
            .subject_public_key_info()
            .subject_public_key()
            .bytes(),
        &[0x30, 0x00]
    );
    assert_eq!(
        decoded
            .tbs_certificate()
            .issuer_unique_id()
            .unwrap()
            .bytes(),
        &[0xaa]
    );
    assert_eq!(
        decoded
            .tbs_certificate()
            .subject_unique_id()
            .unwrap()
            .bit_len(),
        4
    );
    assert_eq!(decoded.tbs_certificate().extensions().unwrap().len(), 1);
    assert_eq!(decoded.signature_value().bytes(), &[0xde, 0xad]);

    let signed = decoded.tbs_certificate().encoded();
    assert_eq!(signed[0], 0x30);
    assert!(encoded.windows(signed.len()).any(|window| window == signed));
}

#[test]
fn omitted_version_decodes_as_v1_without_optional_fields() {
    let encoded = certificate(None, &[]);
    let decoded = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    assert_eq!(
        decoded.tbs_certificate().version(),
        X509CertificateVersion::V1
    );
    assert!(decoded.tbs_certificate().issuer_unique_id().is_none());
    assert!(decoded.tbs_certificate().subject_unique_id().is_none());
    assert!(decoded.tbs_certificate().extensions().is_none());
}

#[test]
fn rejects_an_empty_issuer_name_at_the_certificate_boundary() {
    let encoded = certificate_with(
        tbs_with_issuer(Some(2), &[], der(0x30, &[])),
        algorithm(SIGNATURE_OID),
    );
    assert_eq!(
        decode_x509_certificate(&encoded, Asn1Limits::default())
            .unwrap_err()
            .kind(),
        X509CertificateErrorKind::InvalidTbsCertificate(X509TbsCertificateErrorKind::EmptyIssuer)
    );
}

#[test]
fn enforces_version_constraints_and_optional_field_order() {
    let v1_with_issuer_id = certificate(None, &[der(0x81, &[0x00, 0xaa])]);
    assert_eq!(
        decode_x509_certificate(&v1_with_issuer_id, Asn1Limits::default())
            .unwrap_err()
            .kind(),
        X509CertificateErrorKind::InvalidTbsCertificate(
            X509TbsCertificateErrorKind::VersionDoesNotPermitIssuerUniqueId
        )
    );

    let v2_with_extensions = certificate(Some(1), &[extensions()]);
    assert_eq!(
        decode_x509_certificate(&v2_with_extensions, Asn1Limits::default())
            .unwrap_err()
            .kind(),
        X509CertificateErrorKind::InvalidTbsCertificate(
            X509TbsCertificateErrorKind::VersionDoesNotPermitExtensions
        )
    );

    let out_of_order = certificate(
        Some(2),
        &[der(0x82, &[0x00, 0xbb]), der(0x81, &[0x00, 0xaa])],
    );
    assert_eq!(
        decode_x509_certificate(&out_of_order, Asn1Limits::default())
            .unwrap_err()
            .kind(),
        X509CertificateErrorKind::InvalidTbsCertificate(
            X509TbsCertificateErrorKind::UnexpectedOptionalField
        )
    );
}

#[test]
fn requires_exact_matching_signature_algorithms() {
    let mismatch = certificate_with(tbs(Some(2), &[]), algorithm(KEY_OID));
    let error = decode_x509_certificate(&mismatch, Asn1Limits::default()).unwrap_err();
    assert_eq!(
        error.kind(),
        X509CertificateErrorKind::SignatureAlgorithmMismatch
    );

    let mut alternate_parameters = der(0x06, SIGNATURE_OID);
    alternate_parameters.extend(der(0x04, &[]));
    let mismatch = certificate_with(tbs(Some(2), &[]), der(0x30, &alternate_parameters));
    assert_eq!(
        decode_x509_certificate(&mismatch, Asn1Limits::default())
            .unwrap_err()
            .kind(),
        X509CertificateErrorKind::SignatureAlgorithmMismatch
    );
}

#[test]
fn rejects_noncanonical_unique_ids_and_trailing_certificate_fields() {
    let invalid_id = certificate(Some(2), &[der(0x81, &[0x03, 0xa1])]);
    assert_eq!(
        decode_x509_certificate(&invalid_id, Asn1Limits::default())
            .unwrap_err()
            .kind(),
        X509CertificateErrorKind::InvalidTbsCertificate(
            X509TbsCertificateErrorKind::InvalidIssuerUniqueId(Asn1ErrorKind::NonZeroBitPadding)
        )
    );

    let valid_tbs = tbs(Some(2), &[]);
    let mut fields = valid_tbs;
    fields.extend(algorithm(SIGNATURE_OID));
    fields.extend(der(0x03, &[0x00, 0xde]));
    fields.extend(der(0x05, &[]));
    let trailing = der(0x30, &fields);
    assert_eq!(
        decode_x509_certificate(&trailing, Asn1Limits::default())
            .unwrap_err()
            .kind(),
        X509CertificateErrorKind::TrailingElement
    );
}

#[test]
fn framing_and_shared_budget_failures_are_redacted() {
    let malformed = [0x30, 0x03, 0x30, 0x02, 0x30];
    let error = decode_x509_certificate(&malformed, Asn1Limits::default()).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509CertificateErrorKind::Structure(Asn1ErrorKind::Framing(DerErrorKind::TruncatedValue))
            | X509CertificateErrorKind::InvalidTbsCertificate(
                X509TbsCertificateErrorKind::Structure(Asn1ErrorKind::Framing(
                    DerErrorKind::TruncatedValue
                ))
            )
    ));

    let encoded = certificate(Some(2), &[]);
    let limits = Asn1Limits {
        max_total_elements: 5,
        ..Asn1Limits::default()
    };
    let error = decode_x509_certificate(&encoded, limits).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509CertificateErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
            | X509CertificateErrorKind::InvalidTbsCertificate(
                X509TbsCertificateErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
            )
    ));
    assert!(!error.to_string().contains("dead"));
    assert!(error.to_string().contains("byte"));
}

#[test]
fn composable_entry_point_uses_the_callers_existing_decoder_budget() {
    let encoded_certificate = certificate(Some(2), &[]);
    let container = der(0x30, &encoded_certificate);
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(&container).unwrap();
    let mut values = decoder.sequence(root).unwrap();
    let certificate_element = values.read(&mut decoder).unwrap().unwrap();
    let decoded = decode_x509_certificate_element(&mut decoder, certificate_element).unwrap();
    assert_eq!(decoded.encoded(), encoded_certificate);
    assert!(values.read(&mut decoder).unwrap().is_none());

    let mut decoder = Asn1Decoder::new(Asn1Limits {
        max_total_elements: 2,
        ..Asn1Limits::default()
    });
    let root = decoder.decode_exact(&container).unwrap();
    let mut values = decoder.sequence(root).unwrap();
    let certificate_element = values.read(&mut decoder).unwrap().unwrap();
    let error = decode_x509_certificate_element(&mut decoder, certificate_element).unwrap_err();
    assert!(matches!(
        error.kind(),
        X509CertificateErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
            | X509CertificateErrorKind::InvalidTbsCertificate(
                X509TbsCertificateErrorKind::Structure(Asn1ErrorKind::ElementLimitExceeded)
            )
    ));
}
