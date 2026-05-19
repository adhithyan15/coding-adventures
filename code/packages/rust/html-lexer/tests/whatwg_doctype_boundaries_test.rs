use coding_adventures_html_lexer::{
    create_html_lexer_with_context, DoctypeSeed, HtmlLexContext, HtmlLexer, HtmlTokenizerState,
};
use serde::Deserialize;

mod common;

const WHATWG_DOCTYPE_BOUNDARIES: &str = include_str!("fixtures/whatwg-doctype-boundaries.json");

#[derive(Debug, Deserialize)]
struct DoctypeBoundarySuite {
    format: String,
    description: String,
    cases: Vec<DoctypeBoundaryCase>,
}

#[derive(Debug, Deserialize)]
struct DoctypeBoundaryCase {
    id: String,
    description: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
    #[serde(default)]
    initial_state: Option<String>,
    #[serde(default)]
    current_doctype: Option<FixtureDoctypeSeed>,
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
fn whatwg_doctype_boundaries_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-tokenizer-doctype-boundaries/v1");
    assert!(!suite.description.is_empty());
    assert!(suite.cases.len() >= 40);
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "doctype-public-system-missing-whitespace"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "doctype-bogus-null-discard"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "seeded-after-system-keyword-missing-whitespace"));
}

#[test]
fn whatwg_doctype_boundaries_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        assert!(
            !case.description.is_empty(),
            "case `{}` should describe its DOCTYPE boundary",
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

fn load_suite() -> DoctypeBoundarySuite {
    serde_json::from_str(WHATWG_DOCTYPE_BOUNDARIES)
        .expect("WHATWG DOCTYPE boundary fixture should parse")
}

fn configured_lexer(case: &DoctypeBoundaryCase) -> HtmlLexer {
    let state = case
        .initial_state
        .as_deref()
        .and_then(HtmlTokenizerState::from_html5lib_state)
        .unwrap_or(HtmlTokenizerState::Data);
    let context = if let Some(current_doctype) = case.current_doctype.as_ref() {
        HtmlLexContext::doctype_continuation(
            state,
            DoctypeSeed {
                name: current_doctype.name.clone(),
                public_identifier: current_doctype.public_identifier.clone(),
                system_identifier: current_doctype.system_identifier.clone(),
                force_quirks: current_doctype.force_quirks,
            },
        )
        .unwrap_or_else(|| panic!("case `{}` cannot seed DOCTYPE state", case.id))
    } else {
        HtmlLexContext::new(state)
    };

    create_html_lexer_with_context(&context).expect("HTML lexer context should apply")
}
