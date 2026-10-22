//! CV04a validates the actual cursor and its independently replayable partition.
//! Expectations use raw source slices and explicit coordinates, never token values.
use grammar_tools::token_grammar::{parse_token_grammar, TokenGrammar};
use lexer::grammar_lexer::GrammarLexer;
use lexer::lexical::{validate_lexical_partition, LexicalLimits, LexicalPieceKind, SourcePosition};
use lexer::token::TokenType;
use std::cell::Cell;
use std::rc::Rc;

fn grammar() -> TokenGrammar {
    parse_token_grammar(
        r#"NAME = /[\p{L}_]+/
STRING = /"([^"\\]|\\.)*"/
NUMBER = /[0-9]+/
PLUS = "+"
skip:
  WS = /[ \t\r\n\u2028\u2029]+/
  COMMENT = /\/\*[^*]*\*\//
"#,
    )
    .unwrap()
}
fn position(b: usize, s: usize, u: usize, l: usize, sc: usize, uc: usize) -> SourcePosition {
    SourcePosition {
        byte_offset: b,
        scalar_offset: s,
        utf16_offset: u,
        line: l,
        scalar_column: sc,
        utf16_column: uc,
    }
}

#[test]
fn raw_quotes_escapes_and_trivia_partition_the_exact_source() {
    let src = " /*note*/ \"A\\nB\" + \"λ😀\" \r\n";
    let g = grammar();
    let legacy = GrammarLexer::new(src, &g).tokenize().unwrap();
    let snap =
        GrammarLexer::tokenize_source_with_ranges(src, &g, LexicalLimits::default()).unwrap();
    assert_eq!(snap.source(), src);
    assert_eq!(snap.tokens(), legacy);
    let raw: Vec<_> = snap
        .pieces()
        .iter()
        .map(|p| &src[p.range.start.byte_offset..p.range.end.byte_offset])
        .collect();
    assert_eq!(
        raw,
        [
            " ",
            "/*note*/",
            " ",
            "\"A\\nB\"",
            " ",
            "+",
            " ",
            "\"λ😀\"",
            " \r\n",
            ""
        ]
    );
    assert_eq!(snap.tokens()[0].value, "A\nB");
    assert_eq!(raw[3].len(), 6);
    assert_eq!(raw[7].len(), 8);
    assert_eq!(snap.tokens()[1].type_, TokenType::Plus);
    validate_lexical_partition(
        src,
        snap.tokens().len(),
        snap.pieces(),
        LexicalLimits::default(),
    )
    .unwrap();
    assert_eq!(snap.into_tokens(), legacy);
}

#[test]
fn ecmascript_coordinates_are_distinct_from_legacy_locations() {
    let src = "\"😀\"\r\nλ\rx\u{2028}y\u{2029}z";
    let snap = GrammarLexer::tokenize_source_with_ranges(src, &grammar(), LexicalLimits::default())
        .unwrap();
    let starts: Vec<_> = snap
        .pieces()
        .iter()
        .filter_map(|p| match p.kind {
            LexicalPieceKind::Token(_) | LexicalPieceKind::Eof(_) => Some(p.range.start),
            _ => None,
        })
        .collect();
    assert_eq!(
        starts,
        [
            position(0, 0, 0, 0, 0, 0),
            position(8, 5, 6, 1, 0, 0),
            position(11, 7, 8, 2, 0, 0),
            position(15, 9, 10, 3, 0, 0),
            position(19, 11, 12, 4, 0, 0),
            position(20, 12, 13, 4, 1, 1)
        ]
    );
    assert_eq!(snap.pieces()[0].range.end, position(6, 3, 4, 0, 3, 4));
    // Legacy updates its line only on LF; capture must not rewrite these fields.
    assert_eq!(snap.tokens()[2].line, 2);
    assert_eq!(snap.tokens()[2].column, 3);
    validate_lexical_partition(
        src,
        snap.tokens().len(),
        snap.pieces(),
        LexicalLimits::default(),
    )
    .unwrap();
}

