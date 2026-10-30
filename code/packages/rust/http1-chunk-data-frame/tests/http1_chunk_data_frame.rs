use http1_chunk_data_frame::{
    decode_http1_chunk_data_frame, Http1ChunkDataFrameError, Http1ChunkDataFrameLimits,
};

fn limits() -> Http1ChunkDataFrameLimits {
    Http1ChunkDataFrameLimits::new(1024).unwrap()
}

#[test]
fn decodes_exact_borrowed_data_frame_and_remainder() {
    let input = b"secret\r\nnext";
    let frame = decode_http1_chunk_data_frame(input, 6, limits()).unwrap();
    assert_eq!(frame.chunk_data(), b"secret");
    assert_eq!(frame.encoded_frame(), b"secret\r\n");
    assert_eq!(frame.remainder(), b"next");
    let debug = format!("{frame:?}");
    assert!(debug.contains("chunk_bytes: 6"));
    assert!(!debug.contains("secret"));
    assert!(!debug.contains("next"));
}

#[test]
fn validates_and_exposes_positive_limit() {
    assert_eq!(
        Http1ChunkDataFrameLimits::new(0).unwrap_err(),
        Http1ChunkDataFrameError::InvalidLimits
    );
    let limits = Http1ChunkDataFrameLimits::new(17).unwrap();
    assert_eq!(limits.maximum_chunk_bytes(), 17);
}

#[test]
fn rejects_terminal_chunk_before_consuming_trailer_bytes() {
    assert_eq!(
        decode_http1_chunk_data_frame(b"\r\nHeader: value\r\n\r\n", 0, limits()).unwrap_err(),
        Http1ChunkDataFrameError::TerminalChunkRequiresSeparateParsing
    );
}

#[test]
fn enforces_caller_limit_before_input_length() {
    assert_eq!(
        decode_http1_chunk_data_frame(b"", 5, Http1ChunkDataFrameLimits::new(4).unwrap(),)
            .unwrap_err(),
        Http1ChunkDataFrameError::ChunkTooLarge
    );
}

#[test]
fn rejects_encoded_length_overflow_before_indexing() {
    assert_eq!(
        decode_http1_chunk_data_frame(
            b"",
            usize::MAX,
            Http1ChunkDataFrameLimits::new(usize::MAX).unwrap(),
        )
        .unwrap_err(),
        Http1ChunkDataFrameError::EncodedFrameLengthOverflow
    );
}

#[test]
fn distinguishes_incomplete_data_and_terminator() {
    for (input, expected) in [
        (&b"abc"[..], Http1ChunkDataFrameError::IncompleteChunkData),
        (
            &b"abcd"[..],
            Http1ChunkDataFrameError::IncompleteChunkTerminator,
        ),
        (
            &b"abcd\r"[..],
            Http1ChunkDataFrameError::IncompleteChunkTerminator,
        ),
    ] {
        assert_eq!(
            decode_http1_chunk_data_frame(input, 4, limits()).unwrap_err(),
            expected
        );
    }
}

#[test]
fn requires_exact_crlf_after_declared_data() {
    for input in [
        &b"abcdX"[..],
        &b"abcd\n\r"[..],
        &b"abcd\rX"[..],
        &b"abcdXX"[..],
    ] {
        assert_eq!(
            decode_http1_chunk_data_frame(input, 4, limits()).unwrap_err(),
            Http1ChunkDataFrameError::InvalidChunkTerminator
        );
    }
}

#[test]
fn stable_errors_never_echo_input() {
    let errors = [
        Http1ChunkDataFrameError::InvalidLimits,
        Http1ChunkDataFrameError::TerminalChunkRequiresSeparateParsing,
        Http1ChunkDataFrameError::ChunkTooLarge,
        Http1ChunkDataFrameError::EncodedFrameLengthOverflow,
        Http1ChunkDataFrameError::IncompleteChunkData,
        Http1ChunkDataFrameError::IncompleteChunkTerminator,
        Http1ChunkDataFrameError::InvalidChunkTerminator,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
        assert!(!format!("{error:?}").contains("secret"));
    }
}
