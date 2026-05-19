use coding_adventures_html_lexer::{
    create_html_lexer_with_context, DoctypeSeed, HtmlLexContext, HtmlLexer, HtmlTokenizerState,
};
use serde::Deserialize;

mod common;

const WHATWG_EOF_RECOVERY: &str = include_str!("fixtures/whatwg-eof-recovery.json");

#[derive(Debug, Deserialize)]
struct EofRecoverySuite {
    format: String,
    description: String,
    cases: Vec<EofRecoveryCase>,
}

#[derive(Debug, Deserialize)]
struct EofRecoveryCase {
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
fn whatwg_eof_recovery_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-tokenizer-eof-recovery/v1");
    assert!(!suite.description.is_empty());
    assert!(suite.cases.len() >= 40);
    assert!(suite.cases.iter().any(|case| case.id == "doctype-name"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "seeded-character-reference-rcdata"));
}

#[test]
fn whatwg_eof_recovery_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        assert!(
            !case.description.is_empty(),
            "case `{}` should describe its EOF edge",
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

fn load_suite() -> EofRecoverySuite {
    serde_json::from_str(WHATWG_EOF_RECOVERY).expect("WHATWG EOF recovery fixture should parse")
}

fn configured_lexer(case: &EofRecoveryCase) -> HtmlLexer {
    let state = case
        .initial_state
        .as_deref()
        .and_then(HtmlTokenizerState::from_html5lib_state)
        .unwrap_or(HtmlTokenizerState::Data);
    let mut context = if let Some(current_doctype) = case.current_doctype.as_ref() {
        HtmlLexContext::doctype_continuation(state, doctype_seed(current_doctype))
            .unwrap_or_else(|| panic!("case `{}` cannot seed DOCTYPE state", case.id))
    } else if let Some(current_comment) = case.current_comment.as_deref() {
        HtmlLexContext::comment_continuation(state, current_comment)
            .unwrap_or_else(|| panic!("case `{}` cannot seed comment state", case.id))
    } else if let Some(current_end_tag) = case.current_end_tag.as_deref() {
        HtmlLexContext::end_tag_continuation(
            state,
            case.last_start_tag.as_deref().unwrap_or(""),
            current_end_tag,
            case.temporary_buffer.as_deref().unwrap_or(""),
        )
        .unwrap_or_else(|| panic!("case `{}` cannot seed end-tag state", case.id))
    } else if let Some(return_state) = case
        .return_state
        .as_deref()
        .and_then(HtmlTokenizerState::from_html5lib_state)
    {
        HtmlLexContext::character_reference_continuation(
            state,
            return_state,
            case.temporary_buffer.as_deref().unwrap_or(""),
        )
        .unwrap_or_else(|| panic!("case `{}` cannot seed character reference state", case.id))
    } else if state.is_script_substate() {
        HtmlLexContext::script_substate(state)
            .unwrap_or_else(|| panic!("case `{}` cannot seed script substate", case.id))
    } else {
        HtmlLexContext::new(state)
    };
    if let Some(last_start_tag) = case.last_start_tag.as_deref() {
        if context.last_start_tag.is_none() {
            context = context.with_last_start_tag(last_start_tag);
        }
    }
    if let Some(temporary_buffer) = case.temporary_buffer.as_deref() {
        if context.temporary_buffer.is_none() {
            context = context.with_temporary_buffer(temporary_buffer);
        }
    }

    create_html_lexer_with_context(&context).expect("HTML lexer context should apply")
}

fn doctype_seed(seed: &FixtureDoctypeSeed) -> DoctypeSeed {
    DoctypeSeed {
        name: seed.name.clone(),
        public_identifier: seed.public_identifier.clone(),
        system_identifier: seed.system_identifier.clone(),
        force_quirks: seed.force_quirks,
    }
}
