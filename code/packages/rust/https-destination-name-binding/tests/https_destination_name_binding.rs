use der_asn1::Asn1Limits;
use http1_request_header_field::{
    validate_http1_request_header_field, Http1RequestHeaderFieldLimits,
};
use http1_request_host_authority::{
    validate_http1_request_host_authority, Http1RequestHostAuthorityLimits,
};
use https_destination_name_binding::{
    bind_https_dns_destination, bind_https_host_header_field, bind_https_request_header_fields,
    HttpsDestinationNameBindingError, HttpsHostHeaderBindingError,
    HttpsRequestHeaderFieldsBindingError, HttpsRequestHeaderFieldsLimits,
};
use tls13_server_name::TlsServerName;
use tls_server_identity::MatchKind;
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::X509CertificateServerIdentityErrorKind;

const SIGNATURE_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const KEY_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
const SUBJECT_ALT_NAME_OID: &[u8] = &[0x55, 0x1d, 0x11];

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

fn name(common_name: &[u8]) -> Vec<u8> {
    let mut attribute = der(0x06, &[0x55, 0x04, 0x03]);
    attribute.extend(der(0x0c, common_name));
    der(0x30, &der(0x31, &der(0x30, &attribute)))
}

fn extension(oid: &[u8], value: &[u8]) -> Vec<u8> {
    let mut fields = der(0x06, oid);
    fields.extend(der(0x04, value));
    der(0x30, &fields)
}

fn subject_alt_name(dns_names: &[&str]) -> Vec<u8> {
    let names = dns_names
        .iter()
        .flat_map(|name| der(0x82, name.as_bytes()))
        .collect::<Vec<_>>();
    extension(SUBJECT_ALT_NAME_OID, &der(0x30, &names))
}

fn certificate(extensions: &[Vec<u8>]) -> Vec<u8> {
    let mut validity = der(0x17, b"260101000000Z");
    validity.extend(der(0x17, b"270101000000Z"));

    let mut subject_public_key_info = algorithm(KEY_OID);
    subject_public_key_info.extend(der(0x03, &[0x00, 0x30, 0x00]));

    let mut tbs = der(0xa0, &der(0x02, &[0x02]));
    tbs.extend(der(0x02, &[0x01]));
    tbs.extend(algorithm(SIGNATURE_OID));
    tbs.extend(name(b"issuer.example"));
    tbs.extend(der(0x30, &validity));
    tbs.extend(name(b"wrong.example"));
    tbs.extend(der(0x30, &subject_public_key_info));
    if !extensions.is_empty() {
        tbs.extend(der(0xa3, &der(0x30, &extensions.concat())));
    }

    let mut fields = der(0x30, &tbs);
    fields.extend(algorithm(SIGNATURE_OID));
    fields.extend(der(0x03, &[0x00, 0xde, 0xad]));
    der(0x30, &fields)
}

fn host_limits() -> Http1RequestHostAuthorityLimits {
    Http1RequestHostAuthorityLimits::new(253, 259).unwrap()
}

fn header_limits() -> Http1RequestHeaderFieldLimits {
    Http1RequestHeaderFieldLimits::new(64, 259, 327).unwrap()
}

fn request_fields_limits() -> HttpsRequestHeaderFieldsLimits {
    HttpsRequestHeaderFieldsLimits::new(16).unwrap()
}

#[test]
fn binds_exact_certificate_sni_and_host_case_insensitively() {
    let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("API.Example.Test").unwrap();
    let host =
        validate_http1_request_host_authority(b"api.example.test", Some(8443), host_limits())
            .unwrap();

    let bound =
        bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default()).unwrap();

    assert!(core::ptr::eq(bound.certificate(), &certificate));
    assert_eq!(bound.server_name().as_str(), "API.Example.Test");
    assert_eq!(bound.host_authority().host(), b"api.example.test");
    assert_eq!(bound.host_authority().port(), Some(8443));
    assert_eq!(bound.certificate_identity().extension_index(), 0);
    assert_eq!(
        bound.certificate_identity().match_kind(),
        MatchKind::ExactDns
    );
}

