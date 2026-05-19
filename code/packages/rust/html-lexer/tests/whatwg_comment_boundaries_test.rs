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

    assert_eq!(suite.format, "whatwg-html-tokenizer-comment-boundaries/v1");
    assert!(!suite.description.is_empty());
    assert!(suite.cases.len() >= 45);
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "comment-nested-looking-opener"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "comment-eof-end-bang"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "seeded-bogus-comment-eof"));
}

#[test]
fn whatwg_comment_boundaries_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        assert!(
            !case.description.is_empty(),
            "case `{}` should describe its comment boundary",
            case.id
        );
        let mut lexer = configured_lexer(case);
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
