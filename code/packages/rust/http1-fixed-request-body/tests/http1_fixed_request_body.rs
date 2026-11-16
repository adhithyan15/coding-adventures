use http1_fixed_request_body::{
    bind_http1_fixed_request_content_length_field, encode_http1_fixed_request_content_length,
    validate_http1_fixed_request_body, Http1FixedRequestBodyError, Http1FixedRequestBodyLimits,
    Http1FixedRequestContentLengthFieldError, MAX_HTTP1_FIXED_REQUEST_BODY_BYTES,
    MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES,
};
use http1_request_header_field::{
    validate_http1_request_header_field, Http1RequestHeaderFieldLimits,
};

fn limits() -> Http1FixedRequestBodyLimits {
    Http1FixedRequestBodyLimits::new(4096).unwrap()
}

fn field_limits() -> Http1RequestHeaderFieldLimits {
    Http1RequestHeaderFieldLimits::new(64, 64, 132).unwrap()
}

#[test]
fn binds_exact_body_to_one_canonical_content_length_field() {
    let body = b"grant_type=refresh_token&refresh_token=secret";
    let body_evidence = validate_http1_fixed_request_body(body, limits()).unwrap();
    let expected_length = body.len().to_string();
    let field = validate_http1_request_header_field(
        b"Content-Length",
        expected_length.as_bytes(),
        field_limits(),
    )
    .unwrap();

    let bound = bind_http1_fixed_request_content_length_field(body_evidence, field).unwrap();

    assert_eq!(bound.body().body(), body);
    assert_eq!(bound.field().name(), b"Content-Length");
    assert_eq!(bound.field().value(), expected_length.as_bytes());
}

#[test]
fn accepts_mixed_case_name_and_empty_body_canonical_zero() {
    let body_evidence = validate_http1_fixed_request_body(b"", limits()).unwrap();
    let field =
        validate_http1_request_header_field(b"cOnTeNt-LeNgTh", b"0", field_limits()).unwrap();

    let bound = bind_http1_fixed_request_content_length_field(body_evidence, field).unwrap();

    assert_eq!(bound.body().body(), b"");
    assert_eq!(bound.field().name(), b"cOnTeNt-LeNgTh");
    assert_eq!(bound.field().value(), b"0");
}

#[test]
fn rejects_a_different_field_name_without_retaining_input() {
    let body_evidence = validate_http1_fixed_request_body(b"secret", limits()).unwrap();
    let field = validate_http1_request_header_field(b"X-Length", b"6", field_limits()).unwrap();

    let error = bind_http1_fixed_request_content_length_field(body_evidence, field).unwrap_err();

    assert_eq!(
        error,
        Http1FixedRequestContentLengthFieldError::NotContentLength
    );
    assert!(!error.to_string().contains("X-Length"));
    assert!(!format!("{error:?}").contains("X-Length"));
}

#[test]
fn rejects_noncanonical_or_mismatched_content_length_without_retaining_value() {
    for value in [b"06".as_slice(), b"5", b"7", b"+6", b""] {
        let body_evidence = validate_http1_fixed_request_body(b"secret", limits()).unwrap();
        let field =
            validate_http1_request_header_field(b"Content-Length", value, field_limits()).unwrap();

        let error =
            bind_http1_fixed_request_content_length_field(body_evidence, field).unwrap_err();

        assert_eq!(
            error,
            Http1FixedRequestContentLengthFieldError::ContentLengthMismatch
        );
        assert_eq!(
            error.to_string(),
            "HTTP/1 fixed request-body Content-Length does not match the exact body length"
        );
        assert_eq!(format!("{error:?}"), "ContentLengthMismatch");
    }
}

#[test]
fn bound_debug_redacts_body_and_field_bytes() {
    let body_evidence =
        validate_http1_fixed_request_body(b"refresh_token=secret", limits()).unwrap();
    let field =
        validate_http1_request_header_field(b"Content-Length", b"20", field_limits()).unwrap();
    let bound = bind_http1_fixed_request_content_length_field(body_evidence, field).unwrap();

    let debug = format!("{bound:?}");
    assert!(!debug.contains("refresh_token"));
    assert!(!debug.contains("secret"));
    assert!(!debug.contains("Content-Length"));
    assert!(debug.contains("[REDACTED]"));
}