#[test]
fn accepts_certificate_wildcard_only_through_san_policy() {
    let encoded = certificate(&[subject_alt_name(&["*.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("mail.example.test").unwrap();
    let host =
        validate_http1_request_host_authority(b"MAIL.EXAMPLE.TEST", None, host_limits()).unwrap();

    let bound =
        bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default()).unwrap();

    assert_eq!(
        bound.certificate_identity().match_kind(),
        MatchKind::WildcardDns
    );
}

#[test]
fn rejects_sni_and_host_mismatch_before_certificate_identity() {
    let encoded = certificate(&[]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("api.example.test").unwrap();
    let host =
        validate_http1_request_host_authority(b"other.example.test", None, host_limits()).unwrap();

    let error = bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default())
        .unwrap_err();

    assert_eq!(
        error,
        HttpsDestinationNameBindingError::ServerNameHostMismatch
    );
    assert!(!error.to_string().contains("api.example.test"));
    assert!(!error.to_string().contains("other.example.test"));
}

#[test]
fn rejects_certificate_san_mismatch_without_disclosing_names() {
    let encoded = certificate(&[subject_alt_name(&["secret.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("api.example.test").unwrap();
    let host =
        validate_http1_request_host_authority(b"api.example.test", None, host_limits()).unwrap();

    let error = bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default())
        .unwrap_err();

    let HttpsDestinationNameBindingError::CertificateIdentity(identity) = error else {
        panic!("expected certificate identity failure");
    };
    assert_eq!(
        identity.kind(),
        X509CertificateServerIdentityErrorKind::IdentityMismatch
    );
    assert_eq!(identity.extension_index(), Some(0));
    assert!(!error.to_string().contains("api.example.test"));
    assert!(!error.to_string().contains("secret.example.test"));
}

#[test]
fn applies_the_caller_san_decoder_budget() {
    let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("api.example.test").unwrap();
    let host =
        validate_http1_request_host_authority(b"api.example.test", None, host_limits()).unwrap();
    let limits = Asn1Limits {
        max_total_elements: 1,
        ..Asn1Limits::default()
    };

    let error = bind_https_dns_destination(&certificate, server_name, host, limits).unwrap_err();

    let HttpsDestinationNameBindingError::CertificateIdentity(identity) = error else {
        panic!("expected certificate identity failure");
    };
    assert_eq!(
        identity.kind(),
        X509CertificateServerIdentityErrorKind::InvalidSubjectAlternativeName
    );
}

#[test]
fn debug_output_redacts_all_identity_inputs() {
    let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("api.example.test").unwrap();
    let host = validate_http1_request_host_authority(b"api.example.test", Some(443), host_limits())
        .unwrap();
    let bound =
        bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default()).unwrap();

    let debug = format!("{bound:?}");
    assert!(!debug.contains("api.example.test"));
    assert!(!debug.contains("issuer.example"));
    assert!(debug.matches("[REDACTED]").count() >= 3);
}

#[test]
fn binds_destination_to_one_canonical_host_field() {
    let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("API.Example.Test").unwrap();
    let host =
        validate_http1_request_host_authority(b"API.Example.Test", Some(8443), host_limits())
            .unwrap();
    let destination =
        bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default()).unwrap();
    let field =
        validate_http1_request_header_field(b"host", b"api.example.test:8443", header_limits())
            .unwrap();

    let bound = bind_https_host_header_field(destination, field).unwrap();

    assert!(core::ptr::eq(
        bound.destination().certificate(),
        &certificate
    ));
    assert_eq!(
        bound.destination().server_name().as_str(),
        "API.Example.Test"
    );
    assert_eq!(bound.host_field().name(), b"host");
    assert_eq!(bound.host_field().value(), b"api.example.test:8443");
}

#[test]
fn rejects_a_non_host_field_without_disclosing_it() {
    let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("api.example.test").unwrap();
    let host =
        validate_http1_request_host_authority(b"api.example.test", None, host_limits()).unwrap();
    let destination =
        bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default()).unwrap();
    let field =
        validate_http1_request_header_field(b"X-Secret", b"hidden", header_limits()).unwrap();

    let error = bind_https_host_header_field(destination, field).unwrap_err();

    assert_eq!(error, HttpsHostHeaderBindingError::NotHostField);
    assert!(!error.to_string().contains("X-Secret"));
    assert!(!error.to_string().contains("hidden"));
}

#[test]
fn rejects_noncanonical_or_substituted_host_authorities() {
    for value in [
        b"API.example.test:8443".as_slice(),
        b"api.example.test:08443".as_slice(),
        b"other.example.test:8443".as_slice(),
        b"api.example.test:443".as_slice(),
    ] {
        let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let server_name = TlsServerName::new("api.example.test").unwrap();
        let host =
            validate_http1_request_host_authority(b"api.example.test", Some(8443), host_limits())
                .unwrap();
        let destination =
            bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default())
                .unwrap();
        let field = validate_http1_request_header_field(b"Host", value, header_limits()).unwrap();

        let error = bind_https_host_header_field(destination, field).unwrap_err();

        assert_eq!(error, HttpsHostHeaderBindingError::HostAuthorityMismatch);
        assert!(!error.to_string().contains("api.example.test"));
        assert!(!error.to_string().contains("other.example.test"));
    }
}

#[test]
fn bound_host_field_debug_output_redacts_destination_and_field() {
    let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("api.example.test").unwrap();
    let host = validate_http1_request_host_authority(b"api.example.test", Some(443), host_limits())
        .unwrap();
    let destination =
        bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default()).unwrap();
    let field =
        validate_http1_request_header_field(b"Host", b"api.example.test:443", header_limits())
            .unwrap();

    let bound = bind_https_host_header_field(destination, field).unwrap();
    let debug = format!("{bound:?}");

    assert!(!debug.contains("api.example.test"));
    assert!(!debug.contains("issuer.example"));
    assert!(!debug.contains(":443"));
    assert!(debug.matches("[REDACTED]").count() >= 2);
}

#[test]
fn binds_the_exact_additional_request_field_slice() {
    let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("api.example.test").unwrap();
    let host =
        validate_http1_request_host_authority(b"api.example.test", None, host_limits()).unwrap();
    let destination =
        bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default()).unwrap();
    let host_field =
        validate_http1_request_header_field(b"Host", b"api.example.test", header_limits()).unwrap();
    let host_binding = bind_https_host_header_field(destination, host_field).unwrap();
    let additional_fields = [
        validate_http1_request_header_field(b"Accept", b"application/json", header_limits())
            .unwrap(),
        validate_http1_request_header_field(b"X-Trace", b"opaque", header_limits()).unwrap(),
        validate_http1_request_header_field(b"Hosting", b"allowed", header_limits()).unwrap(),
    ];

    let bound =
        bind_https_request_header_fields(host_binding, &additional_fields, request_fields_limits())
            .unwrap();

    assert!(core::ptr::eq(
        bound.host_binding().destination().certificate(),
        &certificate
    ));
    assert_eq!(bound.host_binding().host_field().name(), b"Host");
    assert!(core::ptr::eq(
        bound.additional_fields(),
        additional_fields.as_slice()
    ));
}

