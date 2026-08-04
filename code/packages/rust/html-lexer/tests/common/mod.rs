use coding_adventures_html_lexer::{create_html_lexer, Attribute, Diagnostic, HtmlLexer, Token};

pub fn assert_suite_metadata<'a, I>(
    actual_format: &str,
    description: &str,
    case_ids: I,
    expected_format: &str,
    minimum_cases: usize,
    required_case_ids: &[&str],
) where
    I: IntoIterator<Item = &'a str>,
{
    let case_ids = case_ids.into_iter().collect::<Vec<_>>();

    assert_eq!(actual_format, expected_format);
    assert!(!description.is_empty());
    assert!(
        case_ids.len() >= minimum_cases,
        "fixture should include at least {minimum_cases} cases"
    );
    for required_case_id in required_case_ids {
        assert!(
            case_ids.contains(required_case_id),
            "fixture should include case `{required_case_id}`"
        );
    }
}

pub fn assert_case_description(case_id: &str, description: &str, label: &str) {
    assert!(
        !description.is_empty(),
        "case `{case_id}` should describe its {label}"
    );
}

#[allow(dead_code)]
pub fn assert_default_lexer_case(
    case_id: &str,
    input: &str,
    expected_tokens: &[String],
    expected_diagnostics: &[String],
) {
    let mut lexer = create_html_lexer().expect("HTML lexer should build");
    assert_lexer_case(
        case_id,
        &mut lexer,
        input,
        expected_tokens,
        expected_diagnostics,
    );
}

pub fn assert_lexer_case(
    case_id: &str,
    lexer: &mut HtmlLexer,
    input: &str,
    expected_tokens: &[String],
    expected_diagnostics: &[String],
) {
    lexer
        .push(input)
        .unwrap_or_else(|error| panic!("case `{case_id}` push failed: {error:?}"));
    lexer
        .finish()
        .unwrap_or_else(|error| panic!("case `{case_id}` finish failed: {error:?}"));

    assert_token_summaries(case_id, lexer.drain_tokens(), expected_tokens);
    assert_diagnostic_codes(case_id, lexer.diagnostics(), expected_diagnostics);
}

pub fn assert_token_summaries(
    case_id: &str,
    actual_tokens: Vec<Token>,
    expected_tokens: &[String],
) {
    assert_eq!(
        token_summaries(actual_tokens),
        coalesce_adjacent_text_summaries(expected_tokens.to_vec()),
        "case `{case_id}` token mismatch"
    );
}

pub fn assert_diagnostic_codes(
    case_id: &str,
    actual_diagnostics: &[Diagnostic],
    expected_diagnostics: &[String],
) {
    assert_eq!(
        diagnostic_codes(actual_diagnostics),
        expected_diagnostics,
        "case `{case_id}` diagnostic mismatch"
    );
}

fn token_summaries(tokens: Vec<Token>) -> Vec<String> {
    coalesce_adjacent_text_summaries(tokens.into_iter().map(token_summary).collect())
}

fn diagnostic_codes(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect()
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
        Token::ProcessingInstruction { target, data } => {
            format!("ProcessingInstruction(target={target}, data={data})")
        }
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
