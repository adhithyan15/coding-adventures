use tls13_server_name::{
    InvalidDnsName, ServerNameError, TlsServerName, HOST_NAME_TYPE,
    MAX_CLIENT_EXTENSION_DATA_LENGTH, MAX_DNS_LABEL_LENGTH, MAX_DNS_NAME_LENGTH,
    SERVER_NAME_EXTENSION_TYPE,
};

#[test]
fn encodes_and_decodes_one_exact_host_name() {
    assert_eq!(SERVER_NAME_EXTENSION_TYPE, 0);
    assert_eq!(HOST_NAME_TYPE, 0);

    let name = TlsServerName::new("Api.Example.com").expect("valid DNS hostname");
    assert_eq!(name.as_str(), "Api.Example.com");
    assert_eq!(name.encoded_len(), 20);

    let mut output = [0xa5; 22];
    let written = name
        .encode_client_hello(&mut output)
        .expect("output has capacity");
    assert_eq!(written, 20);
    assert_eq!(
        &output[..written],
        &[
            0x00, 0x12, 0x00, 0x00, 0x0f, b'A', b'p', b'i', b'.', b'E', b'x', b'a', b'm', b'p',
            b'l', b'e', b'.', b'c', b'o', b'm',
        ]
    );
    assert_eq!(&output[written..], &[0xa5, 0xa5]);

    let decoded =
        TlsServerName::decode_client_hello(&output[..written]).expect("encoded form decodes");
    assert_eq!(decoded, name);
}

#[test]
fn dns_policy_accepts_bounded_labels_and_rejects_invalid_names() {
    let label = "a".repeat(MAX_DNS_LABEL_LENGTH);
    let maximum_name = format!("{label}.{label}.{label}.{}", "a".repeat(61));
    assert_eq!(maximum_name.len(), MAX_DNS_NAME_LENGTH);
    assert!(TlsServerName::new(&maximum_name).is_ok());
    assert!(TlsServerName::new("xn--bcher-kva.example").is_ok());
    assert!(TlsServerName::new("localhost").is_ok());
    assert!(TlsServerName::new("256.example").is_ok());
    assert!(TlsServerName::new("1.2.3.4.5").is_ok());

    let too_long = format!("a.{maximum_name}");
    let cases = [
        ("", InvalidDnsName::Empty),
        (too_long.as_str(), InvalidDnsName::TooLong),
        ("b\u{fc}cher.example", InvalidDnsName::NonAscii),
        ("api.example.com.", InvalidDnsName::TrailingDot),
        ("api..example", InvalidDnsName::EmptyLabel),
        ("-api.example", InvalidDnsName::LeadingHyphen),
        ("api-.example", InvalidDnsName::TrailingHyphen),
        ("*.example", InvalidDnsName::InvalidCharacter),
        ("api_name.example", InvalidDnsName::InvalidCharacter),
        ("192.0.2.1", InvalidDnsName::IpLiteral),
        ("256.0.0.1", InvalidDnsName::IpLiteral),
        ("256.1", InvalidDnsName::IpLiteral),
        ("255.16777216", InvalidDnsName::IpLiteral),
        ("08.0.0.1", InvalidDnsName::IpLiteral),
        ("127.1", InvalidDnsName::IpLiteral),
        ("2130706433", InvalidDnsName::IpLiteral),
        ("4294967295", InvalidDnsName::IpLiteral),
        ("0xffffffff", InvalidDnsName::IpLiteral),
        ("0177.0.0.1", InvalidDnsName::IpLiteral),
        ("0377.0377.0377.0377", InvalidDnsName::IpLiteral),
        ("0x7f.0.0.1", InvalidDnsName::IpLiteral),
        ("0X7F.0.0.1", InvalidDnsName::IpLiteral),
        ("127.0.1", InvalidDnsName::IpLiteral),
        ("127.65535", InvalidDnsName::IpLiteral),
        ("255.16777215", InvalidDnsName::IpLiteral),
        ("255.255.65535", InvalidDnsName::IpLiteral),
        ("2001:db8::1", InvalidDnsName::IpLiteral),
    ];

    for (value, expected) in cases {
        assert_eq!(
            TlsServerName::new(value),
            Err(ServerNameError::InvalidDnsName(expected))
        );
    }

    let overlong_label = format!("{}.example", "a".repeat(MAX_DNS_LABEL_LENGTH + 1));
    assert_eq!(
        TlsServerName::new(&overlong_label),
        Err(ServerNameError::InvalidDnsName(
            InvalidDnsName::LabelTooLong
        ))
    );
}

