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

    common::assert_suite_metadata(
        &suite.format,
        &suite.description,
        suite.cases.iter().map(|case| case.id.as_str()),
        "whatwg-html-tokenizer-attribute-edges/v1",
        25,
        &[
            "duplicate-attributes-drop-later-name",
            "unexpected-solidus-before-attribute",
            "end-tag-with-attributes-and-trailing-solidus",
        ],
    );
}

#[test]
fn whatwg_attribute_edge_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        common::assert_case_description(&case.id, &case.description, "attribute edge");
        common::assert_default_lexer_case(&case.id, &case.input, &case.tokens, &case.diagnostics);
    }
}

fn load_suite() -> AttributeEdgeSuite {
    serde_json::from_str(WHATWG_ATTRIBUTE_EDGES)
        .expect("WHATWG attribute-edge fixture should parse")
}
