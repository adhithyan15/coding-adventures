use coding_adventures_html_lexer::{
    create_html_lexer, html_skeleton_definition, html_skeleton_machine, lex_html, Token,
};
use state_machine::END_INPUT;

#[test]
fn html_skeleton_lexes_text_start_tag_end_tag_and_eof() {
    let tokens = lex_html("<p>Hello</p>").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "p".to_string(),
                attributes: Vec::new(),
                self_closing: false,
            },
            Token::Text("Hello".to_string()),
            Token::EndTag {
                name: "p".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn html_skeleton_supports_chunked_input_and_unicode_any_matcher() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Hello ").unwrap();
    lexer.push("<B>").unwrap();
    lexer.push("snowman: \u{2603}").unwrap();
    lexer.push("</B>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Hello ".to_string()),
            Token::StartTag {
                name: "b".to_string(),
                attributes: Vec::new(),
                self_closing: false,
            },
            Token::Text("snowman: \u{2603}".to_string()),
            Token::EndTag {
                name: "b".to_string()
            },
            Token::Eof,
        ]
    );
    assert!(lexer
        .trace()
        .iter()
        .any(|entry| entry.input == Some('\u{2603}')));
}

#[test]
fn html_skeleton_flushes_text_at_eof() {
    assert_eq!(
        lex_html("plain text").unwrap(),
        vec![Token::Text("plain text".to_string()), Token::Eof]
    );
}

#[test]
fn html_skeleton_reports_recoverable_eof_diagnostic() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![Token::Text("<".to_string()), Token::Eof]
    );
    assert_eq!(lexer.diagnostics().len(), 1);
    assert_eq!(lexer.diagnostics()[0].code, "eof-in-tag-open-state");
}

#[test]
fn html_skeleton_machine_exports_definition_with_eof_matcher() {
    let definition = html_skeleton_machine()
        .unwrap()
        .to_definition("html-skeleton-lexer");

    assert!(definition.transitions.iter().any(|transition| {
        transition.on.as_deref() == Some(END_INPUT)
            && transition
                .actions
                .iter()
                .any(|action| action == "emit(EOF)")
            && !transition.consume
    }));
}

#[test]
fn html_skeleton_generated_definition_preserves_lexer_profile_metadata() {
    let definition = html_skeleton_definition();

    assert_eq!(definition.profile.as_deref(), Some("lexer/v1"));
    assert_eq!(
        definition.runtime_min.as_deref(),
        Some("state-machine-tokenizer/0.1")
    );
    assert_eq!(definition.done.as_deref(), Some("done"));
    assert_eq!(definition.tokens.len(), 4);
    assert!(definition
        .registers
        .iter()
        .any(|register| register.id == "text_buffer"));
    assert_eq!(definition.fixtures.len(), 2);
}

#[test]
fn html_skeleton_generated_fixtures_match_runtime_tokens() {
    let definition = html_skeleton_definition();

    for fixture in definition.fixtures {
        let actual = lex_html(&fixture.input)
            .unwrap()
            .into_iter()
            .map(token_summary)
            .collect::<Vec<_>>();
        assert_eq!(
            actual, fixture.tokens,
            "fixture `{}` should still match",
            fixture.name
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

fn attribute_summary(attributes: &[coding_adventures_html_lexer::Attribute]) -> String {
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
