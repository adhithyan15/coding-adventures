use coding_adventures_html_lexer::{
    create_html_lexer_with_context, HtmlLexContext, HtmlLexer, HtmlTokenizerState,
};
use serde::Deserialize;

mod common;

const WHATWG_CDATA_BOUNDARIES: &str = include_str!("fixtures/whatwg-cdata-boundaries.json");

#[derive(Debug, Deserialize)]
struct CdataBoundarySuite {
    format: String,
    description: String,
    cases: Vec<CdataBoundaryCase>,
}

#[derive(Debug, Deserialize)]
struct CdataBoundaryCase {
    id: String,
    description: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
    #[serde(default)]
    initial_state: Option<String>,
}

#[test]
fn whatwg_cdata_boundaries_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-tokenizer-cdata-boundaries/v1");
    assert!(!suite.description.is_empty());
    assert!(suite.cases.len() >= 25);
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "cdata-markup-stays-text"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "html-cdata-looking-declaration"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "seeded-cdata-end-eof"));
}

#[test]
fn whatwg_cdata_boundaries_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        common::assert_case_description(&case.id, &case.description, "CDATA boundary");
        let mut lexer = configured_lexer(case);
        common::assert_lexer_case(
            &case.id,
            &mut lexer,
            &case.input,
            &case.tokens,
            &case.diagnostics,
        );
    }
}

fn load_suite() -> CdataBoundarySuite {
    serde_json::from_str(WHATWG_CDATA_BOUNDARIES)
        .expect("WHATWG CDATA boundary fixture should parse")
}

fn configured_lexer(case: &CdataBoundaryCase) -> HtmlLexer {
    let state = case
        .initial_state
        .as_deref()
        .and_then(HtmlTokenizerState::from_html5lib_state)
        .unwrap_or(HtmlTokenizerState::Data);
    let context = if state == HtmlTokenizerState::CdataSection {
        HtmlLexContext::cdata_section()
    } else {
        HtmlLexContext::new(state)
    };

    create_html_lexer_with_context(&context).expect("HTML lexer context should apply")
}
