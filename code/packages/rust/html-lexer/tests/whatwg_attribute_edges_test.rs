use coding_adventures_html_lexer::create_html_lexer;
use serde::Deserialize;

mod common;

const WHATWG_ATTRIBUTE_EDGES: &str = include_str!("fixtures/whatwg-attribute-edges.json");

#[derive(Debug, Deserialize)]
struct AttributeEdgeSuite {
    format: String,
    description: String,
    cases: Vec<AttributeEdgeCase>,
}

#[derive(Debug, Deserialize)]
struct AttributeEdgeCase {
    id: String,
    description: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
}

#[test]
fn whatwg_attribute_edge_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-tokenizer-attribute-edges/v1");
    assert!(!suite.description.is_empty());
    assert!(suite.cases.len() >= 25);
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "duplicate-attributes-drop-later-name"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "unexpected-solidus-before-attribute"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "end-tag-with-attributes-and-trailing-solidus"));
}

#[test]
fn whatwg_attribute_edge_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        assert!(
            !case.description.is_empty(),
            "case `{}` should describe its attribute edge",
            case.id
        );
        let mut lexer = create_html_lexer().expect("HTML lexer should build");
        lexer
            .push(&case.input)
            .unwrap_or_else(|error| panic!("case `{}` push failed: {error:?}", case.id));
        lexer
            .finish()
            .unwrap_or_else(|error| panic!("case `{}` finish failed: {error:?}", case.id));

        let actual_tokens = lexer
            .drain_tokens()
            .into_iter()
            .map(common::token_summary)
            .collect::<Vec<_>>();
        assert_eq!(
            common::coalesce_adjacent_text_summaries(actual_tokens),
            common::coalesce_adjacent_text_summaries(case.tokens.clone()),
            "case `{}` token mismatch",
            case.id
        );

        let actual_diagnostics = lexer
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            actual_diagnostics, case.diagnostics,
            "case `{}` diagnostic mismatch",
            case.id
        );
    }
}

fn load_suite() -> AttributeEdgeSuite {
    serde_json::from_str(WHATWG_ATTRIBUTE_EDGES)
        .expect("WHATWG attribute-edge fixture should parse")
}
