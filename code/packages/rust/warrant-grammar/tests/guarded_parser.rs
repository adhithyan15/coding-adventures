//! Bounded guarded Warrant parser tests (VER04c / P006b3).
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, ParserErrorKind, ParserLimits,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

#[test]
fn parses_warrant_declarations_successfully() {
    let source = r#"warrant bitset
  model BitSet = Set
  error DecodeError = TooLarge | InvalidFormat
  op empty() -> BitSet
  assume platform:
    u64.wraps
  example insert_example:
    member(empty, 3) == true
  claim round_trip:
    forall s: BitSet . decode(encode(s)) == Ok(s)
"#;
    let doc = SourceDocument::admit(source.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let file =
        parse_document(&tokens, &doc, ParserLimits::default()).expect("should parse declarations");
    assert_eq!(file.name, "bitset");
    assert_eq!(file.declarations.len(), 6);

    assert!(matches!(&file.declarations[0], Declaration::Model(m) if m.name == "BitSet"));
    assert!(
        matches!(&file.declarations[1], Declaration::Error(e) if e.name == "DecodeError" && e.variants == ["TooLarge", "InvalidFormat"])
    );
    assert!(matches!(&file.declarations[2], Declaration::Op(o) if o.name == "empty"));
    assert!(matches!(&file.declarations[3], Declaration::Assumptions(_)));
    assert!(matches!(&file.declarations[4], Declaration::Example(e) if e.name == "insert_example"));
    assert!(matches!(&file.declarations[5], Declaration::Claim(c) if c.name == "round_trip"));
}

#[test]
fn recursion_depth_guard_strictly_enforced() {
    // Deeply nested parentheses: ((((1 + 1))))
    let source = "warrant test\n  claim deep:\n    ((((1 + 1)))) == 2\n";
    let limits = ParserLimits {
        max_depth: 3,
        max_nodes: 10_000,
        max_parse_work: 100_000,
    };
    let doc = SourceDocument::admit(source.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let err = parse_document(&tokens, &doc, limits).unwrap_err();
    assert_eq!(err.code(), "W0001");
    assert_eq!(err.kind(), ParserErrorKind::RecursionDepth);
}

#[test]
fn node_budget_strictly_enforced() {
    let source = "warrant test\n  claim long:\n    1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 == 45\n";
    let limits = ParserLimits {
        max_depth: 64,
        max_nodes: 5,
        max_parse_work: 100_000,
    };
    let doc = SourceDocument::admit(source.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let err = parse_document(&tokens, &doc, limits).unwrap_err();
    assert_eq!(err.code(), "W0001");
    assert_eq!(err.kind(), ParserErrorKind::NodeBudget);
}

#[test]
fn invalid_limits_rejected() {
    let doc = SourceDocument::admit(b"warrant test\n", SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let limits = ParserLimits {
        max_depth: 0,
        ..Default::default()
    };
    let err = parse_document(&tokens, &doc, limits).unwrap_err();
    assert_eq!(err.kind(), ParserErrorKind::InvalidLimits);
}

#[test]
fn error_diagnostics_never_echo_source() {
    let secret = "VERY_CONFIDENTIAL_TOKEN";
    let source = format!("warrant test\n  claim bad:\n    {secret} &&&&\n");
    let doc = SourceDocument::admit(source.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let err = parse_document(&tokens, &doc, ParserLimits::default()).unwrap_err();

    let display = format!("{err}");
    let debug = format!("{err:?}");
    assert!(!display.contains(secret));
    assert!(!debug.contains(secret));
}

#[test]
fn unterminated_byte_literal_fails_closed_without_panic() {
    let source = "warrant test\n  claim bad:\n    b\"\n";
    let doc = SourceDocument::admit(source.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let err = parse_document(&tokens, &doc, ParserLimits::default()).unwrap_err();
    assert_eq!(err.code(), "W0001");
    assert_eq!(err.kind(), ParserErrorKind::UnexpectedToken);
}

#[test]
fn multibyte_escapes_never_panic() {
    let source = "warrant test\n  claim bad:\n    \"\\世界\"\n";
    let doc = SourceDocument::admit(source.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let err = parse_document(&tokens, &doc, ParserLimits::default()).unwrap_err();
    assert_eq!(err.code(), "W0001");
}

#[test]
fn ast_spans_preserved_faithfully() {
    let source = "warrant test\n  assume platform:\n    u64.wraps\n  claim c:\n    match x with None -> 0 | Some(y) -> y\n";
    let doc = SourceDocument::admit(source.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let file = parse_document(&tokens, &doc, ParserLimits::default()).unwrap();

    // Check platform assumptions span encompasses the entire body
    match &file.declarations[0] {
        Declaration::Assumptions(
            coding_adventures_warrant_grammar::parser::AssumptionsDecl::Platform { rows, span },
        ) => {
            assert!(span.0 < span.1);
            assert_eq!(span.1, rows[0].span.1);
            assert_eq!(rows[0].segments, ["u64", "wraps"]);
        }
        _ => panic!("expected assumptions"),
    }

    // Check match arm span start matches pattern start (not hardcoded 0)
    match &file.declarations[1] {
        Declaration::Claim(c) => match &c.body {
            coding_adventures_warrant_grammar::parser::Expression::Match { arms, .. } => {
                assert!(arms[0].span.0 > 0);
                assert!(arms[1].span.0 > arms[0].span.1);
            }
            _ => panic!("expected match expression"),
        },
        _ => panic!("expected claim"),
    }
}

#[test]
fn binary_expression_allocations_counted_against_node_budget() {
    let source = "warrant test\n  claim add:\n    1 + 2 + 3 + 4 + 5\n";
    let limits = ParserLimits {
        max_depth: 64,
        max_nodes: 3,
        max_parse_work: 100_000,
    };
    let doc = SourceDocument::admit(source.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let err = parse_document(&tokens, &doc, limits).unwrap_err();
    assert_eq!(err.code(), "W0001");
    assert_eq!(err.kind(), ParserErrorKind::NodeBudget);
}

#[test]
fn quantifier_with_space_before_dot_preserves_refinement_identifier() {
    let source = "warrant test\n  claim q:\n    forall x: Nat | x > 0 . x == x\n";
    let doc = SourceDocument::admit(source.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let file = parse_document(&tokens, &doc, ParserLimits::default()).unwrap();
    assert_eq!(file.declarations.len(), 1);
}

#[test]
fn crlf_input_with_qualified_names_parses_successfully() {
    let source = "warrant test\r\n  assume platform:\r\n    u64.wraps\r\n  example e:\r\n    Err(E.Bad) == Err(E.Bad)\r\n";
    let doc = SourceDocument::admit(source.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let file = parse_document(&tokens, &doc, ParserLimits::default()).unwrap();
    assert_eq!(file.declarations.len(), 2);
}

#[test]
fn all_57_canonical_cases_validated() {
    use coding_adventures_test_bootstrap::tfv::{self, Value};
    const CORPUS: &str = include_str!("../../../../specs/fixtures/warrant-syntax-v1/cases.tfv");

    let records = tfv::parse_file(CORPUS).expect("valid tfv corpus");
    assert_eq!(records.len(), 57);

    for record in &records {
        let source = record.field("source").and_then(Value::as_text).unwrap();
        let expected_accept = match record.field("accept") {
            Some(Value::Bool(b)) => *b,
            _ => panic!("missing accept verdict in case {}", record.id),
        };

        let doc = match SourceDocument::admit(source.as_bytes(), SourceLimits::default()) {
            Ok(d) => d,
            Err(_) => {
                assert!(
                    !expected_accept,
                    "case {} failed admission unexpectedly",
                    record.id
                );
                continue;
            }
        };

        let tokens = match capture_layout(&doc, LayoutLimits::default()) {
            Ok(t) => t,
            Err(_) => {
                assert!(
                    !expected_accept,
                    "case {} failed layout capture unexpectedly",
                    record.id
                );
                continue;
            }
        };

        let result = parse_document(&tokens, &doc, ParserLimits::default());

        if expected_accept {
            assert!(
                result.is_ok(),
                "case {} expected ACCEPT, but failed: {:?}",
                record.id,
                result.err()
            );
        } else {
            assert!(
                result.is_err(),
                "case {} expected REJECT, but parsed successfully",
                record.id
            );
        }
    }
}
