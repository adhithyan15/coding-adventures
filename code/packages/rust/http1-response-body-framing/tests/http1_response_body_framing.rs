use http1_response_body_framing::{
    classify_http1_response_body, CloseDelimitedPolicy, Http1ResponseBodyFraming,
    Http1ResponseBodyFramingError, Http1ResponseBodyFramingPolicy, Http1ResponseFramingHeader,
    Http1ResponseRequestSemantics,
};

fn bounded() -> Http1ResponseBodyFramingPolicy {
    Http1ResponseBodyFramingPolicy::new(16, CloseDelimitedPolicy::Reject)
}

fn header<'a>(name: &'a [u8], value: &'a [u8]) -> Http1ResponseFramingHeader<'a> {
    Http1ResponseFramingHeader::new(name, value)
}

#[test]
fn classifies_canonical_fixed_and_zero_lengths() {
    for (value, expected) in [
        (&b"0"[..], Http1ResponseBodyFraming::ContentLength(0)),
        (&b"16"[..], Http1ResponseBodyFraming::ContentLength(16)),
        (&b" \t7\t "[..], Http1ResponseBodyFraming::ContentLength(7)),
    ] {
        assert_eq!(
            classify_http1_response_body(
                Http1ResponseRequestSemantics::Normal,
                200,
                &[header(b"content-length", value)],
                bounded(),
            )
            .unwrap(),
            expected
        );
    }
}

#[test]
fn close_delimited_consumption_is_explicit_and_bounded() {
    let reject =
        classify_http1_response_body(Http1ResponseRequestSemantics::Normal, 200, &[], bounded())
            .unwrap_err();
    assert_eq!(
        reject,
        Http1ResponseBodyFramingError::CloseDelimitedForbidden
    );

    let allow = Http1ResponseBodyFramingPolicy::new(23, CloseDelimitedPolicy::Allow);
    assert_eq!(allow.maximum_body_bytes(), 23);
    assert_eq!(allow.close_delimited(), CloseDelimitedPolicy::Allow);
    assert_eq!(
        classify_http1_response_body(Http1ResponseRequestSemantics::Normal, 200, &[], allow,)
            .unwrap(),
        Http1ResponseBodyFraming::UntilClose {
            maximum_body_bytes: 23
        }
    );
}

#[test]
fn rejects_transfer_encoding_and_content_length_ambiguity() {
    for headers in [
        vec![header(b"Transfer-Encoding", b"chunked")],
        vec![
            header(b"Content-Length", b"4"),
            header(b"Transfer-Encoding", b"chunked"),
        ],
    ] {
        assert_eq!(
            classify_http1_response_body(
                Http1ResponseRequestSemantics::Normal,
                200,
                &headers,
                bounded(),
            )
            .unwrap_err(),
            Http1ResponseBodyFramingError::TransferEncodingForbidden
        );
    }

    assert_eq!(
        classify_http1_response_body(
            Http1ResponseRequestSemantics::Normal,
            200,
            &[
                header(b"Content-Length", b"4"),
                header(b"content-length", b"4"),
            ],
            bounded(),
        )
        .unwrap_err(),
        Http1ResponseBodyFramingError::DuplicateContentLength
    );
}

#[test]
fn rejects_invalid_noncanonical_oversized_or_overflowing_lengths() {
    let overflow = format!("{}0", usize::MAX);
    for (value, expected) in [
        (
            &b""[..],
            Http1ResponseBodyFramingError::InvalidContentLength,
        ),
        (
            &b"1, 1"[..],
            Http1ResponseBodyFramingError::InvalidContentLength,
        ),
        (
            &b"+1"[..],
            Http1ResponseBodyFramingError::InvalidContentLength,
        ),
        (
            &b"01"[..],
            Http1ResponseBodyFramingError::NonCanonicalContentLength,
        ),
        (
            overflow.as_bytes(),
            Http1ResponseBodyFramingError::InvalidContentLength,
        ),
        (
            &b"17"[..],
            Http1ResponseBodyFramingError::ContentLengthTooLarge,
        ),
    ] {
        assert_eq!(
            classify_http1_response_body(
                Http1ResponseRequestSemantics::Normal,
                200,
                &[header(b"Content-Length", value)],
                bounded(),
            )
            .unwrap_err(),
            expected
        );
    }
}