#[test]
fn fallback_whitespace_newline_and_empty_eof_are_real_pieces() {
    let g = parse_token_grammar("NAME = /[a-z]+/").unwrap();
    for src in ["", " \t\r", "a\nb"] {
        let snap =
            GrammarLexer::tokenize_source_with_ranges(src, &g, LexicalLimits::default()).unwrap();
        assert_eq!(
            snap.tokens(),
            GrammarLexer::new(src, &g).tokenize().unwrap()
        );
        let eof = snap.pieces().last().unwrap();
        assert_eq!(eof.kind, LexicalPieceKind::Eof(snap.tokens().len() - 1));
        assert_eq!(eof.range.start, eof.range.end);
        assert_eq!(eof.range.end.byte_offset, src.len());
        validate_lexical_partition(
            src,
            snap.tokens().len(),
            snap.pieces(),
            LexicalLimits::default(),
        )
        .unwrap();
    }
    let snap =
        GrammarLexer::tokenize_source_with_ranges("a\nb", &g, LexicalLimits::default()).unwrap();
    assert_eq!(snap.tokens()[1].type_, TokenType::Newline);
    assert_eq!(snap.pieces()[1].kind, LexicalPieceKind::Token(1));
}

#[test]
fn exact_caps_pass_and_one_less_caps_reject() {
    let src = "a + b";
    let g = grammar();
    let snap =
        GrammarLexer::tokenize_source_with_ranges(src, &g, LexicalLimits::default()).unwrap();
    let n = snap.pieces().len();
    let bytes = n * std::mem::size_of_val(&snap.pieces()[0]);
    let exact = LexicalLimits {
        max_source_bytes: src.len(),
        max_pieces: n,
        max_retained_bytes: bytes,
        max_work: src.len() + n,
    };
    GrammarLexer::tokenize_source_with_ranges(src, &g, exact).unwrap();
    validate_lexical_partition(src, snap.tokens().len(), snap.pieces(), exact).unwrap();
    for limits in [
        LexicalLimits {
            max_source_bytes: src.len() - 1,
            ..exact
        },
        LexicalLimits {
            max_pieces: n - 1,
            ..exact
        },
        LexicalLimits {
            max_retained_bytes: bytes - 1,
            ..exact
        },
        LexicalLimits {
            max_work: src.len() + n - 1,
            ..exact
        },
    ] {
        assert!(GrammarLexer::tokenize_source_with_ranges(src, &g, limits).is_err());
        assert!(
            validate_lexical_partition(src, snap.tokens().len(), snap.pieces(), limits).is_err()
        );
    }
    let unsafe_limits = LexicalLimits {
        max_source_bytes: usize::MAX,
        ..exact
    };
    assert!(GrammarLexer::tokenize_source_with_ranges(src, &g, unsafe_limits).is_err());
    assert!(
        validate_lexical_partition(src, snap.tokens().len(), snap.pieces(), unsafe_limits).is_err()
    );
    assert!(GrammarLexer::tokenize_source_with_ranges(
        "",
        &g,
        LexicalLimits {
            max_pieces: 0,
            ..LexicalLimits::default()
        }
    )
    .is_err());
}

