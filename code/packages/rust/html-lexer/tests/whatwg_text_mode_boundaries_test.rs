use coding_adventures_html_lexer::{
    create_html_lexer_with_context, HtmlLexContext, HtmlLexer, HtmlTokenizerState,
};
use serde::Deserialize;

mod common;

const WHATWG_TEXT_MODE_BOUNDARIES: &str = include_str!("fixtures/whatwg-text-mode-boundaries.json");

#[derive(Debug, Deserialize)]
struct TextModeBoundarySuite {
    format: String,
    description: String,
    cases: Vec<TextModeBoundaryCase>,
}

#[derive(Debug, Deserialize)]
struct TextModeBoundaryCase {
    id: String,
    description: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
    initial_state: String,
    #[serde(default)]
    last_start_tag: Option<String>,
    #[serde(default)]
    current_end_tag: Option<String>,
    #[serde(default)]
    temporary_buffer: Option<String>,
}

#[test]
fn whatwg_text_mode_boundaries_fixture_parses() {
    let suite = load_suite();

    assert_eq!(
        suite.format,
        "whatwg-html-tokenizer-text-mode-boundaries/v1"
    );
    assert!(!suite.description.is_empty());
    assert!(suite.cases.len() >= 25);
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "rcdata-less-than-end-tag"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "rawtext-end-tag-name-self-closing-recovery"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "plaintext-markup-stays-text"));
}

#[test]
fn whatwg_text_mode_boundaries_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        assert!(
            !case.description.is_empty(),
            "case `{}` should describe its text-mode boundary",
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

fn load_suite() -> TextModeBoundarySuite {
    serde_json::from_str(WHATWG_TEXT_MODE_BOUNDARIES)
        .expect("WHATWG text-mode boundary fixture should parse")
}

fn configured_lexer(case: &TextModeBoundaryCase) -> HtmlLexer {
    let state = HtmlTokenizerState::from_html5lib_state(&case.initial_state)
        .unwrap_or_else(|| panic!("case `{}` has unknown initial state", case.id));
    let mut context = HtmlLexContext::new(state);
    if let Some(last_start_tag) = case.last_start_tag.as_deref() {
        context = context.with_last_start_tag(last_start_tag);
    }
    if let Some(current_end_tag) = case.current_end_tag.as_deref() {
        context = context.with_current_end_tag(current_end_tag);
    }
    if let Some(temporary_buffer) = case.temporary_buffer.as_deref() {
        context = context.with_temporary_buffer(temporary_buffer);
    }

    create_html_lexer_with_context(&context).expect("HTML lexer context should apply")
}
