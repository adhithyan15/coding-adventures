//! Bounded significant-layout capture tests (VER04b / P006b2).
use coding_adventures_warrant_grammar::layout::{
    capture_layout, LayoutErrorKind, LayoutLimits, LayoutTokenKind,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

fn admit(source: &str) -> SourceDocument<'_> {
    SourceDocument::admit(source.as_bytes(), SourceLimits::default()).expect("valid source")
}

#[test]
fn flat_document_has_no_indent_dedent() {
    let doc = admit("warrant bitset\nop empty : BitSet\n");
    let tokens = capture_layout(&doc, LayoutLimits::default()).expect("layout captured");

    assert!(tokens
        .iter()
        .all(|t| !matches!(t.kind, LayoutTokenKind::Indent | LayoutTokenKind::Dedent)));

    let words: Vec<_> = tokens
        .iter()
        .filter_map(|t| match &t.kind {
            LayoutTokenKind::Word(s) => Some(*s),
            _ => None,
        })
        .collect();
    assert_eq!(words, ["warrant", "bitset", "op", "empty", "BitSet"]);
}

#[test]
fn nested_blocks_emit_matching_indent_and_dedent() {
    let source = "warrant bitset\n  assume platform:\n    u64.wraps\n";
    let doc = admit(source);
    let tokens = capture_layout(&doc, LayoutLimits::default()).expect("layout captured");

    let indents = tokens
        .iter()
        .filter(|t| matches!(t.kind, LayoutTokenKind::Indent))
        .count();
    let dedents = tokens
        .iter()
        .filter(|t| matches!(t.kind, LayoutTokenKind::Dedent))
        .count();

    assert_eq!(indents, 2, "must emit two indents");
    assert_eq!(dedents, 2, "must emit two matching dedents at EOF");
}

#[test]
fn synthetic_tokens_have_zero_width_original_spans() {
    let source = "assume:\r\n  u64.wraps\r\n";
    let doc = admit(source);
    let tokens = capture_layout(&doc, LayoutLimits::default()).expect("layout captured");

    for token in &tokens {
        if token.is_synthetic() {
            let orig = token
                .original_span(&doc)
                .expect("synthetic token must map to original position");
            assert_eq!(
                orig.start, orig.end,
                "synthetic token must have zero width in original coordinates: {:?}",
                token
            );
        } else {
            let orig = token
                .original_span(&doc)
                .expect("real token must map to original span");
            assert!(
                orig.start.byte_offset < orig.end.byte_offset,
                "real token must have positive original byte width"
            );
        }
    }
}

#[test]
fn tabs_in_indentation_are_strictly_rejected() {
    let source = "assume:\n\tu64.wraps\n";
    let doc = admit(source);
    let err = capture_layout(&doc, LayoutLimits::default()).unwrap_err();

    assert_eq!(err.code(), "W0001");
    assert_eq!(err.kind(), LayoutErrorKind::TabInIndentation);
    assert!(err.byte_range().is_some());
}

#[test]
fn inconsistent_dedent_is_rejected() {
    let source = "assume:\n    block1:\n      nested\n  inconsistent\n";
    let doc = admit(source);
    let err = capture_layout(&doc, LayoutLimits::default()).unwrap_err();

    assert_eq!(err.code(), "W0001");
    assert_eq!(err.kind(), LayoutErrorKind::InconsistentDedent);
}

#[test]
fn max_indent_depth_is_strictly_enforced() {
    let mut source = String::new();
    for i in 1..=10 {
        source.push_str(&"  ".repeat(i));
        source.push_str("level\n");
    }
    let doc = admit(&source);

    let limits = LayoutLimits {
        max_indent_depth: 4,
        max_tokens: 1000,
        max_scan_work: 100_000,
    };
    let err = capture_layout(&doc, limits).unwrap_err();
    assert_eq!(err.kind(), LayoutErrorKind::IndentDepth);
}

#[test]
fn max_tokens_budget_is_enforced() {
    let doc = admit("a b c d e f g h i j k l m n\n");
    let limits = LayoutLimits {
        max_indent_depth: 64,
        max_tokens: 5,
        max_scan_work: 100_000,
    };
    let err = capture_layout(&doc, limits).unwrap_err();
    assert_eq!(err.kind(), LayoutErrorKind::TokenBudget);
}

#[test]
fn comments_and_blank_lines_are_trivia() {
    let source = "warrant test\n\n  -- comment line\n    -- indented comment\n  model X\n";
    let doc = admit(source);
    let tokens = capture_layout(&doc, LayoutLimits::default()).expect("layout captured");

    let indents = tokens
        .iter()
        .filter(|t| matches!(t.kind, LayoutTokenKind::Indent))
        .count();
    let dedents = tokens
        .iter()
        .filter(|t| matches!(t.kind, LayoutTokenKind::Dedent))
        .count();

    assert_eq!(indents, 1, "only model X creates indentation level");
    assert_eq!(dedents, 1);
}

#[test]
fn newline_precedes_indent_for_grammar_compliance() {
    let source = "warrant bitset\n  model BitSet\n";
    let doc = admit(source);
    let tokens = capture_layout(&doc, LayoutLimits::default()).expect("layout captured");

    let indent_pos = tokens
        .iter()
        .position(|t| matches!(t.kind, LayoutTokenKind::Indent))
        .expect("must contain indent");
    assert!(indent_pos > 0);
    assert_eq!(
        tokens[indent_pos - 1].kind,
        LayoutTokenKind::Newline,
        "NEWLINE must precede INDENT for grammar compliance"
    );
}

#[test]
fn multibyte_utf8_source_never_panics() {
    let source = "claim test:\n  ∀ x : Nat . λ y -> x + y\n";
    let doc = admit(source);
    let tokens = capture_layout(&doc, LayoutLimits::default()).expect("multibyte layout captured");
    assert!(!tokens.is_empty());
}

#[test]
fn invalid_limits_rejects_zero_or_excessive_bounds() {
    let doc = admit("warrant test\n");
    let limits = LayoutLimits {
        max_scan_work: 0,
        ..Default::default()
    };
    let err = capture_layout(&doc, limits).unwrap_err();
    assert_eq!(err.kind(), LayoutErrorKind::InvalidLimits);
}

#[test]
fn error_diagnostics_never_echo_source_bytes() {
    let secret = "SECRET_TOKEN_TEXT";
    let source = format!("assume:\n\t{secret}\n");
    let doc = admit(&source);
    let err = capture_layout(&doc, LayoutLimits::default()).unwrap_err();

    let display = format!("{err}");
    let debug = format!("{err:?}");

    assert!(
        !display.contains(secret),
        "Display must not leak source bytes"
    );
    assert!(!debug.contains(secret), "Debug must not leak source bytes");
}
