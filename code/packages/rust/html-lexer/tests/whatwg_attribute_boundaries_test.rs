use coding_adventures_html_lexer::{
    create_html_lexer_with_context, Attribute, HtmlLexContext, HtmlTokenizerState, StartTagSeed,
};
use serde::Deserialize;

mod common;

const WHATWG_ATTRIBUTE_BOUNDARIES: &str = include_str!("fixtures/whatwg-attribute-boundaries.json");

#[derive(Debug, Deserialize)]
struct AttributeBoundarySuite {
    format: String,
    description: String,
    cases: Vec<AttributeBoundaryCase>,
}

#[derive(Debug, Deserialize)]
struct AttributeBoundaryCase {
    id: String,
    description: String,
    input: String,
    initial_state: String,
    start_tag: FixtureStartTagSeed,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureStartTagSeed {
    name: String,
    #[serde(default)]
    attributes: Vec<FixtureAttribute>,
    #[serde(default)]
    self_closing: bool,
    #[serde(default)]
    current_attribute: Option<FixtureAttribute>,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureAttribute {
    name: String,
    value: String,
}

#[test]
fn whatwg_attribute_boundaries_fixture_parses() {
    let suite = load_suite();

    assert_eq!(
        suite.format,
        "whatwg-html-tokenizer-attribute-boundaries/v1"
    );
    assert!(!suite.description.is_empty());
    assert!(suite.cases.len() >= 18);
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "attribute-name-null-replacement"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "after-attribute-value-quoted-missing-whitespace"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "self-closing-start-tag-reconsumes-attribute"));
}

#[test]
fn whatwg_attribute_boundaries_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        assert!(
            !case.description.is_empty(),
            "case `{}` should describe its attribute boundary",
            case.id
        );
        let initial_state = HtmlTokenizerState::from_machine_state(&case.initial_state)
            .unwrap_or_else(|| panic!("case `{}` unknown initial state", case.id));
        let context =
            HtmlLexContext::start_tag_continuation(initial_state, start_tag_seed(&case.start_tag))
                .unwrap_or_else(|| {
                    panic!(
                        "case `{}` initial state should accept a start-tag seed",
                        case.id
                    )
                });
        let mut lexer =
            create_html_lexer_with_context(&context).expect("HTML lexer should build with context");
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

fn load_suite() -> AttributeBoundarySuite {
    serde_json::from_str(WHATWG_ATTRIBUTE_BOUNDARIES)
        .expect("WHATWG attribute boundary fixture should parse")
}

fn start_tag_seed(seed: &FixtureStartTagSeed) -> StartTagSeed {
    StartTagSeed {
        name: seed.name.clone(),
        attributes: seed.attributes.iter().cloned().map(Into::into).collect(),
        self_closing: seed.self_closing,
        current_attribute: seed.current_attribute.clone().map(Into::into),
    }
}

impl From<FixtureAttribute> for Attribute {
    fn from(attribute: FixtureAttribute) -> Self {
        Self {
            name: attribute.name,
            value: attribute.value,
        }
    }
}
