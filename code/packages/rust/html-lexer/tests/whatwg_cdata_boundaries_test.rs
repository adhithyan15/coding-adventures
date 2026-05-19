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
        assert!(
            !case.description.is_empty(),
            "case `{}` should describe its CDATA boundary",
            case.id
        );
        let mut lexer = configured_lexer(case);
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