#[test]
fn detached_descriptor_corruption_is_rejected_without_mutation() {
    let src = "\"😀\" a";
    let snap = GrammarLexer::tokenize_source_with_ranges(src, &grammar(), LexicalLimits::default())
        .unwrap();
    let pieces = snap.pieces().to_vec();
    let check = |bad: Vec<lexer::lexical::LexicalPiece>| {
        let copy = bad.clone();
        assert!(
            validate_lexical_partition(src, snap.tokens().len(), &bad, LexicalLimits::default())
                .is_err(),
            "{bad:?}"
        );
        assert_eq!(bad, copy);
    };
    for field in 0..6 {
        let mut bad = pieces.clone();
        let p = &mut bad[0].range.end;
        match field {
            0 => p.byte_offset += 1,
            1 => p.scalar_offset += 1,
            2 => p.utf16_offset += 1,
            3 => p.line += 1,
            4 => p.scalar_column += 1,
            _ => p.utf16_column += 1,
        }
        check(bad);
    }
    let mut bad = pieces.clone();
    bad[0].range.end.byte_offset = 2;
    check(bad); // inside astral UTF8
    let mut bad = pieces.clone();
    bad[1].range.start = bad[0].range.start;
    check(bad); // overlap
    let mut bad = pieces.clone();
    bad[0].range.end = bad[0].range.start;
    check(bad); // empty real piece
    let mut bad = pieces.clone();
    bad[0].range.end.byte_offset = usize::MAX;
    check(bad);
    let mut bad = pieces.clone();
    bad[2].kind = LexicalPieceKind::Token(0);
    check(bad);
    let mut bad = pieces.clone();
    bad.pop();
    check(bad);
    let mut bad = pieces.clone();
    bad.remove(1);
    check(bad);
    let mut bad = pieces.clone();
    bad.swap(0, 1);
    check(bad);
    let mut bad = pieces.clone();
    bad[0].kind = LexicalPieceKind::Eof(0);
    check(bad);
    assert!(
        validate_lexical_partition(src, usize::MAX, &pieces, LexicalLimits::default()).is_err()
    );
    assert!(validate_lexical_partition(
        "abc",
        snap.tokens().len(),
        &pieces,
        LexicalLimits::default()
    )
    .is_err());
}

#[test]
fn unsupported_hooks_and_callbacks_never_execute() {
    for mode in 0..3 {
        let called = Rc::new(Cell::new(false));
        let observed = called.clone();
        let g = grammar();
        let mut lexer = GrammarLexer::new("a", &g);
        match mode {
            0 => lexer.add_pre_tokenize(Box::new(move |s| {
                observed.set(true);
                s
            })),
            1 => lexer.add_post_tokenize(Box::new(move |t| {
                observed.set(true);
                t
            })),
            _ => lexer.set_on_token(Some(Box::new(move |_, _| observed.set(true)))),
        }
        assert!(lexer
            .tokenize_with_ranges(LexicalLimits::default())
            .is_err());
        assert!(!called.get());
        lexer.tokenize().unwrap();
        assert!(called.get()); // rejection did not replace or execute the hook
    }
}

#[test]
fn indentation_layout_reuse_and_zero_width_matches_reject() {
    for prefix in ["mode: indentation\n", "mode: layout\n"] {
        let g = parse_token_grammar(&format!("{prefix}NAME = /[a-z]+/\n")).unwrap();
        assert!(
            GrammarLexer::tokenize_source_with_ranges("a", &g, LexicalLimits::default()).is_err()
        );
    }
    let g = grammar();
    let mut lexer = GrammarLexer::new("a", &g);
    lexer.tokenize().unwrap();
    assert!(lexer
        .tokenize_with_ranges(LexicalLimits::default())
        .is_err());
    for definition in ["NAME = /a*/\n", "NAME = /[a-z]+/\nskip:\n  EMPTY = / */\n"] {
        let g = parse_token_grammar(definition).unwrap();
        assert!(
            GrammarLexer::tokenize_source_with_ranges("z", &g, LexicalLimits::default()).is_err()
        );
    }
}

#[test]
fn case_normalization_and_escape_processing_preserve_raw_ranges() {
    let g = parse_token_grammar("# @case_insensitive true\nNAME = /[a-z]+/\nSTRING = /\"([^\"\\\\]|\\\\.)*\"/\nskip:\n  WS = / +/\nkeywords:\n  SELECT\n").unwrap();
    let src = "select \"A\\nB\"";
    let snap =
        GrammarLexer::tokenize_source_with_ranges(src, &g, LexicalLimits::default()).unwrap();
    assert_eq!(snap.tokens()[0].value, "SELECT");
    assert_eq!(snap.tokens()[1].value, "A\nB");
    assert_eq!(
        &src[snap.pieces()[0].range.start.byte_offset..snap.pieces()[0].range.end.byte_offset],
        "select"
    );
    assert_eq!(
        &src[snap.pieces()[2].range.start.byte_offset..snap.pieces()[2].range.end.byte_offset],
        "\"A\\nB\""
    );
}

