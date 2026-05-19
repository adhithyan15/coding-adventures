use coding_adventures_html_lexer::{
    create_html_lexer_with_context, HtmlLexContext, HtmlLexer, HtmlTokenizerState,
};
use serde::Deserialize;

mod common;

const WHATWG_SCRIPT_ESCAPE_BOUNDARIES: &str =
    include_str!("fixtures/whatwg-script-escape-boundaries.json");

#[derive(Debug, Deserialize)]
struct ScriptEscapeBoundarySuite {
    format: String,
    description: String,
    cases: Vec<ScriptEscapeBoundaryCase>,
}

#[derive(Debug, Deserialize)]
struct ScriptEscapeBoundaryCase {
    id: String,
    description: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
    initial_state: String,
    last_start_tag: String,
    #[serde(default)]
    temporary_buffer: Option<String>,
}

#[test]
fn whatwg_script_escape_boundaries_fixture_parses() {
    let suite = load_suite();

    common::assert_suite_metadata(
        &suite.format,
        &suite.description,
        suite.cases.iter().map(|case| case.id.as_str()),
        "whatwg-html-tokenizer-script-escape-boundaries/v1",
        40,
        &[
            "script-data-comment-eof",
            "script-escaped-less-than-uppercase-script",
            "script-double-escape-end-script",
        ],
    );
}

#[test]
fn whatwg_script_escape_boundaries_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        common::assert_case_description(&case.id, &case.description, "script escape boundary");
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

fn load_suite() -> ScriptEscapeBoundarySuite {
    serde_json::from_str(WHATWG_SCRIPT_ESCAPE_BOUNDARIES)
        .expect("WHATWG script escape boundary fixture should parse")
}

fn configured_lexer(case: &ScriptEscapeBoundaryCase) -> HtmlLexer {
    let state = HtmlTokenizerState::from_html5lib_state(&case.initial_state)
        .unwrap_or_else(|| panic!("case `{}` has unknown initial state", case.id));
    let mut context = HtmlLexContext::script_substate(state)
        .unwrap_or_else(|| panic!("case `{}` cannot seed script substate", case.id));
    if context.last_start_tag.as_deref() != Some(case.last_start_tag.as_str()) {
        context = context.with_last_start_tag(&case.last_start_tag);
    }
    if let Some(temporary_buffer) = case.temporary_buffer.as_deref() {
        context = context.with_temporary_buffer(temporary_buffer);
    }

    create_html_lexer_with_context(&context).expect("HTML lexer context should apply")
}
