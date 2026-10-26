use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_authority_information_access::{
    decode_authority_information_access, AuthorityInformationAccess, ID_AD_OCSP_OID,
};
use x509_extension::decode_x509_extension;
use x509_ocsp_aia_locator_policy::{
    select_unique_x509_ocsp_aia_locator, X509OcspAiaLocatorErrorKind, X509OcspAiaLocatorPolicy,
    X509OcspAiaLocatorPolicyErrorKind, X509OcspAiaSchemePolicy, MAX_ALLOWED_SCHEMES,
    MAX_EXPLICIT_PORTS_PER_SCHEME,
};

const AIA_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x01, 0x01];
const ID_AD_OCSP_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01];
const ID_AD_CA_ISSUERS_OID_DER: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x02];

fn element(tag: u8, value: &[u8]) -> Vec<u8> {
    assert!(value.len() < 128);
    let mut encoded = vec![tag, value.len() as u8];
    encoded.extend_from_slice(value);
    encoded
}

fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
    let contents: Vec<u8> = fields.iter().flatten().copied().collect();
    element(0x30, &contents)
}

fn description(method: &[u8], uri: &str) -> Vec<u8> {
    sequence(&[element(0x06, method), element(0x86, uri.as_bytes())])
}

fn decode_aia(descriptions: &[Vec<u8>]) -> AuthorityInformationAccess<'static> {
    let payload = sequence(descriptions);
    let mut fields = element(0x06, AIA_OID_DER);
    fields.extend_from_slice(&element(0x04, &payload));
    let encoded = Box::leak(element(0x30, &fields).into_boxed_slice());
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    let extension = decode_x509_extension(&mut decoder, root).unwrap();
    decode_authority_information_access(&mut decoder, extension).unwrap()
}

