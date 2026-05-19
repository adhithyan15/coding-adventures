use coding_adventures_html_lexer::{
    create_html_lexer_with_context, HtmlLexContext, HtmlLexer, HtmlTokenizerState,
};
use serde::Deserialize;

mod common;

const WHATWG_CHARACTER_REFERENCE_BOUNDARIES: &str =
    include_str!("fixtures/whatwg-character-reference-boundaries.json");

#[derive(Debug, Deserialize)]
struct CharacterReferenceBoundarySuite {
    format: String,
    description: String,
    cases: Vec<CharacterReferenceBoundaryCase>,
}

#[derive(Debug, Deserialize)]
struct CharacterReferenceBoundaryCase {
    id: String,
    description: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
    #[serde(default)]
    initial_state: Option<String>,
    #[serde(default)]
    return_state: Option<String>,
    #[serde(default)]
    temporary_buffer: Option<String>,
    #[serde(default)]
    last_start_tag: Option<String>,
}

#[test]
fn whatwg_character_reference_boundaries_fixture_parses() {
    let suite = load_suite();

    common::assert_suite_metadata(
        &suite.format,
        &suite.description,
        suite.cases.iter().map(|case| case.id.as_str()),
        "whatwg-html-tokenizer-character-reference-boundaries/v1",
        40,
        &[
            "data-ambiguous-name-text",
            "attribute-ambiguous-preserved",
            "seeded-decimal-reference-rcdata",
        ],
    );
}

#[test]
fn whatwg_character_reference_boundaries_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        common::assert_case_description(
            &case.id,
            &case.description,
            "character-reference boundary",
        );
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

fn load_suite() -> CharacterReferenceBoundarySuite {
    serde_json::from_str(WHATWG_CHARACTER_REFERENCE_BOUNDARIES)
        .expect("WHATWG character-reference boundary fixture should parse")
}

fn configured_lexer(case: &CharacterReferenceBoundaryCase) -> HtmlLexer {
    let state = case
        .initial_state
        .as_deref()
        .and_then(HtmlTokenizerState::from_html5lib_state)
        .unwrap_or(HtmlTokenizerState::Data);
    let mut context = if let Some(return_state) = case.return_state.as_deref() {
        let return_state = HtmlTokenizerState::from_html5lib_state(return_state)
            .unwrap_or_else(|| panic!("case `{}` has unknown return state", case.id));
        let temporary_buffer = case.temporary_buffer.as_deref().unwrap_or("");
        HtmlLexContext::character_reference_continuation(state, return_state, temporary_buffer)
            .unwrap_or_else(|| panic!("case `{}` cannot seed character reference state", case.id))
    } else {
        HtmlLexContext::new(state)
    };
    if let Some(last_start_tag) = case.last_start_tag.as_deref() {
        context = context.with_last_start_tag(last_start_tag);
    }

    create_html_lexer_with_context(&context).expect("HTML lexer context should apply")
}