#[test]
fn accepts_an_empty_additional_request_field_slice() {
    let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("api.example.test").unwrap();
    let host =
        validate_http1_request_host_authority(b"api.example.test", None, host_limits()).unwrap();
    let destination =
        bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default()).unwrap();
    let host_field =
        validate_http1_request_header_field(b"host", b"api.example.test", header_limits()).unwrap();
    let host_binding = bind_https_host_header_field(destination, host_field).unwrap();
    let additional_fields = [];

    let bound =
        bind_https_request_header_fields(host_binding, &additional_fields, request_fields_limits())
            .unwrap();

    assert!(bound.additional_fields().is_empty());
}

#[test]
fn rejects_every_additional_host_field_case_insensitively() {
    for name in [b"Host".as_slice(), b"HOST".as_slice(), b"hOsT".as_slice()] {
        let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let server_name = TlsServerName::new("api.example.test").unwrap();
        let host = validate_http1_request_host_authority(b"api.example.test", None, host_limits())
            .unwrap();
        let destination =
            bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default())
                .unwrap();
        let host_field =
            validate_http1_request_header_field(b"Host", b"api.example.test", header_limits())
                .unwrap();
        let host_binding = bind_https_host_header_field(destination, host_field).unwrap();
        let additional_fields = [
            validate_http1_request_header_field(b"Accept", b"text/plain", header_limits()).unwrap(),
            validate_http1_request_header_field(name, b"secret.example.test", header_limits())
                .unwrap(),
        ];

        let error = bind_https_request_header_fields(
            host_binding,
            &additional_fields,
            request_fields_limits(),
        )
        .unwrap_err();

        assert_eq!(
            error,
            HttpsRequestHeaderFieldsBindingError::AdditionalHostField
        );
        assert!(!error.to_string().contains("secret.example.test"));
        assert!(!format!("{error:?}").contains("secret.example.test"));
    }
}

