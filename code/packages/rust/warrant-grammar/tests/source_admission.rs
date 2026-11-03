//! Authored raw coordinates are the oracle: normalized offsets are never reused
//! as original offsets. Tiny exhaustive ranges complement the named witnesses.
use coding_adventures_warrant_grammar::source::{
    OriginalPosition, SourceDocument, SourceErrorKind, SourceLimits,
};

fn admit(bytes: &[u8]) -> SourceDocument<'_> {
    SourceDocument::admit(bytes, SourceLimits::default()).unwrap()
}

#[test]
fn lf_input_borrows_the_exact_original_buffer() {
    for text in ["", "warrant x", "a\n", "\n\n", "-- λ😀\n"] {
        let doc = admit(text.as_bytes());
        assert_eq!(doc.original(), text);
        assert_eq!(doc.normalized(), text);
        assert_eq!(doc.original().as_ptr(), doc.normalized().as_ptr());
        assert!(doc.is_normalized_borrowed());
        assert!(doc.owned_buffer_bytes() <= SourceLimits::default().max_owned_bytes);
    }
}

#[test]
fn mixed_unicode_coordinates_are_original_bytes_and_one_based() {
    let doc = admit("aλ\r\n😀z\nq\r\n".as_bytes());
    assert_eq!(doc.normalized(), "aλ\n😀z\nq\n");
    assert!(!doc.is_normalized_borrowed());
    // normalized boundary, original boundary, original line, byte-column
    for (n, byte_offset, line, byte_column) in [
        (0, 0, 1, 1),
        (1, 1, 1, 2),
        (3, 3, 1, 4),
        (4, 5, 2, 1),
        (8, 9, 2, 5),
        (9, 10, 2, 6),
        (10, 11, 3, 1),
        (11, 12, 3, 2),
        (12, 14, 4, 1),
    ] {
        let point = doc.original_span(n, n).unwrap();
        assert_eq!(
            point.start,
            OriginalPosition {
                byte_offset,
                line,
                byte_column
            }
        );
        assert_eq!(point.start, point.end);
    }
    let newline = doc.original_span(3, 4).unwrap();
    assert_eq!(
        &doc.original()[newline.start.byte_offset..newline.end.byte_offset],
        "\r\n"
    );
    let word = doc.original_span(4, 9).unwrap();
    assert_eq!(
        &doc.original()[word.start.byte_offset..word.end.byte_offset],
        "😀z"
    );
    let whole = doc.original_span(0, 12).unwrap();
    assert_eq!(whole.end.byte_offset, doc.original().len());
}

#[test]
fn every_scalar_range_retains_its_raw_spelling() {
    for raw in [
        "",
        "x",
        "\r\n",
        "\n\r\n\n",
        "λ\r\n😀x\nq",
        "\"A\\nB\"\r\n  -- λ😀\n",
    ] {
        let doc = admit(raw.as_bytes());
        // Independent sequential oracle: CRLF is one unit; all other scalars
        // keep their spelling. No line-start index or subject mapping is used.
        let mut chars = raw.char_indices().peekable();
        let mut points = vec![(0, 0, 1, 1)];
        let (mut normalized, mut line, mut column) = (0, 1, 1);
        while let Some((offset, ch)) = chars.next() {
            let mut raw_end = offset + ch.len_utf8();
            if ch == '\r' {
                let (lf, _) = chars.next().unwrap();
                raw_end = lf + 1;
            }
            normalized += if ch == '\r' { 1 } else { ch.len_utf8() };
            if matches!(ch, '\r' | '\n') {
                line += 1;
                column = 1;
            } else {
                column += ch.len_utf8();
            }
            points.push((normalized, raw_end, line, column));
        }
        for &(start, raw_start, line, byte_column) in &points {
            let expected = OriginalPosition {
                byte_offset: raw_start,
                line,
                byte_column,
            };
            for &(end, raw_end, _, _) in &points {
                if end < start {
                    continue;
                }
                let span = doc.original_span(start, end).unwrap();
                assert_eq!(span.start, expected);
                assert_eq!(span.end.byte_offset, raw_end);
                assert_eq!(
                    raw[raw_start..raw_end].replace("\r\n", "\n"),
                    doc.normalized()[start..end]
                );
            }
        }
    }
}

#[test]
fn range_queries_reject_bad_utf8_boundaries_and_numbers() {
    let doc = admit("λ\r\n😀".as_bytes());
    for (start, end) in [
        (1, 1),
        (4, 4),
        (5, 6),
        (0, 8),
        (7, 0),
        (usize::MAX, usize::MAX),
    ] {
        assert!(doc.original_span(start, end).is_none());
    }
    assert!(doc.original_span(7, 7).is_some());
}

#[test]
fn unicode_line_separators_do_not_redefine_warrant_line_coordinates() {
    let doc = admit("x\u{2028}\u{2029}λ".as_bytes());
    let end = doc.original_span(9, 9).unwrap().end;
    assert_eq!(
        end,
        OriginalPosition {
            byte_offset: 9,
            line: 1,
            byte_column: 10
        }
    );
    // An interior U+FEFF is ordinary source for the later lexer to classify.
    assert_eq!(admit("x\u{feff}".as_bytes()).normalized(), "x\u{feff}");
}

