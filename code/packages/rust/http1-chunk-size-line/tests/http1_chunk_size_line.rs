use http1_chunk_size_line::{
    decode_http1_chunk_size_line, Http1ChunkSizeLineError, Http1ChunkSizeLineLimits,
    MAX_HTTP1_CHUNK_SIZE_LINE_BYTES,
};

fn limits() -> Http1ChunkSizeLineLimits {
    Http1ChunkSizeLineLimits::new(32, 4096).unwrap()
}

#[test]
fn decodes_zero_upper_lower_and_borrowed_remainder() {
    for (input, expected, remainder) in [
        (&b"0\r\ntrailers"[..], 0, &b"trailers"[..]),
        (&b"a\r\n0123456789"[..], 10, &b"0123456789"[..]),
        (&b"FF\r\nbody"[..], 255, &b"body"[..]),
        (&b"000f\r\nbody"[..], 15, &b"body"[..]),
    ] {
        let line = decode_http1_chunk_size_line(input, limits()).unwrap();
        let end = input.len() - remainder.len();
        assert_eq!(line.chunk_size(), expected);
        assert_eq!(line.encoded_line(), &input[..end]);
        assert_eq!(line.remainder(), remainder);
        assert!(!format!("{line:?}").contains("body"));
    }
}

#[test]
fn validates_and_exposes_limits() {
    for line_limit in [0, MAX_HTTP1_CHUNK_SIZE_LINE_BYTES + 1] {
        assert_eq!(
            Http1ChunkSizeLineLimits::new(line_limit, 1).unwrap_err(),
            Http1ChunkSizeLineError::InvalidLimits
        );
    }
    let limits = Http1ChunkSizeLineLimits::new(7, 9).unwrap();
    assert_eq!(limits.maximum_line_bytes(), 7);
    assert_eq!(limits.maximum_chunk_bytes(), 9);
}

#[test]
fn enforces_exact_line_limit_and_chunk_limit() {
    let exact = Http1ChunkSizeLineLimits::new(2, 255).unwrap();
    assert_eq!(
        decode_http1_chunk_size_line(b"ff\r\n", exact)
            .unwrap()
            .chunk_size(),
        255
    );
    assert_eq!(
        decode_http1_chunk_size_line(b"100\r\n", exact).unwrap_err(),
        Http1ChunkSizeLineError::LineTooLong
    );
    assert_eq!(
        decode_http1_chunk_size_line(b"100\r\n", Http1ChunkSizeLineLimits::new(32, 255).unwrap(),)
            .unwrap_err(),
        Http1ChunkSizeLineError::ChunkTooLarge
    );
}

#[test]
fn enforces_repository_line_ceiling_before_value_parsing() {
    let limits = Http1ChunkSizeLineLimits::new(MAX_HTTP1_CHUNK_SIZE_LINE_BYTES, 1).unwrap();
    let exact = format!("{}1\r\n", "0".repeat(MAX_HTTP1_CHUNK_SIZE_LINE_BYTES - 1));
    assert_eq!(
        decode_http1_chunk_size_line(exact.as_bytes(), limits)
            .unwrap()
            .chunk_size(),
        1
    );

    let overlong = format!("{}\r\n", "0".repeat(MAX_HTTP1_CHUNK_SIZE_LINE_BYTES + 1));
    assert_eq!(
        decode_http1_chunk_size_line(overlong.as_bytes(), limits).unwrap_err(),
        Http1ChunkSizeLineError::LineTooLong
    );
}

#[test]
fn rejects_incomplete_or_non_crlf_lines() {
    for (input, expected) in [
        (&b""[..], Http1ChunkSizeLineError::IncompleteLine),
        (&b"4"[..], Http1ChunkSizeLineError::IncompleteLine),
        (&b"4\r"[..], Http1ChunkSizeLineError::IncompleteLine),
        (&b"4\n"[..], Http1ChunkSizeLineError::InvalidLineEnding),
        (&b"4\rx"[..], Http1ChunkSizeLineError::InvalidLineEnding),
    ] {
        assert_eq!(
            decode_http1_chunk_size_line(input, limits()).unwrap_err(),
            expected
        );
    }
}

#[test]
fn rejects_empty_invalid_extensions_and_overflow() {
    let overflow = format!("{:x}0\r\n", usize::MAX);
    for (input, expected) in [
        (&b"\r\n"[..], Http1ChunkSizeLineError::EmptyChunkSize),
        (&b" 4\r\n"[..], Http1ChunkSizeLineError::InvalidChunkSize),
        (&b"+4\r\n"[..], Http1ChunkSizeLineError::InvalidChunkSize),
        (&b"g\r\n"[..], Http1ChunkSizeLineError::InvalidChunkSize),
        (
            &b"4;name=value\r\n"[..],
            Http1ChunkSizeLineError::ChunkExtensionsUnsupported,
        ),
        (
            overflow.as_bytes(),
            Http1ChunkSizeLineError::ChunkSizeOverflow,
        ),
    ] {
        assert_eq!(
            decode_http1_chunk_size_line(input, limits()).unwrap_err(),
            expected
        );
    }
}

#[test]
fn stable_errors_never_echo_input() {
    let errors = [
        Http1ChunkSizeLineError::InvalidLimits,
        Http1ChunkSizeLineError::IncompleteLine,
        Http1ChunkSizeLineError::LineTooLong,
        Http1ChunkSizeLineError::InvalidLineEnding,
        Http1ChunkSizeLineError::EmptyChunkSize,
        Http1ChunkSizeLineError::InvalidChunkSize,
        Http1ChunkSizeLineError::ChunkSizeOverflow,
        Http1ChunkSizeLineError::ChunkTooLarge,
        Http1ChunkSizeLineError::ChunkExtensionsUnsupported,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
        assert!(!format!("{error:?}").contains("secret"));
    }
}
