use coding_adventures_html_lexer::{
    create_html_lexer_with_context, DoctypeSeed, HtmlLexContext, HtmlLexer, HtmlTokenizerState,
};
use serde::Deserialize;

mod common;

const WHATWG_TEXT_MODE_DELIMITERS: &str = include_str!("fixtures/whatwg-text-mode-delimiters.json");

#[derive(Debug, Deserialize)]
struct TextModeDelimiterSuite {
    format: String,
    description: String,
    cases: Vec<TextModeDelimiterCase>,
}

#[derive(Debug, Deserialize)]
struct TextModeDelimiterCase {
    id: String,
    description: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
    #[serde(default)]
    initial_state: Option<String>,
    #[serde(default)]
    last_start_tag: Option<String>,
    #[serde(default)]
    current_end_tag: Option<String>,
    #[serde(default)]
    current_comment: Option<String>,
    #[serde(default)]
    current_doctype: Option<FixtureDoctypeSeed>,
    #[serde(default)]
    temporary_buffer: Option<String>,
    #[serde(default)]
    return_state: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureDoctypeSeed {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    public_identifier: Option<String>,
    #[serde(default)]
    system_identifier: Option<String>,
    #[serde(default)]
    force_quirks: bool,
}

#[test]
fn whatwg_text_mode_delimiter_fixture_parses() {
    let suite = load_suite();

    assert_eq!(
        suite.format,
        "whatwg-html-tokenizer-text-mode-delimiters/v1"
    );
    assert!(!suite.description.is_empty());
    assert!(suite.cases.len() >= 50);
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "rcdata-matching-end-tag"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "script-escaped-double-escape-start"));
}

#[test]
fn whatwg_text_mode_delimiter_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        common::assert_case_description(&case.id, &case.description, "delimiter edge");
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

fn load_suite() -> TextModeDelimiterSuite {
    serde_json::from_str(WHATWG_TEXT_MODE_DELIMITERS)
        .expect("WHATWG text-mode delimiter fixture should parse")
}

fn configured_lexer(case: &TextModeDelimiterCase) -> HtmlLexer {
    let state = case
        .initial_state
        .as_deref()
        .and_then(HtmlTokenizerState::from_html5lib_state)
        .unwrap_or(HtmlTokenizerState::Data);
    let mut context = HtmlLexContext::new(state);
    if let Some(last_start_tag) = case.last_start_tag.as_deref() {
        context = context.with_last_start_tag(last_start_tag);
    }
    if let Some(current_end_tag) = case.current_end_tag.as_deref() {
        context = context.with_current_end_tag(current_end_tag);
    }
    if let Some(current_comment) = case.current_comment.as_deref() {
        context = context.with_current_comment(current_comment);
    }
    if let Some(current_doctype) = case.current_doctype.as_ref() {
        context = context.with_current_doctype(DoctypeSeed {
            name: current_doctype.name.clone(),
            public_identifier: current_doctype.public_identifier.clone(),
            system_identifier: current_doctype.system_identifier.clone(),
            force_quirks: current_doctype.force_quirks,
        });
    }
    if let Some(temporary_buffer) = case.temporary_buffer.as_deref() {
        context = context.with_temporary_buffer(temporary_buffer);
    }
    if let Some(return_state) = case
        .return_state
        .as_deref()
        .and_then(HtmlTokenizerState::from_html5lib_state)
    {
        context = context.with_return_state(return_state);
    }

    create_html_lexer_with_context(&context).expect("HTML lexer context should apply")
}
