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
        common::assert_case_description(&case.id, &case.description, "attribute boundary");
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
        common::assert_lexer_case(
            &case.id,
            &mut lexer,
            &case.input,
            &case.tokens,
            &case.diagnostics,
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
