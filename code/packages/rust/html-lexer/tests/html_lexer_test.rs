use coding_adventures_html_lexer::{create_html_lexer, html_skeleton_machine, lex_html, Token};
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
