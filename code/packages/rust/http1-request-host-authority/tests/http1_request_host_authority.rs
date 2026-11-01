use http1_request_host_authority::{
    encode_http1_request_host_authority, validate_http1_request_host_authority,
    Http1RequestHostAuthorityError, Http1RequestHostAuthorityLimits,
    MAX_HTTP1_REQUEST_HOST_AUTHORITY_BYTES, MAX_HTTP1_REQUEST_HOST_NAME_BYTES,
};

fn limits() -> Http1RequestHostAuthorityLimits {
    Http1RequestHostAuthorityLimits::new(253, 259).unwrap()
}

#[test]
fn validates_and_encodes_lowercase_host_without_port() {
    let authority =
        validate_http1_request_host_authority(b"provider.example", None, limits()).unwrap();
    let mut output = [0xa5; 259];
    let encoded = encode_http1_request_host_authority(&authority, &mut output).unwrap();
    let encoded_length = encoded.len();

    assert_eq!(encoded, b"provider.example");
    assert_eq!(authority.host(), b"provider.example");
    assert_eq!(authority.port(), None);
    assert_eq!(authority.encoded_length(), encoded_length);
    assert_eq!(output[encoded_length], 0xa5);
}

#[test]
fn canonicalizes_dns_case_and_encodes_exact_explicit_ports() {
    for (host, port, expected) in [
        (
            b"OAuth.Example".as_slice(),
            1,
            b"oauth.example:1".as_slice(),
        ),
        (b"OAuth.Example".as_slice(), 443, b"oauth.example:443"),
        (b"OAuth.Example".as_slice(), 8_443, b"oauth.example:8443"),
        (
            b"OAuth.Example".as_slice(),
            u16::MAX,
            b"oauth.example:65535",
        ),
    ] {
        let authority = validate_http1_request_host_authority(host, Some(port), limits()).unwrap();
        let mut output = [0; 259];
        assert_eq!(
            encode_http1_request_host_authority(&authority, &mut output).unwrap(),
            expected
        );
    }
}

#[test]
fn accepts_data_driven_ldh_names_and_idna_alabels() {
    for host in [
        b"a.example".as_slice(),
        b"token-api.provider.example",
        b"xn--bcher-kva.example",
        b"7.example",
    ] {
        assert!(validate_http1_request_host_authority(host, None, limits()).is_ok());
    }
}

#[test]
fn validates_limit_construction_and_exact_ceilings() {
    let limits = Http1RequestHostAuthorityLimits::new(
        MAX_HTTP1_REQUEST_HOST_NAME_BYTES,
        MAX_HTTP1_REQUEST_HOST_AUTHORITY_BYTES,
    )
    .unwrap();
    assert_eq!(
        limits.maximum_host_bytes(),
        MAX_HTTP1_REQUEST_HOST_NAME_BYTES
    );
    assert_eq!(
        limits.maximum_authority_bytes(),
        MAX_HTTP1_REQUEST_HOST_AUTHORITY_BYTES
    );

    for invalid in [
        Http1RequestHostAuthorityLimits::new(0, 1),
        Http1RequestHostAuthorityLimits::new(MAX_HTTP1_REQUEST_HOST_NAME_BYTES + 1, 1),
        Http1RequestHostAuthorityLimits::new(1, 0),
        Http1RequestHostAuthorityLimits::new(1, MAX_HTTP1_REQUEST_HOST_AUTHORITY_BYTES + 1),
    ] {
        assert_eq!(
            invalid.unwrap_err(),
            Http1RequestHostAuthorityError::InvalidLimits
        );
    }
}

#[test]
fn rejects_empty_oversized_and_malformed_dns_names() {
    assert_eq!(
        validate_http1_request_host_authority(b"", None, limits()).unwrap_err(),
        Http1RequestHostAuthorityError::EmptyHost
    );
    let oversized = vec![b'a'; MAX_HTTP1_REQUEST_HOST_NAME_BYTES + 1];
    assert_eq!(
        validate_http1_request_host_authority(&oversized, None, limits()).unwrap_err(),
        Http1RequestHostAuthorityError::HostTooLong
    );
    for host in [b".example".as_slice(), b"example.", b"two..dots"] {
        assert_eq!(
            validate_http1_request_host_authority(host, None, limits()).unwrap_err(),
            Http1RequestHostAuthorityError::EmptyLabel
        );
    }
    let long_label = vec![b'a'; 64];
    assert_eq!(
        validate_http1_request_host_authority(&long_label, None, limits()).unwrap_err(),
        Http1RequestHostAuthorityError::LabelTooLong
    );
}

#[test]
fn rejects_uri_delimiters_ip_literals_unicode_and_invalid_labels() {
    for host in [
        b"-bad.example".as_slice(),
        b"bad-.example",
        b"under_score.example",
        b"provider.example:443",
        b"user@provider.example",
        b"provider.example/path",
        b"provider example",
        b"provider\r\nexample",
        b"[2001:db8::1]",
        b"b\xc3\xbccher.example",
    ] {
        assert_eq!(
            validate_http1_request_host_authority(host, None, limits()).unwrap_err(),
            Http1RequestHostAuthorityError::InvalidLabel
        );
    }
    for host in [
        b"192.0.2.1".as_slice(),
        b"001.002.003.004",
        b"127.1",
        b"2130706433",
        b"017700000001",
        b"0x7f.0.0.1",
        b"0X7F000001",
        b"999.999.999.999",
    ] {
        assert_eq!(
            validate_http1_request_host_authority(host, None, limits()).unwrap_err(),
            Http1RequestHostAuthorityError::IpLiteral
        );
    }
}

#[test]
fn enforces_nonzero_port_authority_limit_and_output_capacity_before_writing() {
    assert_eq!(
        validate_http1_request_host_authority(b"example.test", Some(0), limits()).unwrap_err(),
        Http1RequestHostAuthorityError::ZeroPort
    );
    let tight = Http1RequestHostAuthorityLimits::new(253, 15).unwrap();
    assert_eq!(
        validate_http1_request_host_authority(b"example.test", Some(443), tight).unwrap_err(),
        Http1RequestHostAuthorityError::AuthorityTooLong
    );

    let authority =
        validate_http1_request_host_authority(b"example.test", Some(443), limits()).unwrap();
    let mut output = [0xa5; 8];
    assert_eq!(
        encode_http1_request_host_authority(&authority, &mut output).unwrap_err(),
        Http1RequestHostAuthorityError::OutputTooSmall
    );
    assert_eq!(output, [0xa5; 8]);
}

#[test]
fn debug_and_errors_do_not_render_hostname_bytes() {
    let authority =
        validate_http1_request_host_authority(b"private.provider.example", Some(443), limits())
            .unwrap();
    let debug = format!("{authority:?}");
    assert!(!debug.contains("private.provider.example"));
    assert!(debug.contains("[REDACTED]"));

    let error =
        validate_http1_request_host_authority(b"private.provider.example/path", None, limits())
            .unwrap_err();
    assert!(!error.to_string().contains("private.provider.example"));
}
