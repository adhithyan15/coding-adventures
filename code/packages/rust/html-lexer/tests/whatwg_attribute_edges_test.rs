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

        common::assert_token_summaries(&case.id, lexer.drain_tokens(), &case.tokens);
        common::assert_diagnostic_codes(&case.id, lexer.diagnostics(), &case.diagnostics);
    }
}

fn load_suite() -> AttributeEdgeSuite {
    serde_json::from_str(WHATWG_ATTRIBUTE_EDGES)
        .expect("WHATWG attribute-edge fixture should parse")
}
