use coding_adventures_html_lexer::{
    create_html_lexer_with_context, HtmlLexContext, HtmlLexer, HtmlTokenizerState,
};
use serde::Deserialize;

mod common;

const WHATWG_COMMENT_BOUNDARIES: &str = include_str!("fixtures/whatwg-comment-boundaries.json");

#[derive(Debug, Deserialize)]
struct CommentBoundarySuite {
    format: String,
    description: String,
    cases: Vec<CommentBoundaryCase>,
}

#[derive(Debug, Deserialize)]
struct CommentBoundaryCase {
    id: String,
    description: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
    #[serde(default)]
    initial_state: Option<String>,
    #[serde(default)]
    current_comment: Option<String>,
}

#[test]
fn whatwg_comment_boundaries_fixture_parses() {
    let suite = load_suite();

    common::assert_suite_metadata(
        &suite.format,
        &suite.description,
        suite.cases.iter().map(|case| case.id.as_str()),
        "whatwg-html-tokenizer-comment-boundaries/v1",
        45,
        &[
            "comment-nested-looking-opener",
            "comment-eof-end-bang",
            "seeded-bogus-comment-eof",
        ],
    );
}

#[test]
fn whatwg_comment_boundaries_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        common::assert_case_description(&case.id, &case.description, "comment boundary");
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

fn load_suite() -> CommentBoundarySuite {
    serde_json::from_str(WHATWG_COMMENT_BOUNDARIES)
        .expect("WHATWG comment boundary fixture should parse")
}

fn configured_lexer(case: &CommentBoundaryCase) -> HtmlLexer {
    let state = case
        .initial_state
        .as_deref()
        .and_then(HtmlTokenizerState::from_html5lib_state)
        .unwrap_or(HtmlTokenizerState::Data);
    let context = if let Some(current_comment) = case.current_comment.as_deref() {
        HtmlLexContext::comment_continuation(state, current_comment)
            .unwrap_or_else(|| panic!("case `{}` cannot seed comment state", case.id))
    } else {
        HtmlLexContext::new(state)
    };

    create_html_lexer_with_context(&context).expect("HTML lexer context should apply")
}
