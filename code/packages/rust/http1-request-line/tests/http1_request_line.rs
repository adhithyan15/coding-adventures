use http1_request_line::{
    encode_http1_request_line, validate_http1_request_line, Http1RequestLineError,
    Http1RequestLineLimits, Http1RequestVersion, MAX_HTTP1_REQUEST_LINE_BYTES,
    MAX_HTTP1_REQUEST_METHOD_BYTES, MAX_HTTP1_REQUEST_TARGET_BYTES,
};

fn limits() -> Http1RequestLineLimits {
    Http1RequestLineLimits::new(16, 256, 300).unwrap()
}

#[test]
fn validates_and_encodes_one_exact_http11_origin_form_line() {
    let line = validate_http1_request_line(
        b"POST",
        b"/oauth/token?mode=refresh",
        Http1RequestVersion::Http11,
        limits(),
    )
    .unwrap();
    let mut output = [0xa5; 300];
    let encoded = encode_http1_request_line(&line, &mut output).unwrap();

    assert_eq!(encoded, b"POST /oauth/token?mode=refresh HTTP/1.1\r\n");
    assert_eq!(line.method(), b"POST");
    assert_eq!(line.target(), b"/oauth/token?mode=refresh");
    assert_eq!(line.version(), Http1RequestVersion::Http11);
    assert_eq!(line.encoded_length(), encoded.len());
    assert_eq!(output[encoded.len()], 0xa5);
}

#[test]
fn preserves_data_driven_token_methods_and_http10() {
    let line = validate_http1_request_line(
        b"CUSTOM-METHOD",
        b"/",
        Http1RequestVersion::Http10,
        limits(),
    )
    .unwrap();
    let mut output = [0; 64];
    assert_eq!(
        encode_http1_request_line(&line, &mut output).unwrap(),
        b"CUSTOM-METHOD / HTTP/1.0\r\n"
    );
}

#[test]
fn accepts_exact_origin_form_path_query_and_percent_encoding_grammar() {
    let line = validate_http1_request_line(
        b"GET",
        b"/a!$&'()*+,-._~;=:@/%2F?next=/b?c&value=%7e",
        Http1RequestVersion::Http11,
        limits(),
    )
    .unwrap();
    assert_eq!(
        line.target(),
        b"/a!$&'()*+,-._~;=:@/%2F?next=/b?c&value=%7e"
    );
}

#[test]
fn validates_limit_construction_and_exposes_exact_ceilings() {
    let limits = Http1RequestLineLimits::new(
        MAX_HTTP1_REQUEST_METHOD_BYTES,
        MAX_HTTP1_REQUEST_TARGET_BYTES,
        MAX_HTTP1_REQUEST_LINE_BYTES,
    )
    .unwrap();
    assert_eq!(
        limits.maximum_method_bytes(),
        MAX_HTTP1_REQUEST_METHOD_BYTES
    );
    assert_eq!(
        limits.maximum_target_bytes(),
        MAX_HTTP1_REQUEST_TARGET_BYTES
    );
    assert_eq!(limits.maximum_line_bytes(), MAX_HTTP1_REQUEST_LINE_BYTES);

    for invalid in [
        Http1RequestLineLimits::new(0, 1, 1),
        Http1RequestLineLimits::new(MAX_HTTP1_REQUEST_METHOD_BYTES + 1, 1, 1),
        Http1RequestLineLimits::new(1, 0, 1),
        Http1RequestLineLimits::new(1, MAX_HTTP1_REQUEST_TARGET_BYTES + 1, 1),
        Http1RequestLineLimits::new(1, 1, 0),
        Http1RequestLineLimits::new(1, 1, MAX_HTTP1_REQUEST_LINE_BYTES + 1),
    ] {
        assert_eq!(invalid.unwrap_err(), Http1RequestLineError::InvalidLimits);
    }
}

#[test]
fn rejects_empty_oversized_and_non_token_methods() {
    assert_eq!(
        validate_http1_request_line(b"", b"/", Http1RequestVersion::Http11, limits()).unwrap_err(),
        Http1RequestLineError::EmptyMethod
    );
    assert_eq!(
        validate_http1_request_line(
            b"METHOD-THAT-IS-TOO-LONG",
            b"/",
            Http1RequestVersion::Http11,
            limits(),
        )
        .unwrap_err(),
        Http1RequestLineError::MethodTooLong
    );
    for method in [b"BAD METHOD".as_slice(), b"BAD\rMETHOD", b"BAD:METHOD"] {
        assert_eq!(
            validate_http1_request_line(method, b"/", Http1RequestVersion::Http11, limits())
                .unwrap_err(),
            Http1RequestLineError::InvalidMethod
        );
    }
}

#[test]
fn rejects_non_origin_form_and_unsafe_targets() {
    assert_eq!(
        validate_http1_request_line(b"GET", b"", Http1RequestVersion::Http11, limits())
            .unwrap_err(),
        Http1RequestLineError::EmptyTarget
    );
    assert_eq!(
        validate_http1_request_line(
            b"GET",
            b"https://provider.example/token",
            Http1RequestVersion::Http11,
            limits(),
        )
        .unwrap_err(),
        Http1RequestLineError::TargetNotOriginForm
    );
    for target in [
        b"/token value".as_slice(),
        b"/token#fragment",
        b"/token\r\nInjected: value",
        b"/token\x7f",
        b"/a\\b",
        b"/a\"b",
        b"/[x]",
        b"/x%GG",
        b"/x%2",
    ] {
        assert_eq!(
            validate_http1_request_line(b"GET", target, Http1RequestVersion::Http11, limits())
                .unwrap_err(),
            Http1RequestLineError::InvalidTarget
        );
    }

    let oversized = vec![b'a'; limits().maximum_target_bytes() + 1];
    let mut oversized_origin = oversized;
    oversized_origin[0] = b'/';
    assert_eq!(
        validate_http1_request_line(
            b"GET",
            &oversized_origin,
            Http1RequestVersion::Http11,
            limits(),
        )
        .unwrap_err(),
        Http1RequestLineError::TargetTooLong
    );
}

#[test]
fn enforces_complete_line_and_output_bounds_before_writing() {
    let tight = Http1RequestLineLimits::new(16, 256, 16).unwrap();
    assert_eq!(
        validate_http1_request_line(b"GET", b"/token", Http1RequestVersion::Http11, tight)
            .unwrap_err(),
        Http1RequestLineError::LineTooLong
    );

    let line =
        validate_http1_request_line(b"GET", b"/token", Http1RequestVersion::Http11, limits())
            .unwrap();
    let mut output = [0xa5; 8];
    assert_eq!(
        encode_http1_request_line(&line, &mut output).unwrap_err(),
        Http1RequestLineError::OutputTooSmall
    );
    assert_eq!(output, [0xa5; 8]);
}

#[test]
fn debug_and_errors_do_not_retain_or_render_request_bytes() {
    let line = validate_http1_request_line(
        b"SECRET-METHOD",
        b"/secret-target",
        Http1RequestVersion::Http11,
        limits(),
    )
    .unwrap();
    let debug = format!("{line:?}");
    assert!(!debug.contains("SECRET-METHOD"));
    assert!(!debug.contains("secret-target"));
    assert!(debug.contains("[REDACTED]"));

    let error = validate_http1_request_line(
        b"BAD METHOD",
        b"/secret-target",
        Http1RequestVersion::Http11,
        limits(),
    )
    .unwrap_err();
    assert!(!error.to_string().contains("BAD METHOD"));
    assert!(!error.to_string().contains("secret-target"));
}