#[test]
fn binds_body_and_encodes_exact_shortest_content_length() {
    let body = b"grant_type=refresh_token&refresh_token=secret";
    let evidence = validate_http1_fixed_request_body(body, limits()).unwrap();
    let mut output = [0xa5; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES + 1];
    let encoded = encode_http1_fixed_request_content_length(&evidence, &mut output).unwrap();
    let encoded_length = encoded.len();

    assert_eq!(encoded, body.len().to_string().as_bytes());
    assert_eq!(evidence.body(), body);
    assert_eq!(evidence.content_length(), body.len());
    assert_eq!(evidence.content_length_value_length(), encoded_length);
    assert_eq!(output[encoded_length], 0xa5);
}

#[test]
fn encodes_empty_body_as_zero() {
    let evidence = validate_http1_fixed_request_body(b"", limits()).unwrap();
    let mut output = [0; 1];
    assert_eq!(
        encode_http1_fixed_request_content_length(&evidence, &mut output).unwrap(),
        b"0"
    );
}

#[test]
fn encodes_every_decimal_width_without_leading_zeroes() {
    for length in [0, 1, 9, 10, 99, 100, 999, 1_000, 9_999, 10_000] {
        let body = vec![0u8; length];
        let maximum = Http1FixedRequestBodyLimits::new(MAX_HTTP1_FIXED_REQUEST_BODY_BYTES).unwrap();
        let evidence = validate_http1_fixed_request_body(&body, maximum).unwrap();
        let mut output = [0; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
        let encoded = encode_http1_fixed_request_content_length(&evidence, &mut output).unwrap();
        assert_eq!(encoded, length.to_string().as_bytes());
        assert!(!encoded.starts_with(b"0") || encoded == b"0");
    }
}

#[test]
fn validates_zero_and_maximum_limits() {
    let zero = Http1FixedRequestBodyLimits::new(0).unwrap();
    assert_eq!(zero.maximum_body_bytes(), 0);
    assert!(validate_http1_fixed_request_body(b"", zero).is_ok());
    assert_eq!(
        validate_http1_fixed_request_body(b"x", zero).unwrap_err(),
        Http1FixedRequestBodyError::BodyTooLong
    );

    let maximum = Http1FixedRequestBodyLimits::new(MAX_HTTP1_FIXED_REQUEST_BODY_BYTES).unwrap();
    assert_eq!(
        maximum.maximum_body_bytes(),
        MAX_HTTP1_FIXED_REQUEST_BODY_BYTES
    );
    let maximum_body = vec![0u8; MAX_HTTP1_FIXED_REQUEST_BODY_BYTES];
    let maximum_evidence = validate_http1_fixed_request_body(&maximum_body, maximum).unwrap();
    let mut output = [0; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    assert_eq!(
        encode_http1_fixed_request_content_length(&maximum_evidence, &mut output).unwrap(),
        b"1048576"
    );
    assert_eq!(
        Http1FixedRequestBodyLimits::new(MAX_HTTP1_FIXED_REQUEST_BODY_BYTES + 1).unwrap_err(),
        Http1FixedRequestBodyError::InvalidLimits
    );
}

#[test]
fn rejects_body_above_caller_limit_without_retaining_payload() {
    let body = b"secret-value";
    let tight = Http1FixedRequestBodyLimits::new(body.len() - 1).unwrap();
    let error = validate_http1_fixed_request_body(body, tight).unwrap_err();
    assert_eq!(error, Http1FixedRequestBodyError::BodyTooLong);
    assert!(!error.to_string().contains("secret-value"));
    assert!(!format!("{error:?}").contains("secret-value"));
}

#[test]
fn leaves_undersized_output_unchanged() {
    let body = vec![0u8; 100];
    let evidence = validate_http1_fixed_request_body(&body, limits()).unwrap();
    let mut output = [0xa5; 2];
    assert_eq!(
        encode_http1_fixed_request_content_length(&evidence, &mut output).unwrap_err(),
        Http1FixedRequestBodyError::OutputTooSmall
    );
    assert_eq!(output, [0xa5; 2]);
}

#[test]
fn debug_redacts_potentially_sensitive_body() {
    let evidence =
        validate_http1_fixed_request_body(b"refresh_token=secret-value", limits()).unwrap();
    let debug = format!("{evidence:?}");
    assert!(!debug.contains("refresh_token"));
    assert!(!debug.contains("secret-value"));
    assert!(debug.contains("[REDACTED]"));
}
