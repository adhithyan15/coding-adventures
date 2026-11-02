use http1_response_head::{
    decode_http1_response_head, Http1ResponseHeadError, Http1ResponseHeadLimits,
    Http1ResponseVersion, MAX_HTTP1_RESPONSE_HEADERS, MAX_HTTP1_RESPONSE_HEAD_BYTES,
    MAX_HTTP1_RESPONSE_LINE_BYTES,
};

fn limits() -> Http1ResponseHeadLimits {
    Http1ResponseHeadLimits::new(1_024, 256, 8).unwrap()
}

fn error(input: &[u8]) -> Http1ResponseHeadError {
    decode_http1_response_head(input, limits()).unwrap_err()
}

#[test]
fn decodes_exact_head_headers_and_body_without_copying() {
    let input = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Trace:\t public \t\r\n\r\n{\"ok\":true}";
    let head_end = input
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap()
        + 4;
    let head = decode_http1_response_head(input, limits()).unwrap();
    assert_eq!(head.version(), Http1ResponseVersion::Http11);
    assert_eq!(head.status_code(), 200);
    assert_eq!(head.reason_phrase(), b"OK");
    assert_eq!(head.encoded_head(), &input[..head_end]);
    assert_eq!(head.body_tail(), b"{\"ok\":true}");
    assert_eq!(head.headers().len(), 2);
    assert_eq!(head.headers()[0].name(), b"Content-Type");
    assert_eq!(head.headers()[0].value(), b"application/json");
    assert!(head.headers()[0].name_eq_ignore_ascii_case(b"content-type"));
    assert_eq!(head.headers()[1].value(), b"public");
    assert!(!head.headers()[1].name_eq_ignore_ascii_case(b"content-type"));
    assert!(format!("{head:?}").contains("reason_phrase: \"[REDACTED]\""));
    assert!(!format!("{head:?}").contains("application/json"));
    assert!(!format!("{:?}", head.headers()[0]).contains("Content-Type"));
}

#[test]
fn accepts_http10_empty_reason_empty_headers_and_obsolete_text() {
    let input = b"HTTP/1.0 204 \x80\r\n\r\n";
    let head = decode_http1_response_head(input, limits()).unwrap();
    assert_eq!(head.version(), Http1ResponseVersion::Http10);
    assert_eq!(head.status_code(), 204);
    assert_eq!(head.reason_phrase(), &[0x80]);
    assert!(head.headers().is_empty());

    let input = b"HTTP/1.1 200 \r\nX-Bytes: \x80\xff\r\n\r\n";
    let head = decode_http1_response_head(input, limits()).unwrap();
    assert_eq!(head.reason_phrase(), b"");
    assert_eq!(head.headers()[0].value(), &[0x80, 0xff]);
}

#[test]
fn validates_and_exposes_caller_limits() {
    for invalid in [
        (0, 1, 0),
        (MAX_HTTP1_RESPONSE_HEAD_BYTES + 1, 1, 0),
        (1, 0, 0),
        (
            MAX_HTTP1_RESPONSE_HEAD_BYTES,
            MAX_HTTP1_RESPONSE_LINE_BYTES + 1,
            0,
        ),
        (8, 9, 0),
        (8, 8, MAX_HTTP1_RESPONSE_HEADERS + 1),
    ] {
        assert_eq!(
            Http1ResponseHeadLimits::new(invalid.0, invalid.1, invalid.2).unwrap_err(),
            Http1ResponseHeadError::InvalidLimits
        );
    }
    let limits = Http1ResponseHeadLimits::new(512, 128, 0).unwrap();
    assert_eq!(limits.maximum_head_bytes(), 512);
    assert_eq!(limits.maximum_line_bytes(), 128);
    assert_eq!(limits.maximum_headers(), 0);
}

#[test]
fn rejects_incomplete_oversized_or_non_crlf_lines() {
    assert_eq!(
        error(b"HTTP/1.1 200 OK\r\n"),
        Http1ResponseHeadError::IncompleteHead
    );
    assert_eq!(
        error(b"HTTP/1.1 200 OK\r"),
        Http1ResponseHeadError::IncompleteHead
    );
    assert_eq!(
        error(b"HTTP/1.1 200 OK\n\n"),
        Http1ResponseHeadError::InvalidLineEnding
    );
    assert_eq!(
        error(b"HTTP/1.1 200 OK\rx"),
        Http1ResponseHeadError::InvalidLineEnding
    );

    let short = Http1ResponseHeadLimits::new(16, 16, 0).unwrap();
    assert_eq!(
        decode_http1_response_head(b"HTTP/1.1 200 OK\r\n\r\n", short).unwrap_err(),
        Http1ResponseHeadError::HeadTooLong
    );
    assert_eq!(
        decode_http1_response_head(b"123456789012345\r", short).unwrap_err(),
        Http1ResponseHeadError::HeadTooLong
    );
    assert_eq!(
        decode_http1_response_head(b"1234567890123456", short).unwrap_err(),
        Http1ResponseHeadError::HeadTooLong
    );
    let short_line = Http1ResponseHeadLimits::new(64, 8, 0).unwrap();
    assert_eq!(
        decode_http1_response_head(b"HTTP/1.1 200 OK\r\n\r\n", short_line).unwrap_err(),
        Http1ResponseHeadError::LineTooLong
    );
}

#[test]
fn rejects_invalid_status_lines_without_echoing_payload() {
    let cases: &[(&[u8], Http1ResponseHeadError)] = &[
        (
            b"HTTP/1.1 200\r\n\r\n",
            Http1ResponseHeadError::InvalidStatusLine,
        ),
        (
            b"HTTP/2.0 200 OK\r\n\r\n",
            Http1ResponseHeadError::InvalidVersion,
        ),
        (
            b"HTTP/1.1 2x0 OK\r\n\r\n",
            Http1ResponseHeadError::InvalidStatusCode,
        ),
        (
            b"HTTP/1.1 099 Nope\r\n\r\n",
            Http1ResponseHeadError::InvalidStatusCode,
        ),
        (
            b"HTTP/1.1 600 Nope\r\n\r\n",
            Http1ResponseHeadError::InvalidStatusCode,
        ),
        (
            b"HTTP/1.1 200 O\x01K\r\n\r\n",
            Http1ResponseHeadError::InvalidReasonPhrase,
        ),
    ];
    for (input, expected) in cases {
        let actual = error(input);
        assert_eq!(&actual, expected);
        assert!(!actual.to_string().contains("Nope"));
        assert!(!format!("{actual:?}").contains("200"));
    }
}

#[test]
fn rejects_malformed_or_excess_headers() {
    let cases: &[(&[u8], Http1ResponseHeadError)] = &[
        (
            b"HTTP/1.1 200 OK\r\n folded\r\n\r\n",
            Http1ResponseHeadError::ObsoleteLineFolding,
        ),
        (
            b"HTTP/1.1 200 OK\r\nMissing\r\n\r\n",
            Http1ResponseHeadError::MissingHeaderColon,
        ),
        (
            b"HTTP/1.1 200 OK\r\n:value\r\n\r\n",
            Http1ResponseHeadError::EmptyHeaderName,
        ),
        (
            b"HTTP/1.1 200 OK\r\nBad Name:value\r\n\r\n",
            Http1ResponseHeadError::InvalidHeaderName,
        ),
        (
            b"HTTP/1.1 200 OK\r\nName : value\r\n\r\n",
            Http1ResponseHeadError::InvalidHeaderName,
        ),
        (
            b"HTTP/1.1 200 OK\r\n\tfolded\r\n\r\n",
            Http1ResponseHeadError::ObsoleteLineFolding,
        ),
        (
            b"HTTP/1.1 200 OK\r\nName:\x01\r\n\r\n",
            Http1ResponseHeadError::InvalidHeaderValue,
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(error(input), *expected);
    }

    let one = Http1ResponseHeadLimits::new(128, 64, 1).unwrap();
    assert_eq!(
        decode_http1_response_head(b"HTTP/1.1 200 OK\r\nA: 1\r\nB: 2\r\n\r\n", one).unwrap_err(),
        Http1ResponseHeadError::TooManyHeaders
    );

    let none = Http1ResponseHeadLimits::new(128, 64, 0).unwrap();
    assert_eq!(
        decode_http1_response_head(b"HTTP/1.1 200 OK\r\nA:\r\n\r\n", none).unwrap_err(),
        Http1ResponseHeadError::TooManyHeaders
    );
}

#[test]
fn accepts_exact_limits_empty_values_and_arbitrary_body_bytes() {
    let input = b"HTTP/1.1 599 \r\nEmpty:\t \r\n\r\nbody\n\r\0\xff";
    let head_end = input
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap()
        + 4;
    let status_line_length = b"HTTP/1.1 599 ".len();
    let exact = Http1ResponseHeadLimits::new(head_end, status_line_length, 1).unwrap();
    let head = decode_http1_response_head(input, exact).unwrap();
    assert_eq!(head.status_code(), 599);
    assert_eq!(head.reason_phrase(), b"");
    assert_eq!(head.headers()[0].value(), b"");
    assert_eq!(head.encoded_head().len(), head_end);
    assert_eq!(head.body_tail(), b"body\n\r\0\xff");
}

#[test]
fn distinguishes_status_line_shape_from_version_and_code_failures() {
    let cases: &[(&[u8], Http1ResponseHeadError)] = &[
        (b"\r\n\r\n", Http1ResponseHeadError::InvalidStatusLine),
        (
            b"HTTP/1.1\t200 OK\r\n\r\n",
            Http1ResponseHeadError::InvalidStatusLine,
        ),
        (
            b"HTTP/1.1 200\tOK\r\n\r\n",
            Http1ResponseHeadError::InvalidStatusLine,
        ),
        (
            b"http/1.1 200 OK\r\n\r\n",
            Http1ResponseHeadError::InvalidVersion,
        ),
        (
            b"HTTP/1.1 20 OK\r\n\r\n",
            Http1ResponseHeadError::InvalidStatusLine,
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(error(input), *expected);
    }
}

#[test]
fn display_messages_cover_every_payload_free_category() {
    let errors = [
        Http1ResponseHeadError::InvalidLimits,
        Http1ResponseHeadError::IncompleteHead,
        Http1ResponseHeadError::HeadTooLong,
        Http1ResponseHeadError::InvalidLineEnding,
        Http1ResponseHeadError::LineTooLong,
        Http1ResponseHeadError::InvalidVersion,
        Http1ResponseHeadError::InvalidStatusLine,
        Http1ResponseHeadError::InvalidStatusCode,
        Http1ResponseHeadError::InvalidReasonPhrase,
        Http1ResponseHeadError::TooManyHeaders,
        Http1ResponseHeadError::ObsoleteLineFolding,
        Http1ResponseHeadError::MissingHeaderColon,
        Http1ResponseHeadError::EmptyHeaderName,
        Http1ResponseHeadError::InvalidHeaderName,
        Http1ResponseHeadError::InvalidHeaderValue,
    ];
    for error in errors {
        let message = error.to_string();
        assert!(!message.is_empty());
        assert!(!message.contains("secret"));
    }
}