fn https_policy<'a>(schemes: &'a [X509OcspAiaSchemePolicy<'a>]) -> X509OcspAiaLocatorPolicy<'a> {
    X509OcspAiaLocatorPolicy::new(schemes, 512, 253, 256)
}

#[test]
fn selects_one_exact_ocsp_locator_and_retains_borrowed_components() {
    let aia = decode_aia(&[
        description(
            ID_AD_CA_ISSUERS_OID_DER,
            "http://ca.example.test/issuer.der",
        ),
        description(
            ID_AD_OCSP_OID_DER,
            "HTTPS://Ocsp.Example.test:8443/status/v1?nonce=a%2Fb",
        ),
    ]);
    let schemes = [X509OcspAiaSchemePolicy::new("https", 443, &[8443])];
    let policy = https_policy(&schemes);
    let selected = select_unique_x509_ocsp_aia_locator(&aia, &policy).unwrap();

    assert_eq!(selected.description_index(), 1);
    assert!(selected
        .authority_information_access()
        .get(1)
        .unwrap()
        .access_method()
        .equals(ID_AD_OCSP_OID));
    assert_eq!(selected.scheme_policy().scheme(), "https");
    assert_eq!(selected.scheme_policy().default_port(), 443);
    assert_eq!(selected.scheme_policy().explicit_ports(), &[8443]);
    assert_eq!(selected.host(), "Ocsp.Example.test");
    assert_eq!(selected.explicit_port(), Some(8443));
    assert_eq!(selected.effective_port(), 8443);
    assert_eq!(selected.path(), "/status/v1");
    assert_eq!(selected.query(), Some("nonce=a%2Fb"));
    assert_eq!(selected.path_and_query(), "/status/v1?nonce=a%2Fb");
    assert_eq!(
        selected.uri(),
        "HTTPS://Ocsp.Example.test:8443/status/v1?nonce=a%2Fb"
    );
    assert_eq!(format!("{selected:?}"), "SelectedX509OcspAiaLocator");
}

#[test]
fn scheme_and_port_rules_are_caller_owned_data() {
    let aia = decode_aia(&[description(
        ID_AD_OCSP_OID_DER,
        "http://ocsp.example.test?format=der",
    )]);
    let schemes = [
        X509OcspAiaSchemePolicy::new("https", 443, &[]),
        X509OcspAiaSchemePolicy::new("http", 80, &[8080]),
    ];
    let policy = https_policy(&schemes);
    assert_eq!(policy.allowed_schemes(), &schemes);
    assert_eq!(policy.maximum_uri_bytes(), 512);
    assert_eq!(policy.maximum_host_bytes(), 253);
    assert_eq!(policy.maximum_path_and_query_bytes(), 256);

    let selected = select_unique_x509_ocsp_aia_locator(&aia, &policy).unwrap();
    assert_eq!(selected.scheme_policy().scheme(), "http");
    assert_eq!(selected.explicit_port(), None);
    assert_eq!(selected.effective_port(), 80);
    assert_eq!(selected.path(), "");
    assert_eq!(selected.query(), Some("format=der"));
}

#[test]
fn requires_exactly_one_ocsp_description() {
    let schemes = [X509OcspAiaSchemePolicy::new("https", 443, &[])];
    let policy = https_policy(&schemes);
    let none = decode_aia(&[description(
        ID_AD_CA_ISSUERS_OID_DER,
        "https://ca.example.test/issuer.der",
    )]);
    assert_eq!(
        select_unique_x509_ocsp_aia_locator(&none, &policy)
            .unwrap_err()
            .kind(),
        X509OcspAiaLocatorErrorKind::NoOcspLocator
    );

    let ambiguous = decode_aia(&[
        description(ID_AD_OCSP_OID_DER, "https://one.example.test"),
        description(ID_AD_OCSP_OID_DER, "https://two.example.test"),
    ]);
    assert_eq!(
        select_unique_x509_ocsp_aia_locator(&ambiguous, &policy)
            .unwrap_err()
            .kind(),
        X509OcspAiaLocatorErrorKind::AmbiguousOcspLocator
    );
}

#[test]
fn never_skips_a_malformed_or_disallowed_ocsp_alternative() {
    let aia = decode_aia(&[
        description(ID_AD_OCSP_OID_DER, "https://valid.example.test"),
        description(ID_AD_OCSP_OID_DER, "ftp://invalid.example.test"),
    ]);
    let schemes = [X509OcspAiaSchemePolicy::new("https", 443, &[])];
    assert_eq!(
        select_unique_x509_ocsp_aia_locator(&aia, &https_policy(&schemes))
            .unwrap_err()
            .kind(),
        X509OcspAiaLocatorErrorKind::DisallowedScheme
    );
}

#[test]
fn rejects_disallowed_and_malformed_authorities() {
    let schemes = [X509OcspAiaSchemePolicy::new("https", 443, &[8443])];
    let policy = https_policy(&schemes);
    let cases = [
        (
            "ocsp.example.test",
            X509OcspAiaLocatorErrorKind::MissingScheme,
        ),
        (
            "https:ocsp.example.test",
            X509OcspAiaLocatorErrorKind::MissingAuthority,
        ),
        (
            "https:///status",
            X509OcspAiaLocatorErrorKind::MissingAuthority,
        ),
        (
            "https://user@ocsp.example.test",
            X509OcspAiaLocatorErrorKind::UserinfoNotAllowed,
        ),
        (
            "https://127.0.0.1",
            X509OcspAiaLocatorErrorKind::IpLiteralHost,
        ),
        ("https://127.1", X509OcspAiaLocatorErrorKind::IpLiteralHost),
        (
            "https://2130706433",
            X509OcspAiaLocatorErrorKind::IpLiteralHost,
        ),
        (
            "https://0177.0.0.1",
            X509OcspAiaLocatorErrorKind::IpLiteralHost,
        ),
        (
            "https://0x7f000001",
            X509OcspAiaLocatorErrorKind::IpLiteralHost,
        ),
        (
            "https://0X7f.0.0.1",
            X509OcspAiaLocatorErrorKind::IpLiteralHost,
        ),
        ("https://[::1]", X509OcspAiaLocatorErrorKind::InvalidPort),
        (
            "https://-bad.example",
            X509OcspAiaLocatorErrorKind::InvalidHost,
        ),
        (
            "https://bad..example",
            X509OcspAiaLocatorErrorKind::InvalidHost,
        ),
        (
            "https://bad_.example",
            X509OcspAiaLocatorErrorKind::InvalidHost,
        ),
        (
            "https://host.example:",
            X509OcspAiaLocatorErrorKind::InvalidPort,
        ),
        (
            "https://host.example:0",
            X509OcspAiaLocatorErrorKind::InvalidPort,
        ),
        (
            "https://host.example:444",
            X509OcspAiaLocatorErrorKind::DisallowedPort,
        ),
        (
            "https://host.example/path#fragment",
            X509OcspAiaLocatorErrorKind::FragmentNotAllowed,
        ),
        (
            "https://host.example/a%2",
            X509OcspAiaLocatorErrorKind::InvalidPathAndQuery,
        ),
        (
            "https://host.example/a b",
            X509OcspAiaLocatorErrorKind::InvalidPathAndQuery,
        ),
    ];

    for (uri, expected) in cases {
        let aia = decode_aia(&[description(ID_AD_OCSP_OID_DER, uri)]);
        assert_eq!(
            select_unique_x509_ocsp_aia_locator(&aia, &policy)
                .unwrap_err()
                .kind(),
            expected,
            "unexpected classification for {uri}"
        );
    }
}

#[test]
fn enforces_caller_and_hard_length_bounds() {
    let schemes = [X509OcspAiaSchemePolicy::new("https", 443, &[])];
    let aia = decode_aia(&[description(
        ID_AD_OCSP_OID_DER,
        "https://ocsp.example.test/long",
    )]);

    let uri_limited = X509OcspAiaLocatorPolicy::new(&schemes, 8, 253, 256);
    assert_eq!(
        select_unique_x509_ocsp_aia_locator(&aia, &uri_limited)
            .unwrap_err()
            .kind(),
        X509OcspAiaLocatorErrorKind::UriTooLong
    );
    let host_limited = X509OcspAiaLocatorPolicy::new(&schemes, 512, 4, 256);
    assert_eq!(
        select_unique_x509_ocsp_aia_locator(&aia, &host_limited)
            .unwrap_err()
            .kind(),
        X509OcspAiaLocatorErrorKind::HostTooLong
    );
    let target_limited = X509OcspAiaLocatorPolicy::new(&schemes, 512, 253, 2);
    assert_eq!(
        select_unique_x509_ocsp_aia_locator(&aia, &target_limited)
            .unwrap_err()
            .kind(),
        X509OcspAiaLocatorErrorKind::PathAndQueryTooLong
    );
}

#[test]
fn validates_complete_policy_before_locator_data() {
    let aia = decode_aia(&[description(
        ID_AD_OCSP_OID_DER,
        "ftp://private.example.invalid/credential",
    )]);
    let duplicate_schemes = [
        X509OcspAiaSchemePolicy::new("https", 443, &[]),
        X509OcspAiaSchemePolicy::new("https", 8443, &[]),
    ];
    let error =
        select_unique_x509_ocsp_aia_locator(&aia, &https_policy(&duplicate_schemes)).unwrap_err();
    assert_eq!(
        error.kind(),
        X509OcspAiaLocatorErrorKind::InvalidPolicy(
            X509OcspAiaLocatorPolicyErrorKind::DuplicateScheme
        )
    );
    assert!(!error.to_string().contains("private"));
    assert!(!format!("{error:?}").contains("credential"));

    let too_many_schemes =
        vec![X509OcspAiaSchemePolicy::new("https", 443, &[]); MAX_ALLOWED_SCHEMES + 1];
    assert_eq!(
        select_unique_x509_ocsp_aia_locator(&aia, &https_policy(&too_many_schemes))
            .unwrap_err()
            .kind(),
        X509OcspAiaLocatorErrorKind::InvalidPolicy(
            X509OcspAiaLocatorPolicyErrorKind::TooManyAllowedSchemes
        )
    );

    let too_many_ports = vec![8443; MAX_EXPLICIT_PORTS_PER_SCHEME + 1];
    let schemes = [X509OcspAiaSchemePolicy::new("https", 443, &too_many_ports)];
    assert_eq!(
        select_unique_x509_ocsp_aia_locator(&aia, &https_policy(&schemes))
            .unwrap_err()
            .kind(),
        X509OcspAiaLocatorErrorKind::InvalidPolicy(
            X509OcspAiaLocatorPolicyErrorKind::TooManyExplicitPorts
        )
    );
}

#[test]
fn classifies_remaining_invalid_policy_forms() {
    let aia = decode_aia(&[description(ID_AD_OCSP_OID_DER, "https://ocsp.example")]);
    let cases: Vec<(
        Vec<X509OcspAiaSchemePolicy<'static>>,
        usize,
        usize,
        usize,
        X509OcspAiaLocatorPolicyErrorKind,
    )> = vec![
        (
            vec![],
            512,
            253,
            256,
            X509OcspAiaLocatorPolicyErrorKind::NoAllowedSchemes,
        ),
        (
            vec![X509OcspAiaSchemePolicy::new("HTTPS", 443, &[])],
            512,
            253,
            256,
            X509OcspAiaLocatorPolicyErrorKind::InvalidScheme,
        ),
        (
            vec![X509OcspAiaSchemePolicy::new("https", 0, &[])],
            512,
            253,
            256,
            X509OcspAiaLocatorPolicyErrorKind::InvalidDefaultPort,
        ),
        (
            vec![X509OcspAiaSchemePolicy::new("https", 443, &[0])],
            512,
            253,
            256,
            X509OcspAiaLocatorPolicyErrorKind::InvalidExplicitPort,
        ),
        (
            vec![X509OcspAiaSchemePolicy::new("https", 443, &[8443, 8443])],
            512,
            253,
            256,
            X509OcspAiaLocatorPolicyErrorKind::DuplicateExplicitPort,
        ),
        (
            vec![X509OcspAiaSchemePolicy::new("https", 443, &[])],
            0,
            253,
            256,
            X509OcspAiaLocatorPolicyErrorKind::InvalidUriLimit,
        ),
        (
            vec![X509OcspAiaSchemePolicy::new("https", 443, &[])],
            512,
            0,
            256,
            X509OcspAiaLocatorPolicyErrorKind::InvalidHostLimit,
        ),
        (
            vec![X509OcspAiaSchemePolicy::new("https", 443, &[])],
            512,
            253,
            2049,
            X509OcspAiaLocatorPolicyErrorKind::InvalidPathAndQueryLimit,
        ),
    ];

    for (schemes, uri_limit, host_limit, target_limit, expected) in cases {
        let policy = X509OcspAiaLocatorPolicy::new(&schemes, uri_limit, host_limit, target_limit);
        assert_eq!(
            select_unique_x509_ocsp_aia_locator(&aia, &policy)
                .unwrap_err()
                .kind(),
            X509OcspAiaLocatorErrorKind::InvalidPolicy(expected)
        );
    }
}