#[test]
fn forbidden_source_has_closed_kinds_and_exact_raw_intervals() {
    let cases: &[(&[u8], SourceErrorKind, usize, usize)] = &[
        (&[0xff], SourceErrorKind::Utf8, 0, 1),
        (&[b'a', 0xe2, b'('], SourceErrorKind::Utf8, 1, 2),
        (&[b'a', 0xf0, 0x9f], SourceErrorKind::Utf8, 1, 3),
        (b"\xef\xbb\xbfx", SourceErrorKind::Bom, 0, 3),
        (b"\0", SourceErrorKind::Nul, 0, 1),
        (b"a\r\nb\0", SourceErrorKind::Nul, 4, 5),
        (b"\r", SourceErrorKind::BareCr, 0, 1),
        (b"x\ry", SourceErrorKind::BareCr, 1, 2),
        (b"\r\r\n", SourceErrorKind::BareCr, 0, 1),
    ];
    for &(bytes, kind, start, end) in cases {
        let err = SourceDocument::admit(bytes, SourceLimits::default()).unwrap_err();
        assert_eq!(err.kind(), kind);
        assert_eq!(err.code(), "W0001");
        assert_eq!(err.byte_range(), Some(start..end));
    }
}

#[test]
fn each_configuration_ceiling_is_enforced_before_empty_input() {
    let base = SourceLimits::default();
    for limits in [
        SourceLimits {
            max_source_bytes: base.max_source_bytes + 1,
            ..base
        },
        SourceLimits {
            max_owned_bytes: base.max_owned_bytes + 1,
            ..base
        },
        SourceLimits {
            max_scan_work: base.max_scan_work + 1,
            ..base
        },
        SourceLimits {
            max_source_bytes: usize::MAX,
            ..base
        },
    ] {
        let err = SourceDocument::admit(b"", limits).unwrap_err();
        assert_eq!(err.kind(), SourceErrorKind::InvalidLimits);
        assert_eq!(err.byte_range(), None);
    }
}

#[test]
fn byte_and_work_limits_have_exact_lowered_boundaries() {
    let base = SourceLimits::default();
    assert!(SourceDocument::admit(
        b"abc",
        SourceLimits {
            max_source_bytes: 3,
            max_scan_work: 9,
            ..base
        }
    )
    .is_ok());
    for (limits, kind) in [
        (
            SourceLimits {
                max_source_bytes: 2,
                ..base
            },
            SourceErrorKind::SourceBytes,
        ),
        (
            SourceLimits {
                max_scan_work: 8,
                ..base
            },
            SourceErrorKind::Work,
        ),
        (
            SourceLimits {
                max_scan_work: 0,
                ..base
            },
            SourceErrorKind::Work,
        ),
    ] {
        let err = SourceDocument::admit(b"abc", limits).unwrap_err();
        assert_eq!(err.kind(), kind);
        assert!(err.byte_range().is_none());
    }
    assert!(SourceDocument::admit(
        b"",
        SourceLimits {
            max_source_bytes: 0,
            max_scan_work: 0,
            ..base
        }
    )
    .is_ok());
}

#[test]
fn capacity_limits_include_the_line_index_and_normalized_buffer() {
    for bytes in [b"".as_slice(), b"a\nb", b"a\r\nb\r\n"] {
        let owned = admit(bytes).owned_buffer_bytes();
        assert!(owned >= 2 * std::mem::size_of::<usize>());
        let base = SourceLimits::default();
        assert!(SourceDocument::admit(
            bytes,
            SourceLimits {
                max_owned_bytes: owned,
                ..base
            }
        )
        .is_ok());
        let err = SourceDocument::admit(
            bytes,
            SourceLimits {
                max_owned_bytes: owned - 1,
                ..base
            },
        )
        .unwrap_err();
        assert_eq!(err.kind(), SourceErrorKind::OwnedBytes);
        assert_eq!(err.byte_range(), None);
    }
    let err = SourceDocument::admit(
        b"",
        SourceLimits {
            max_owned_bytes: 0,
            ..SourceLimits::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.kind(), SourceErrorKind::OwnedBytes);
}

#[test]
fn full_source_byte_and_dense_newline_boundaries_remain_bounded() {
    let cap = SourceLimits::default().max_source_bytes;
    let dense = vec![b'\n'; cap];
    let doc = admit(&dense);
    assert_eq!(doc.original_span(cap, cap).unwrap().end.line, cap + 1);
    assert!(doc.owned_buffer_bytes() <= SourceLimits::default().max_owned_bytes);
    let over = vec![0xff; cap + 1];
    let err = SourceDocument::admit(&over, SourceLimits::default()).unwrap_err();
    // Admission happens before scanning even a deliberately invalid buffer.
    assert_eq!(err.kind(), SourceErrorKind::SourceBytes);
}

#[test]
fn diagnostics_and_document_debug_never_reproduce_source() {
    let marker = "PRIVATE_WARRANT_λ_😀";
    let doc = admit(marker.as_bytes());
    assert!(!format!("{doc:?}").contains(marker));
    let malformed = format!("{marker}\0");
    let err = SourceDocument::admit(malformed.as_bytes(), SourceLimits::default()).unwrap_err();
    for public in [format!("{err}"), format!("{err:?}")] {
        assert!(!public.contains(marker));
        assert!(!public.contains('λ'));
    }
    assert!(format!("{err}").starts_with("W0001:"));
    let _: &dyn std::error::Error = &err;
    assert_eq!(err.clone(), err);
}
