use coding_adventures_html_lexer::{Attribute, Diagnostic, Token};

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
