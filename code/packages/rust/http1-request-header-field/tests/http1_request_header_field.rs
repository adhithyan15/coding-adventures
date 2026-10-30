use http1_request_header_field::{
    encode_http1_request_header_field, validate_http1_request_header_field,
    Http1RequestHeaderFieldError, Http1RequestHeaderFieldLimits,
    MAX_HTTP1_REQUEST_HEADER_LINE_BYTES, MAX_HTTP1_REQUEST_HEADER_NAME_BYTES,
    MAX_HTTP1_REQUEST_HEADER_VALUE_BYTES,
};

fn limits() -> Http1RequestHeaderFieldLimits {
    Http1RequestHeaderFieldLimits::new(64, 512, 580).unwrap()
}

#[test]
fn validates_and_encodes_one_exact_field_line() {
    let field =
        validate_http1_request_header_field(b"Host", b"provider.example", limits()).unwrap();
    let mut output = [0xa5; 580];
    let encoded = encode_http1_request_header_field(&field, &mut output).unwrap();
    let encoded_length = encoded.len();

    assert_eq!(encoded, b"Host: provider.example\r\n");
    assert_eq!(field.name(), b"Host");
    assert_eq!(field.value(), b"provider.example");
    assert_eq!(field.encoded_length(), encoded_length);
    assert_eq!(output[encoded_length], 0xa5);
}

#[test]
fn preserves_data_driven_token_names_empty_values_and_internal_whitespace() {
    let mut output = [0; 128];
    let empty = validate_http1_request_header_field(b"X-Custom", b"", limits()).unwrap();
    assert_eq!(
        encode_http1_request_header_field(&empty, &mut output).unwrap(),
        b"X-Custom: \r\n"
    );

    let spaced =
        validate_http1_request_header_field(b"X-Custom", b"one two\tthree", limits()).unwrap();
    assert_eq!(
        encode_http1_request_header_field(&spaced, &mut output).unwrap(),
        b"X-Custom: one two\tthree\r\n"
    );
}

#[test]
fn validates_limit_construction_and_exposes_exact_ceilings() {
    let limits = Http1RequestHeaderFieldLimits::new(
        MAX_HTTP1_REQUEST_HEADER_NAME_BYTES,
        MAX_HTTP1_REQUEST_HEADER_VALUE_BYTES,
        MAX_HTTP1_REQUEST_HEADER_LINE_BYTES,
    )
    .unwrap();
    assert_eq!(
        limits.maximum_name_bytes(),
        MAX_HTTP1_REQUEST_HEADER_NAME_BYTES
    );
    assert_eq!(
        limits.maximum_value_bytes(),
        MAX_HTTP1_REQUEST_HEADER_VALUE_BYTES
    );
    assert_eq!(
        limits.maximum_line_bytes(),
        MAX_HTTP1_REQUEST_HEADER_LINE_BYTES
    );

    for invalid in [
        Http1RequestHeaderFieldLimits::new(0, 0, 1),
        Http1RequestHeaderFieldLimits::new(MAX_HTTP1_REQUEST_HEADER_NAME_BYTES + 1, 0, 1),
        Http1RequestHeaderFieldLimits::new(1, MAX_HTTP1_REQUEST_HEADER_VALUE_BYTES + 1, 1),
        Http1RequestHeaderFieldLimits::new(1, 0, 0),
        Http1RequestHeaderFieldLimits::new(1, 0, MAX_HTTP1_REQUEST_HEADER_LINE_BYTES + 1),
    ] {
        assert_eq!(
            invalid.unwrap_err(),
            Http1RequestHeaderFieldError::InvalidLimits
        );
    }
}

#[test]
fn rejects_empty_oversized_and_non_token_names() {
    assert_eq!(
        validate_http1_request_header_field(b"", b"value", limits()).unwrap_err(),
        Http1RequestHeaderFieldError::EmptyName
    );
    let oversized = vec![b'a'; limits().maximum_name_bytes() + 1];
    assert_eq!(
        validate_http1_request_header_field(&oversized, b"value", limits()).unwrap_err(),
        Http1RequestHeaderFieldError::NameTooLong
    );
    for name in [b"Bad Name".as_slice(), b"Bad:Name", b"Bad\r\nName"] {
        assert_eq!(
            validate_http1_request_header_field(name, b"value", limits()).unwrap_err(),
            Http1RequestHeaderFieldError::InvalidName
        );
    }
}

#[test]
fn rejects_unsafe_non_ascii_and_outer_whitespace_values() {
    for value in [
        b"bad\r\nInjected: value".as_slice(),
        b"bad\0value",
        b"bad\x7fvalue",
        b"bad\x80value",
    ] {
        assert_eq!(
            validate_http1_request_header_field(b"X-Test", value, limits()).unwrap_err(),
            Http1RequestHeaderFieldError::InvalidValue
        );
    }
    for value in [
        b" leading".as_slice(),
        b"trailing ",
        b"\tleading",
        b"trailing\t",
    ] {
        assert_eq!(
            validate_http1_request_header_field(b"X-Test", value, limits()).unwrap_err(),
            Http1RequestHeaderFieldError::ValueHasOuterWhitespace
        );
    }

    let oversized = vec![b'a'; limits().maximum_value_bytes() + 1];
    assert_eq!(
        validate_http1_request_header_field(b"X-Test", &oversized, limits()).unwrap_err(),
        Http1RequestHeaderFieldError::ValueTooLong
    );
}

#[test]
fn enforces_complete_line_and_output_bounds_before_writing() {
    let tight = Http1RequestHeaderFieldLimits::new(16, 16, 8).unwrap();
    assert_eq!(
        validate_http1_request_header_field(b"Host", b"value", tight).unwrap_err(),
        Http1RequestHeaderFieldError::LineTooLong
    );

    let field = validate_http1_request_header_field(b"Host", b"value", limits()).unwrap();
    let mut output = [0xa5; 8];
    assert_eq!(
        encode_http1_request_header_field(&field, &mut output).unwrap_err(),
        Http1RequestHeaderFieldError::OutputTooSmall
    );
    assert_eq!(output, [0xa5; 8]);
}

#[test]
fn debug_and_errors_do_not_render_potentially_sensitive_field_bytes() {
    let field =
        validate_http1_request_header_field(b"Authorization", b"Bearer secret-value", limits())
            .unwrap();
    let debug = format!("{field:?}");
    assert!(!debug.contains("Authorization"));
    assert!(!debug.contains("secret-value"));
    assert!(debug.contains("[REDACTED]"));

    let error = validate_http1_request_header_field(
        b"Authorization",
        b"Bearer secret-value\r\nInjected: true",
        limits(),
    )
    .unwrap_err();
    assert!(!error.to_string().contains("Authorization"));
    assert!(!error.to_string().contains("secret-value"));
}
