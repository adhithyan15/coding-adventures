use coding_adventures_html_lexer::{create_html_lexer, Attribute, Token};
use serde::Deserialize;

const WHATWG_ATTRIBUTE_EDGES: &str = include_str!("fixtures/whatwg-attribute-edges.json");

#[derive(Debug, Deserialize)]
struct AttributeEdgeSuite {
    format: String,
    description: String,
    cases: Vec<AttributeEdgeCase>,
}

#[derive(Debug, Deserialize)]
struct AttributeEdgeCase {
    id: String,
    description: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
}

#[test]
fn whatwg_attribute_edge_fixture_parses() {
    let suite = load_suite();

    assert_eq!(suite.format, "whatwg-html-tokenizer-attribute-edges/v1");
    assert!(!suite.description.is_empty());
    assert!(suite.cases.len() >= 25);
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "duplicate-attributes-drop-later-name"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "unexpected-solidus-before-attribute"));
    assert!(suite
        .cases
        .iter()
        .any(|case| case.id == "end-tag-with-attributes-and-trailing-solidus"));
}

#[test]
fn whatwg_attribute_edge_cases_match_default_lexer() {
    let suite = load_suite();

    for case in &suite.cases {
        assert!(
            !case.description.is_empty(),
            "case `{}` should describe its attribute edge",
            case.id
        );
        let mut lexer = create_html_lexer().expect("HTML lexer should build");
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

fn load_suite() -> AttributeEdgeSuite {
    serde_json::from_str(WHATWG_ATTRIBUTE_EDGES)
        .expect("WHATWG attribute-edge fixture should parse")
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