#[test]
fn crlf_split_across_fallback_pieces_advances_one_line() {
    let g = parse_token_grammar("NAME = /[a-z]+/").unwrap();
    let src = "a\r\nb";
    let snap =
        GrammarLexer::tokenize_source_with_ranges(src, &g, LexicalLimits::default()).unwrap();
    assert_eq!(snap.pieces()[1].kind, LexicalPieceKind::Trivia);
    assert_eq!(snap.pieces()[1].range.end, position(2, 2, 2, 1, 0, 0));
    assert_eq!(snap.pieces()[2].range.end, position(3, 3, 3, 1, 0, 0));
    assert_eq!(snap.pieces()[3].range.start, position(3, 3, 3, 1, 0, 0));
    validate_lexical_partition(
        src,
        snap.tokens().len(),
        snap.pieces(),
        LexicalLimits::default(),
    )
    .unwrap();
}

#[test]
fn capture_failure_discards_partial_evidence_and_cannot_restart_budget() {
    let g = grammar();
    let mut lexer = GrammarLexer::new("a + b", &g);
    let error = lexer
        .tokenize_with_ranges(LexicalLimits {
            max_pieces: 2,
            ..LexicalLimits::default()
        })
        .unwrap_err();
    assert!(matches!(error, lexer::lexical::LexicalError::Limit(_)));
    let error = lexer
        .tokenize_with_ranges(LexicalLimits::default())
        .unwrap_err();
    assert!(matches!(
        error,
        lexer::lexical::LexicalError::Unsupported("lexer reuse")
    ));
    let mut lexer = GrammarLexer::new("a @", &g);
    assert!(matches!(
        lexer
            .tokenize_with_ranges(LexicalLimits::default())
            .unwrap_err(),
        lexer::lexical::LexicalError::Lex(_)
    ));
    assert!(lexer
        .tokenize_with_ranges(LexicalLimits::default())
        .is_err());
}

#[test]
fn every_configuration_ceiling_is_checked_even_for_empty_input() {
    for limits in [
        LexicalLimits {
            max_source_bytes: 128 * 1024 * 1024 + 1,
            ..LexicalLimits::default()
        },
        LexicalLimits {
            max_pieces: 1_048_577,
            ..LexicalLimits::default()
        },
        LexicalLimits {
            max_retained_bytes: 256 * 1024 * 1024 + 1,
            ..LexicalLimits::default()
        },
        LexicalLimits {
            max_work: 512 * 1024 * 1024 + 1,
            ..LexicalLimits::default()
        },
    ] {
        assert!(matches!(
            GrammarLexer::tokenize_source_with_ranges("", &grammar(), limits).unwrap_err(),
            lexer::lexical::LexicalError::Limit("configuration ceiling")
        ));
        assert!(validate_lexical_partition("", 0, &[], limits).is_err());
    }
}

#[test]
fn dense_unicode_partition_has_monotonic_coordinates_and_bound_work() {
    // One-character matches stress descriptor growth; source prefixes are never
    // searched again. Astral lengths alternate with CRLF and Unicode separators.
    let g = parse_token_grammar("CHAR = /(?s:.)/").unwrap();
    let src = "😀λ\r\nx\u{2028}y\u{2029}".repeat(1024);
    let snap =
        GrammarLexer::tokenize_source_with_ranges(&src, &g, LexicalLimits::default()).unwrap();
    assert_eq!(snap.pieces().len(), src.chars().count() + 1);
    assert_eq!(
        snap.tokens(),
        GrammarLexer::new(&src, &g).tokenize().unwrap()
    );
    let eof = snap.pieces().last().unwrap();
    assert_eq!(
        eof.range.end,
        position(
            src.len(),
            src.chars().count(),
            src.encode_utf16().count(),
            3072,
            0,
            0
        )
    );
    validate_lexical_partition(
        &src,
        snap.tokens().len(),
        snap.pieces(),
        LexicalLimits {
            max_work: src.len() + snap.pieces().len(),
            ..LexicalLimits::default()
        },
    )
    .unwrap();
}
