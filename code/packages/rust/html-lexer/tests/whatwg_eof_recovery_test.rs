use coding_adventures_html_lexer::{
    apply_html_lex_context, create_html_lexer, Attribute, DoctypeSeed, HtmlLexContext, HtmlLexer,
    HtmlTokenizerState, Token,
};
use serde::Deserialize;

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

fn load_suite() -> EofRecoverySuite {
    serde_json::from_str(WHATWG_EOF_RECOVERY).expect("WHATWG EOF recovery fixture should parse")
}

fn configured_lexer(case: &EofRecoveryCase) -> HtmlLexer {
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

    let mut lexer = create_html_lexer().expect("HTML lexer should build");
    apply_html_lex_context(&mut lexer, &context).expect("HTML lexer context should apply");
    lexer
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
