use coding_adventures_html_lexer::{
    create_html_lexer, html1_machine, html_skeleton_machine, Attribute, HtmlLexer, Token,
};
use serde::Deserialize;
use serde_json::Value;

const HTML_SKELETON_FIXTURES: &str = include_str!("fixtures/html-skeleton.json");
const HTML1_FIXTURES: &str = include_str!("fixtures/html1.json");
const HTML5LIB_RAW_FIXTURES: &str = include_str!("fixtures/upstream-html5lib-smoke.test");
const HTML5LIB_NORMALIZED_FIXTURES: &str = include_str!("fixtures/html5lib-smoke.json");

#[derive(Debug, Deserialize)]
struct FixtureSuite {
    format: String,
    suite: String,
    description: String,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct FixtureCase {
    id: String,
    #[serde(default)]
    description: String,
    input: String,
    tokens: Vec<String>,
    #[serde(default)]
    diagnostics: Vec<String>,
    #[serde(default)]
    initial_state: Option<String>,
    #[serde(default)]
    last_start_tag: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Html5libTokenizerFile {
    tests: Vec<Html5libTokenizerTest>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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

#[derive(Debug, Deserialize)]
struct Html5libNormalizedSuite {
    format: String,
    suite: String,
    description: String,
    source: String,
    generator: String,
    supported_initial_states: Vec<String>,
    cases: Vec<FixtureCase>,
    skipped: Vec<Html5libSkippedCase>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Html5libSkippedCase {
    id: String,
    description: String,
    reason: String,
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
    assert_eq!(html1.cases.len(), 94);
}

#[test]
fn html5lib_smoke_fixture_file_parses() {
    let file = load_html5lib_file(HTML5LIB_RAW_FIXTURES);

    assert_eq!(file.tests.len(), 101);
    assert_eq!(
        file.tests[0].description,
        "simple start and end tag in data state"
    );
    assert!(file.tests[0].initial_states.is_empty());
    assert!(file.tests[0].last_start_tag.is_none());
    assert_eq!(file.tests[2].initial_states, vec!["Data state".to_string()]);
    assert_eq!(file.tests[4].errors[0].code, "missing-doctype-name");
    assert_eq!(file.tests[5].errors[0].code, "eof-in-doctype");
    assert_eq!(file.tests[6].errors[0].code, "eof-in-doctype");
    assert_eq!(file.tests[7].errors[0].code, "invalid-doctype-keyword");
    assert_eq!(
        file.tests[14].errors[0].code,
        "missing-doctype-public-identifier"
    );
    assert_eq!(
        file.tests[19].errors[0].code,
        "missing-doctype-system-identifier"
    );
    assert_eq!(
        file.tests[20].errors[0].code,
        "unexpected-character-after-doctype-system-identifier"
    );
    assert_eq!(file.tests[22].errors[0].code, "eof-in-comment");
    assert_eq!(file.tests[22].errors[0].line, 1);
    assert_eq!(file.tests[22].errors[0].col, 9);
    assert_eq!(
        file.tests[23].errors[0].code,
        "abrupt-closing-of-empty-comment"
    );
    assert_eq!(
        file.tests[25].initial_states,
        vec!["RCDATA state".to_string()]
    );
    assert_eq!(file.tests[25].last_start_tag.as_deref(), Some("title"));
    assert_eq!(
        file.tests[27].initial_states,
        vec!["RAWTEXT state".to_string()]
    );
    assert_eq!(file.tests[27].last_start_tag.as_deref(), Some("style"));
}

#[test]
fn normalized_html5lib_fixture_parses_with_importer_metadata() {
    let normalized = load_html5lib_normalized_suite(HTML5LIB_NORMALIZED_FIXTURES);

    assert_eq!(normalized.format, "venture-html-lexer-fixtures/v1");
    assert_eq!(normalized.suite, "html5lib-smoke");
    assert!(!normalized.description.is_empty());
    assert_eq!(normalized.source, "upstream-html5lib-smoke.test");
    assert_eq!(normalized.generator, "normalize_html5lib_fixtures.py");
    assert_eq!(
        normalized.supported_initial_states,
        vec![
            "CDATA section state".to_string(),
            "Data state".to_string(),
            "PLAINTEXT state".to_string(),
            "RAWTEXT state".to_string(),
            "RCDATA state".to_string(),
            "Script data double escaped state".to_string(),
            "Script data escaped state".to_string(),
            "Script data state".to_string()
        ]
    );
    assert_eq!(normalized.cases.len(), 101);
    assert!(normalized.skipped.is_empty());
    assert_eq!(
        normalized.cases[4].diagnostics,
        vec!["missing-doctype-name".to_string()]
    );
    assert_eq!(
        normalized.cases[5].diagnostics,
        vec!["eof-in-doctype".to_string()]
    );
    assert_eq!(
        normalized.cases[6].diagnostics,
        vec!["eof-in-doctype".to_string()]
    );
    assert_eq!(
        normalized.cases[7].diagnostics,
        vec!["invalid-doctype-keyword".to_string()]
    );
    assert_eq!(
        normalized.cases[14].diagnostics,
        vec!["missing-doctype-public-identifier".to_string()]
    );
    assert_eq!(
        normalized.cases[19].diagnostics,
        vec!["missing-doctype-system-identifier".to_string()]
    );
    assert_eq!(
        normalized.cases[20].diagnostics,
        vec!["unexpected-character-after-doctype-system-identifier".to_string()]
    );
    assert_eq!(
        normalized.cases[23].diagnostics,
        vec!["abrupt-closing-of-empty-comment".to_string()]
    );
    assert_eq!(
        normalized.cases[25].initial_state.as_deref(),
        Some("RCDATA state")
    );
    assert_eq!(
        normalized.cases[25].last_start_tag.as_deref(),
        Some("title")
    );
    assert_eq!(
        normalized.cases[26].initial_state.as_deref(),
        Some("RCDATA state")
    );
    assert_eq!(
        normalized.cases[26].last_start_tag.as_deref(),
        Some("title")
    );
    assert_eq!(
        normalized.cases[27].initial_state.as_deref(),
        Some("RAWTEXT state")
    );
    assert_eq!(
        normalized.cases[27].last_start_tag.as_deref(),
        Some("style")
    );
}

#[test]
fn bootstrap_skeleton_conformance_cases_match_generated_machine() {
    let suite = load_suite(HTML_SKELETON_FIXTURES);
    run_fixture_suite(&suite, |_case| {
        html_skeleton_machine()
            .map(HtmlLexer::new)
            .map_err(|error| error.to_string())
    });
}

#[test]
fn html1_conformance_cases_match_generated_machine() {
    let suite = load_suite(HTML1_FIXTURES);
    run_fixture_suite(&suite, |case| {
        let mut lexer = html1_machine()
            .map(HtmlLexer::new)
            .map_err(|error| error.to_string())?;
        configure_lexer_for_case(&mut lexer, case).map_err(|error| format!("{error:?}"))?;
        Ok(lexer)
    });
}

#[test]
fn html1_conformance_cases_match_default_wrapper() {
    let suite = load_suite(HTML1_FIXTURES);
    run_fixture_suite(&suite, |case| {
        let mut lexer = create_html_lexer().map_err(|error| format!("{error:?}"))?;
        configure_lexer_for_case(&mut lexer, case).map_err(|error| format!("{error:?}"))?;
        Ok(lexer)
    });
}

#[test]
fn normalized_html5lib_cases_match_default_wrapper() {
    let normalized = load_html5lib_normalized_suite(HTML5LIB_NORMALIZED_FIXTURES);
    let suite = executable_suite_from_normalized(&normalized);

    assert_eq!(suite.format, "venture-html-lexer-fixtures/v1");
    assert_eq!(suite.suite, "html5lib-smoke");
    assert_eq!(suite.cases.len(), 101);

    run_fixture_suite(&suite, |case| {
        let mut lexer = create_html_lexer().map_err(|error| format!("{error:?}"))?;
        configure_lexer_for_case(&mut lexer, case).map_err(|error| format!("{error:?}"))?;
        Ok(lexer)
    });
}

#[test]
fn normalized_html5lib_cases_match_generated_html1_machine() {
    let normalized = load_html5lib_normalized_suite(HTML5LIB_NORMALIZED_FIXTURES);
    let suite = executable_suite_from_normalized(&normalized);

    run_fixture_suite(&suite, |case| {
        let mut lexer = html1_machine()
            .map(HtmlLexer::new)
            .map_err(|error| error.to_string())?;
        configure_lexer_for_case(&mut lexer, case).map_err(|error| format!("{error:?}"))?;
        Ok(lexer)
    });
}

#[test]
fn normalized_html5lib_cases_have_no_remaining_runtime_gaps() {
    let normalized = load_html5lib_normalized_suite(HTML5LIB_NORMALIZED_FIXTURES);
    let unsupported = unsupported_runtime_cases(&normalized);

    assert!(
        unsupported.is_empty(),
        "unexpected runtime gaps: {unsupported:?}"
    );
}

fn load_suite(raw: &str) -> FixtureSuite {
    serde_json::from_str(raw).expect("fixture suite should parse")
}

fn load_html5lib_file(raw: &str) -> Html5libTokenizerFile {
    serde_json::from_str(raw).expect("html5lib tokenizer fixture file should parse")
}

fn load_html5lib_normalized_suite(raw: &str) -> Html5libNormalizedSuite {
    serde_json::from_str(raw).expect("normalized html5lib fixture suite should parse")
}

fn executable_suite_from_normalized(normalized: &Html5libNormalizedSuite) -> FixtureSuite {
    FixtureSuite {
        format: normalized.format.clone(),
        suite: normalized.suite.clone(),
        description: format!("{} (runtime-executable corpus)", normalized.description),
        cases: normalized
            .cases
            .iter()
            .filter(|case| is_supported_by_current_runtime(case))
            .cloned()
            .collect(),
    }
}

fn unsupported_runtime_cases(normalized: &Html5libNormalizedSuite) -> Vec<FixtureCase> {
    normalized
        .cases
        .iter()
        .filter(|case| !is_supported_by_current_runtime(case))
        .cloned()
        .collect()
}

fn is_supported_by_current_runtime(case: &FixtureCase) -> bool {
    match case.initial_state.as_deref() {
        None | Some("Data state") => case.last_start_tag.is_none(),
        Some("CDATA section state") => case.last_start_tag.is_none(),
        Some("PLAINTEXT state") => case.last_start_tag.is_none(),
        Some("RCDATA state") => case.last_start_tag.is_some(),
        Some("RAWTEXT state") => case.last_start_tag.is_some(),
        Some("Script data double escaped state") => case.last_start_tag.is_some(),
        Some("Script data escaped state") => case.last_start_tag.is_some(),
        Some("Script data state") => case.last_start_tag.is_some(),
        Some(_) => false,
    }
}

fn run_fixture_suite(
    suite: &FixtureSuite,
    create_lexer: impl Fn(&FixtureCase) -> Result<HtmlLexer, String>,
) {
    for case in &suite.cases {
        let mut lexer = create_lexer(case).unwrap_or_else(|error| {
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

fn configure_lexer_for_case(
    lexer: &mut HtmlLexer,
    case: &FixtureCase,
) -> coding_adventures_html_lexer::Result<()> {
    if let Some(initial_state) = case.initial_state.as_deref() {
        lexer.set_initial_state(machine_state_for_fixture(initial_state))?;
    }
    if let Some(last_start_tag) = case.last_start_tag.as_deref() {
        lexer.set_last_start_tag(last_start_tag);
    }
    Ok(())
}

fn machine_state_for_fixture(state: &str) -> &str {
    match state {
        "Data state" => "data",
        "CDATA section state" => "cdata_section",
        "PLAINTEXT state" => "plaintext",
        "RCDATA state" => "rcdata",
        "RAWTEXT state" => "rawtext",
        "Script data double escaped state" => "script_data_double_escaped",
        "Script data escaped state" => "script_data_escaped",
        "Script data state" => "script_data",
        other => panic!("unsupported fixture state `{other}`"),
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