#[test]
fn output_failure_is_atomic_and_maximum_encoding_is_bounded() {
    assert_eq!(MAX_CLIENT_EXTENSION_DATA_LENGTH, 258);

    let name = TlsServerName::new("api.example").expect("valid name");
    let mut short = [0xa5; 15];
    assert_eq!(
        name.encode_client_hello(&mut short),
        Err(ServerNameError::OutputTooSmall)
    );
    assert_eq!(short, [0xa5; 15]);

    let maximum_name = format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(61),
    );
    let maximum = TlsServerName::new(&maximum_name).expect("maximum name is valid");
    assert_eq!(maximum.encoded_len(), MAX_CLIENT_EXTENSION_DATA_LENGTH);
    let mut output = [0_u8; MAX_CLIENT_EXTENSION_DATA_LENGTH];
    assert_eq!(maximum.encode_client_hello(&mut output), Ok(output.len()));
}

#[test]
fn decoder_rejects_malformed_lengths_extra_entries_and_unknown_types() {
    let malformed = [
        &[][..],
        &[0, 0][..],
        &[0, 4, 0, 0, 1][..],
        &[0, 3, 0, 0, 0][..],
        &[0, 4, 0, 0, 2, b'a'][..],
        &[0, 4, 0, 0, 1, b'a', 0][..],
        &[0, 8, 0, 0, 1, b'a', 0, 0, 1, b'b'][..],
    ];
    for extension_data in malformed {
        assert_eq!(
            TlsServerName::decode_client_hello(extension_data),
            Err(ServerNameError::MalformedClientExtension)
        );
    }

    assert_eq!(
        TlsServerName::decode_client_hello(&[0, 4, 1, 0, 1, b'a']),
        Err(ServerNameError::UnsupportedNameType)
    );
    assert_eq!(
        TlsServerName::decode_client_hello(&[0, 4, 0, 0, 1, 0xff]),
        Err(ServerNameError::InvalidDnsName(InvalidDnsName::NonAscii))
    );
}

#[test]
fn encrypted_extensions_acknowledgement_must_be_empty() {
    let name = TlsServerName::new("api.example").expect("valid name");
    assert_eq!(name.validate_server_acknowledgement(&[]), Ok(()));
    assert_eq!(
        name.validate_server_acknowledgement(&[0]),
        Err(ServerNameError::MalformedServerAcknowledgement)
    );
}

#[test]
fn diagnostics_expose_only_stable_categories_and_public_shape() {
    let invalid = ServerNameError::InvalidDnsName(InvalidDnsName::InvalidCharacter);
    assert_eq!(invalid.to_string(), "invalid TLS server DNS name");
    let rendered = format!("{invalid:?}");
    assert_eq!(rendered, "InvalidDnsName(InvalidCharacter)");
    assert_eq!(
        ServerNameError::MalformedClientExtension.to_string(),
        "malformed TLS client server-name extension"
    );
    assert_eq!(
        ServerNameError::UnsupportedNameType.to_string(),
        "unsupported TLS server-name type"
    );
    assert_eq!(
        ServerNameError::OutputTooSmall.to_string(),
        "output is too small for the TLS server-name extension"
    );
    assert_eq!(
        ServerNameError::MalformedServerAcknowledgement.to_string(),
        "malformed TLS server-name acknowledgement"
    );

    let name = TlsServerName::new("private.example").expect("valid name");
    let debug = format!("{name:?}");
    assert!(debug.contains("length: 15"));
    assert!(debug.contains("label_count: 2"));
    assert!(!debug.contains("private"));
}