#[test]
fn bound_request_fields_debug_output_redacts_all_fields() {
    let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("api.example.test").unwrap();
    let host =
        validate_http1_request_host_authority(b"api.example.test", None, host_limits()).unwrap();
    let destination =
        bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default()).unwrap();
    let host_field =
        validate_http1_request_header_field(b"Host", b"api.example.test", header_limits()).unwrap();
    let host_binding = bind_https_host_header_field(destination, host_field).unwrap();
    let additional_fields =
        [
            validate_http1_request_header_field(
                b"Authorization",
                b"Bearer secret",
                header_limits(),
            )
            .unwrap(),
        ];

    let bound =
        bind_https_request_header_fields(host_binding, &additional_fields, request_fields_limits())
            .unwrap();
    let debug = format!("{bound:?}");

    assert!(!debug.contains("api.example.test"));
    assert!(!debug.contains("Authorization"));
    assert!(!debug.contains("Bearer secret"));
    assert!(debug.matches("[REDACTED]").count() >= 2);
}

#[test]
fn rejects_invalid_or_exceeded_request_field_limits_without_observed_counts() {
    assert_eq!(
        HttpsRequestHeaderFieldsLimits::new(64)
            .unwrap()
            .maximum_fields(),
        64
    );
    assert_eq!(
        HttpsRequestHeaderFieldsLimits::new(0),
        Err(HttpsRequestHeaderFieldsBindingError::InvalidLimits)
    );
    assert_eq!(
        HttpsRequestHeaderFieldsLimits::new(65),
        Err(HttpsRequestHeaderFieldsBindingError::InvalidLimits)
    );

    let encoded = certificate(&[subject_alt_name(&["api.example.test"])]);
    let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
    let server_name = TlsServerName::new("api.example.test").unwrap();
    let host =
        validate_http1_request_host_authority(b"api.example.test", None, host_limits()).unwrap();
    let destination =
        bind_https_dns_destination(&certificate, server_name, host, Asn1Limits::default()).unwrap();
    let host_field =
        validate_http1_request_header_field(b"Host", b"api.example.test", header_limits()).unwrap();
    let host_binding = bind_https_host_header_field(destination, host_field).unwrap();
    let additional_fields =
        [validate_http1_request_header_field(b"Accept", b"text/plain", header_limits()).unwrap()];

    let error = bind_https_request_header_fields(
        host_binding,
        &additional_fields,
        HttpsRequestHeaderFieldsLimits::new(1).unwrap(),
    )
    .unwrap_err();

    assert_eq!(error, HttpsRequestHeaderFieldsBindingError::TooManyFields);
    assert!(!error.to_string().contains('2'));
    assert!(!format!("{error:?}").contains('2'));
}