#[test]
fn enforces_bodyless_status_and_request_semantics() {
    for status in [100, 199, 204] {
        assert_eq!(
            classify_http1_response_body(
                Http1ResponseRequestSemantics::Normal,
                status,
                &[],
                bounded(),
            )
            .unwrap(),
            Http1ResponseBodyFraming::NoBody
        );
        assert_eq!(
            classify_http1_response_body(
                Http1ResponseRequestSemantics::Normal,
                status,
                &[header(b"Content-Length", b"0")],
                bounded(),
            )
            .unwrap_err(),
            Http1ResponseBodyFramingError::ContentLengthForbidden
        );
        assert_eq!(
            classify_http1_response_body(
                Http1ResponseRequestSemantics::Head,
                status,
                &[header(b"Content-Length", b"0")],
                bounded(),
            )
            .unwrap_err(),
            Http1ResponseBodyFramingError::ContentLengthForbidden
        );
    }

    for headers in [vec![], vec![header(b"Content-Length", b"999")]] {
        assert_eq!(
            classify_http1_response_body(
                Http1ResponseRequestSemantics::Head,
                200,
                &headers,
                bounded(),
            )
            .unwrap(),
            Http1ResponseBodyFraming::NoBody
        );
        assert_eq!(
            classify_http1_response_body(
                Http1ResponseRequestSemantics::Normal,
                304,
                &headers,
                bounded(),
            )
            .unwrap(),
            Http1ResponseBodyFraming::NoBody
        );
    }

    assert_eq!(
        classify_http1_response_body(
            Http1ResponseRequestSemantics::Normal,
            205,
            &[header(b"Content-Length", b"0")],
            bounded(),
        )
        .unwrap(),
        Http1ResponseBodyFraming::ContentLength(0)
    );
    assert_eq!(
        classify_http1_response_body(Http1ResponseRequestSemantics::Normal, 205, &[], bounded(),)
            .unwrap_err(),
        Http1ResponseBodyFramingError::CloseDelimitedForbidden
    );
}

#[test]
fn rejects_successful_connect_tunnels_but_frames_failed_connect() {
    assert_eq!(
        classify_http1_response_body(Http1ResponseRequestSemantics::Connect, 200, &[], bounded(),)
            .unwrap_err(),
        Http1ResponseBodyFramingError::SuccessfulConnectUnsupported
    );
    assert_eq!(
        classify_http1_response_body(
            Http1ResponseRequestSemantics::Connect,
            407,
            &[header(b"Content-Length", b"4")],
            bounded(),
        )
        .unwrap(),
        Http1ResponseBodyFraming::ContentLength(4)
    );
}

#[test]
fn validates_status_and_all_header_syntax_before_classifying() {
    for status in [0, 99, 600, u16::MAX] {
        assert_eq!(
            classify_http1_response_body(
                Http1ResponseRequestSemantics::Normal,
                status,
                &[],
                bounded(),
            )
            .unwrap_err(),
            Http1ResponseBodyFramingError::InvalidStatusCode
        );
    }
    for (field, expected) in [
        (
            header(b"", b"x"),
            Http1ResponseBodyFramingError::InvalidHeaderName,
        ),
        (
            header(b"Bad Name", b"x"),
            Http1ResponseBodyFramingError::InvalidHeaderName,
        ),
        (
            header(b"X-Test", b"bad\x7f"),
            Http1ResponseBodyFramingError::InvalidHeaderValue,
        ),
    ] {
        assert_eq!(
            classify_http1_response_body(
                Http1ResponseRequestSemantics::Normal,
                200,
                &[field],
                bounded(),
            )
            .unwrap_err(),
            expected
        );
    }
}

#[test]
fn borrowed_headers_are_redacted_and_payload_free_errors_are_stable() {
    let field = header(b"X-Secret", b"token-value");
    assert_eq!(field.name(), b"X-Secret");
    assert_eq!(field.value(), b"token-value");
    let debug = format!("{field:?}");
    assert!(!debug.contains("X-Secret"));
    assert!(!debug.contains("token-value"));

    let errors = [
        Http1ResponseBodyFramingError::InvalidStatusCode,
        Http1ResponseBodyFramingError::InvalidHeaderName,
        Http1ResponseBodyFramingError::InvalidHeaderValue,
        Http1ResponseBodyFramingError::TransferEncodingForbidden,
        Http1ResponseBodyFramingError::DuplicateContentLength,
        Http1ResponseBodyFramingError::InvalidContentLength,
        Http1ResponseBodyFramingError::NonCanonicalContentLength,
        Http1ResponseBodyFramingError::ContentLengthTooLarge,
        Http1ResponseBodyFramingError::ContentLengthForbidden,
        Http1ResponseBodyFramingError::CloseDelimitedForbidden,
        Http1ResponseBodyFramingError::SuccessfulConnectUnsupported,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
        assert!(!format!("{error:?}").contains("token-value"));
    }
}
