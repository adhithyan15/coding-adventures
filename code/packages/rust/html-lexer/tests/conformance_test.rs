use coding_adventures_html_lexer::{
    create_html_lexer, html1_machine, html_skeleton_machine, Attribute, HtmlLexer, Token,
};
use serde::Deserialize;
use serde_json::Value;

const HTML_SKELETON_FIXTURES: &str = include_str!("fixtures/html-skeleton.json");
const HTML1_FIXTURES: &str = include_str!("fixtures/html1.json");
const HTML5LIB_SMOKE_FIXTURES: &str = include_str!("fixtures/upstream-html5lib-smoke.test");

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

#[derive(Debug, Deserialize)]
struct Html5libTokenizerFile {
    tests: Vec<Html5libTokenizerTest>,
}

#[derive(Debug, Deserialize)]
struct Html5libTokenizerTest {
    description: String,
    input: String,
    output: Vec<Value>,
    #[serde(default, rename = "initialStates")]
    initial_states: Vec<String>,
    #[serde(default, rename = "lastStartTag")]
    last_start_tag: Option<String>,
    #[serde(default)]
    errors: Vec<Html5libTokenizerError>,
}

#[derive(Debug, Deserialize)]
struct Html5libTokenizerError {
    code: String,
    line: usize,
    col: usize,
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
fn html5lib_smoke_fixture_file_parses() {
    let file = load_html5lib_file(HTML5LIB_SMOKE_FIXTURES);

    assert_eq!(file.tests.len(), 4);
    assert_eq!(
        file.tests[0].description,
        "simple start and end tag in data state"
    );
    assert!(file.tests[0].initial_states.is_empty());
    assert!(file.tests[0].last_start_tag.is_none());
    assert_eq!(file.tests[3].errors[0].code, "eof-in-comment");
    assert_eq!(file.tests[3].errors[0].line, 1);
    assert_eq!(file.tests[3].errors[0].col, 9);
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

#[test]
fn normalized_html5lib_cases_match_default_wrapper() {
    let file = load_html5lib_file(HTML5LIB_SMOKE_FIXTURES);
    let suite = normalize_html5lib_suite(&file);

    assert_eq!(suite.format, "venture-html-lexer-fixtures/v1");
    assert_eq!(suite.suite, "html5lib-smoke");
    assert_eq!(suite.cases.len(), 4);

    run_fixture_suite(&suite, || {
        create_html_lexer().map_err(|error| format!("{error:?}"))
    });
}

#[test]
fn normalized_html5lib_cases_match_generated_html1_machine() {
    let file = load_html5lib_file(HTML5LIB_SMOKE_FIXTURES);
    let suite = normalize_html5lib_suite(&file);

    run_fixture_suite(&suite, || {
        html1_machine()
            .map(HtmlLexer::new)
            .map_err(|error| error.to_string())
    });
}

fn load_suite(raw: &str) -> FixtureSuite {
    serde_json::from_str(raw).expect("fixture suite should parse")
}

fn load_html5lib_file(raw: &str) -> Html5libTokenizerFile {
    serde_json::from_str(raw).expect("html5lib tokenizer fixture file should parse")
}

fn normalize_html5lib_suite(file: &Html5libTokenizerFile) -> FixtureSuite {
    FixtureSuite {
        format: "venture-html-lexer-fixtures/v1".to_string(),
        suite: "html5lib-smoke".to_string(),
        description:
            "Normalized html5lib-style tokenizer smoke cases lowered into the Venture fixture schema"
                .to_string(),
        cases: file
            .tests
            .iter()
            .enumerate()
            .map(|(index, test)| normalize_html5lib_case(index, test))
            .collect(),
    }
}

fn normalize_html5lib_case(index: usize, test: &Html5libTokenizerTest) -> FixtureCase {
    assert!(
        test.initial_states.is_empty() || test.initial_states == ["Data state".to_string()],
        "html5lib smoke normalizer currently supports only the data state"
    );
    assert!(
        test.last_start_tag.is_none(),
        "html5lib smoke normalizer does not yet support lastStartTag"
    );

    let mut tokens = Vec::new();
    let mut pending_text = String::new();

    for token in &test.output {
        let items = token
            .as_array()
            .expect("html5lib tokenizer output tokens should be arrays");
        let kind = items
            .first()
            .and_then(Value::as_str)
            .expect("html5lib tokenizer token kind should be a string");

        match kind {
            "Character" => {
                pending_text.push_str(
                    items
                        .get(1)
                        .and_then(Value::as_str)
                        .expect("Character token should carry data"),
                );
            }
            "StartTag" => {
                flush_pending_text(&mut pending_text, &mut tokens);
                tokens.push(normalize_html5lib_start_tag(items));
            }
            "EndTag" => {
                flush_pending_text(&mut pending_text, &mut tokens);
                tokens.push(format!(
                    "EndTag(name={})",
                    items
                        .get(1)
                        .and_then(Value::as_str)
                        .expect("EndTag token should carry a name")
                ));
            }
            "Comment" => {
                flush_pending_text(&mut pending_text, &mut tokens);
                tokens.push(format!(
                    "Comment(data={})",
                    items
                        .get(1)
                        .and_then(Value::as_str)
                        .expect("Comment token should carry data")
                ));
            }
            "DOCTYPE" => {
                flush_pending_text(&mut pending_text, &mut tokens);
                tokens.push(normalize_html5lib_doctype(items));
            }
            other => panic!("unsupported html5lib smoke token kind `{other}`"),
        }
    }

    flush_pending_text(&mut pending_text, &mut tokens);
    tokens.push("EOF".to_string());

    FixtureCase {
        id: format!("html5lib-smoke-{}", index + 1),
        input: test.input.clone(),
        tokens,
        diagnostics: test.errors.iter().map(|error| error.code.clone()).collect(),
    }
}

fn flush_pending_text(pending_text: &mut String, tokens: &mut Vec<String>) {
    if pending_text.is_empty() {
        return;
    }

    tokens.push(format!("Text(data={pending_text})"));
    pending_text.clear();
}

fn normalize_html5lib_start_tag(items: &[Value]) -> String {
    let name = items
        .get(1)
        .and_then(Value::as_str)
        .expect("StartTag token should carry a name");
    let attributes = items
        .get(2)
        .and_then(Value::as_object)
        .expect("StartTag token should carry an attribute map");
    let attribute_summary = if attributes.is_empty() {
        "[]".to_string()
    } else {
        let joined = attributes
            .iter()
            .map(|(name, value)| {
                format!(
                    "{}={}",
                    name,
                    value
                        .as_str()
                        .expect("attribute values should be strings in smoke fixtures")
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{joined}]")
    };
    let self_closing = items.get(3).and_then(Value::as_bool).unwrap_or(false);

    format!("StartTag(name={name}, attributes={attribute_summary}, self_closing={self_closing})")
}

fn normalize_html5lib_doctype(items: &[Value]) -> String {
    let name = items
        .get(1)
        .and_then(Value::as_str)
        .expect("DOCTYPE token should carry a name");
    let correctness = items
        .get(4)
        .and_then(Value::as_bool)
        .expect("DOCTYPE token should carry correctness");
    let force_quirks = !correctness;

    format!("Doctype(name={name}, force_quirks={force_quirks})")
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
