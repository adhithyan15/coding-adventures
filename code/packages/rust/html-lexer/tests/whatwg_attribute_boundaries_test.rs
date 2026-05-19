use coding_adventures_html_lexer::{
    create_html_lexer_with_context, Attribute, HtmlLexContext, HtmlTokenizerState, StartTagSeed,
    Token,
};
use serde::Deserialize;

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
            .map(token_summary)
            .collect::<Vec<_>>();
        assert_eq!(
            coalesce_adjacent_text_summaries(actual_tokens),
            coalesce_adjacent_text_summaries(case.tokens.clone()),
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

fn token_summary(token: Token) -> String {
    match token {
        Token::Text(data) => format!("Text(data={data})"),
        Token::StartTag {
            name,
            attributes,
            self_closing,
        } => format!(
            "StartTag(name={name}, attributes={}, self_closing={self_closing})",
            attribute_summary(&attributes)
        ),
        Token::EndTag { name } => format!("EndTag(name={name})"),
        Token::Comment(data) => format!("Comment(data={data})"),
        Token::Doctype {
            name,
            public_identifier,
            system_identifier,
            force_quirks,
        } => doctype_summary(name, public_identifier, system_identifier, force_quirks),
        Token::Eof => "EOF".to_string(),
    }
}

fn doctype_summary(
    name: Option<String>,
    public_identifier: Option<String>,
    system_identifier: Option<String>,
    force_quirks: bool,
) -> String {
    let name = name.unwrap_or_else(|| "null".to_string());
    match (public_identifier, system_identifier) {
        (None, None) => format!("Doctype(name={name}, force_quirks={force_quirks})"),
        (public_identifier, system_identifier) => format!(
            "Doctype(name={name}, public_identifier={}, system_identifier={}, force_quirks={force_quirks})",
            public_identifier.unwrap_or_else(|| "null".to_string()),
            system_identifier.unwrap_or_else(|| "null".to_string())
        ),
    }
}

fn attribute_summary(attributes: &[Attribute]) -> String {
    if attributes.is_empty() {
        "[]".to_string()
    } else {
        let joined = attributes
            .iter()
            .map(|attribute| format!("{}={}", attribute.name, attribute.value))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{joined}]")
    }
}

fn coalesce_adjacent_text_summaries(tokens: Vec<String>) -> Vec<String> {
    let mut coalesced: Vec<String> = Vec::new();
    for token in tokens {
        let Some(text) = token
            .strip_prefix("Text(data=")
            .and_then(|text| text.strip_suffix(')'))
        else {
            coalesced.push(token);
            continue;
        };
        if let Some(previous) = coalesced.last_mut() {
            if previous.starts_with("Text(data=") && previous.ends_with(')') {
                previous.pop();
                previous.push_str(text);
                previous.push(')');
                continue;
            }
        }
        coalesced.push(token);
    }
    coalesced
}
