use coding_adventures_html_lexer::{
    create_html_lexer, html1_machine, html_skeleton_machine, Attribute, HtmlLexer, Token,
};
use serde::Deserialize;

const HTML_SKELETON_FIXTURES: &str = include_str!("fixtures/html-skeleton.json");
const HTML1_FIXTURES: &str = include_str!("fixtures/html1.json");

#[derive(Debug, Deserialize)]
struct FixtureSuite {
    format: String,
    suite: String,
    description: String,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    id: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
}

#[test]
fn fixture_manifests_parse() {
    let skeleton = load_suite(HTML_SKELETON_FIXTURES);
    let html1 = load_suite(HTML1_FIXTURES);

    assert_eq!(skeleton.format, "venture-html-lexer-fixtures/v1");
    assert_eq!(skeleton.suite, "html-skeleton");
    assert!(!skeleton.description.is_empty());
    assert_eq!(skeleton.cases.len(), 3);

    assert_eq!(html1.format, "venture-html-lexer-fixtures/v1");
    assert_eq!(html1.suite, "html1");
    assert!(!html1.description.is_empty());
    assert_eq!(html1.cases.len(), 5);
}

#[test]
fn bootstrap_skeleton_conformance_cases_match_generated_machine() {
    let suite = load_suite(HTML_SKELETON_FIXTURES);
    run_fixture_suite(&suite, || {
        html_skeleton_machine()
            .map(HtmlLexer::new)
            .map_err(|error| error.to_string())
    });
}

#[test]
fn html1_conformance_cases_match_generated_machine() {
    let suite = load_suite(HTML1_FIXTURES);
    run_fixture_suite(&suite, || {
        html1_machine()
            .map(HtmlLexer::new)
            .map_err(|error| error.to_string())
    });
}

#[test]
fn html1_conformance_cases_match_default_wrapper() {
    let suite = load_suite(HTML1_FIXTURES);
    run_fixture_suite(&suite, || {
        create_html_lexer().map_err(|error| format!("{error:?}"))
    });
}

fn load_suite(raw: &str) -> FixtureSuite {
    serde_json::from_str(raw).expect("fixture suite should parse")
}

fn run_fixture_suite(suite: &FixtureSuite, create_lexer: impl Fn() -> Result<HtmlLexer, String>) {
    for case in &suite.cases {
        let mut lexer = create_lexer().unwrap_or_else(|error| {
            panic!("suite `{}` failed to construct lexer: {error}", suite.suite)
        });
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
            actual_tokens, case.tokens,
            "suite `{}` case `{}` token mismatch",
            suite.suite, case.id
        );

        let actual_diagnostics = lexer
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            actual_diagnostics, case.diagnostics,
            "suite `{}` case `{}` diagnostic mismatch",
            suite.suite, case.id
        );
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
        Token::Doctype { name, force_quirks } => match name {
            Some(name) => format!("Doctype(name={name}, force_quirks={force_quirks})"),
            None => format!("Doctype(name=null, force_quirks={force_quirks})"),
        },
        Token::Eof => "EOF".to_string(),
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
