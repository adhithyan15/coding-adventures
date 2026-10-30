use http1_empty_trailer_section::{
    decode_http1_empty_trailer_section, Http1EmptyTrailerSectionError,
};

#[test]
fn decodes_exact_empty_section_and_borrowed_remainder() {
    let section = decode_http1_empty_trailer_section(b"\r\nnext").unwrap();
    assert_eq!(section.encoded_section(), b"\r\n");
    assert_eq!(section.remainder(), b"next");
    let debug = format!("{section:?}");
    assert!(!debug.contains("next"));
    assert!(debug.contains("[REDACTED]"));
}

#[test]
fn distinguishes_incomplete_and_invalid_line_endings() {
    for (input, expected) in [
        (&b""[..], Http1EmptyTrailerSectionError::IncompleteSection),
        (&b"\r"[..], Http1EmptyTrailerSectionError::IncompleteSection),
        (&b"\n"[..], Http1EmptyTrailerSectionError::InvalidLineEnding),
        (
            &b"\rX"[..],
            Http1EmptyTrailerSectionError::InvalidLineEnding,
        ),
    ] {
        assert_eq!(
            decode_http1_empty_trailer_section(input).unwrap_err(),
            expected
        );
    }
}

#[test]
fn rejects_trailer_fields_without_waiting_for_their_terminator() {
    for input in [&b"X"[..], &b"Name: value\r\n\r\n"[..]] {
        assert_eq!(
            decode_http1_empty_trailer_section(input).unwrap_err(),
            Http1EmptyTrailerSectionError::TrailerFieldsUnsupported
        );
    }
}

#[test]
fn stable_errors_never_echo_input() {
    for error in [
        Http1EmptyTrailerSectionError::IncompleteSection,
        Http1EmptyTrailerSectionError::InvalidLineEnding,
        Http1EmptyTrailerSectionError::TrailerFieldsUnsupported,
    ] {
        assert!(!error.to_string().is_empty());
        assert!(!format!("{error:?}").contains("secret"));
    }
}
