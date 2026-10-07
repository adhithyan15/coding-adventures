//! Real generated ECMAScript grammars exercise mode and template cursor paths.
use coding_adventures_javascript_lexer::{tokenize_javascript_typed, tokenize_javascript_typed_with_ranges};
use coding_adventures_javascript_tokens::EsVersion;
use lexer::lexical::{validate_lexical_partition, LexicalLimits, LexicalPieceKind};

#[test]
fn all_ecmascript_editions_preserve_tokens_and_exact_literal_spelling() {
    let src = "/*😀*/var x=\"A\\n😀\";\r\n";
    for &version in EsVersion::ALL {
        let snap = tokenize_javascript_typed_with_ranges(src, version, LexicalLimits::default()).unwrap();
        assert_eq!(snap.tokens(), tokenize_javascript_typed(src, version).unwrap(), "{version}");
        validate_lexical_partition(src, snap.tokens().len(), snap.pieces(), LexicalLimits::default()).unwrap();
        let string = snap.pieces().iter().find(|p| matches!(p.kind, LexicalPieceKind::Token(i) if snap.tokens()[i].value == "A\\n😀")).unwrap();
        assert_eq!(&src[string.range.start.byte_offset..string.range.end.byte_offset], "\"A\\n😀\"");
        assert_eq!(string.range.end.byte_offset-string.range.start.byte_offset, 9);
        assert_eq!(string.range.end.utf16_offset-string.range.start.utf16_offset, 7);
        assert_eq!(string.range.end.scalar_offset-string.range.start.scalar_offset, 6);
    }
}

#[test]
fn multiline_and_nested_templates_regex_and_division_keep_actual_modes() {
    for src in [
        "let x=`😀\r\nλ`;report(x);",
        "report(`a${obj.name}b`);",
        "report(`a${{x:1}.x}b${`c${value}d`}e`);",
        "var re=/a[\\/]b/g;var q=one/two/three;",
        "report(true? /x/ : /y/);",
        "function f(){return\r\n/*😀\u{2028}*/1}",
        "a\u{2028}b\u{2029}c\rd\ne\r\nf;",
        "//😀\r\n/* λ\n𐐀 */",
    ] {
        let snap = tokenize_javascript_typed_with_ranges(src, EsVersion::Es2025, LexicalLimits::default()).unwrap();
        assert_eq!(snap.tokens(), tokenize_javascript_typed(src, EsVersion::Es2025).unwrap(), "{src}");
        validate_lexical_partition(src, snap.tokens().len(), snap.pieces(), LexicalLimits::default()).unwrap();
        let rejoined = snap.pieces().iter().map(|p| &src[p.range.start.byte_offset..p.range.end.byte_offset]).collect::<String>();
        assert_eq!(rejoined, src);
    }
}

#[test]
fn malformed_source_returns_no_snapshot_and_resource_errors_are_distinct() {
    let error = tokenize_javascript_typed_with_ranges("var x=@", EsVersion::Es2025, LexicalLimits::default()).unwrap_err();
    assert!(matches!(error, lexer::lexical::LexicalError::Lex(_)));
    let error = tokenize_javascript_typed_with_ranges("var x=1", EsVersion::Es2025, LexicalLimits { max_source_bytes: 2, ..LexicalLimits::default() }).unwrap_err();
    assert!(matches!(error, lexer::lexical::LexicalError::Limit("source bytes")));
}
