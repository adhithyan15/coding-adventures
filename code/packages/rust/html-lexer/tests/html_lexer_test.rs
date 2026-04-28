use coding_adventures_html_lexer::{
    create_html_lexer, html1_definition, html1_machine, html_skeleton_definition,
    html_skeleton_machine, lex_html, Attribute, Token,
};
use state_machine::END_INPUT;

#[test]
fn default_html_lexer_still_lexes_basic_text_tags_and_eof() {
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
fn default_html_lexer_supports_html1_attributes_comments_and_doctypes() {
    let tokens = lex_html(
        "<!DOCTYPE HTML><IMG SRC=\"mosaic.gif\" ALT='Splash' hidden=1/>Before<!--note-->After",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Doctype {
                name: Some("html".to_string()),
                force_quirks: false,
            },
            Token::StartTag {
                name: "img".to_string(),
                attributes: vec![
                    Attribute {
                        name: "src".to_string(),
                        value: "mosaic.gif".to_string(),
                    },
                    Attribute {
                        name: "alt".to_string(),
                        value: "Splash".to_string(),
                    },
                    Attribute {
                        name: "hidden".to_string(),
                        value: "1".to_string(),
                    },
                ],
                self_closing: true,
            },
            Token::Text("Before".to_string()),
            Token::Comment("note".to_string()),
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_chunked_input_and_unicode_any_matcher() {
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
fn default_html_lexer_reports_recoverable_comment_eof_diagnostic() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<!--open").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![Token::Comment("open".to_string()), Token::Eof]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "eof-in-comment"));
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_end_tags() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer.push("Hello</title>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Hello".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_lt_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer.push("Hello&lt;/title>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![Token::Text("Hello</title>".to_string()), Token::Eof]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rawtext_end_tags() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rawtext").unwrap();
    lexer.set_last_start_tag("style");

    lexer.push("body { color: red; }</style>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("body { color: red; }".to_string()),
            Token::EndTag {
                name: "style".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_keeps_ampersands_literal_in_seeded_rawtext() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rawtext").unwrap();
    lexer.set_last_start_tag("style");

    lexer.push("body &lt; </style>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("body &lt; ".to_string()),
            Token::EndTag {
                name: "style".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_named_character_references_in_data() {
    let tokens = lex_html("Fish &amp; &lt;b&gt; &quot;quote&quot; &apos;ok&apos;").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Fish & <b> \"quote\" 'ok'".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_named_character_references_in_attributes() {
    let tokens =
        lex_html("<a title=\"Fish &amp; Chips\" data-x='&lt;ok&gt;' note=&quot;hi&quot;>").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "title".to_string(),
                        value: "Fish & Chips".to_string(),
                    },
                    Attribute {
                        name: "data-x".to_string(),
                        value: "<ok>".to_string(),
                    },
                    Attribute {
                        name: "note".to_string(),
                        value: "\"hi\"".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer.push("Fish &amp; &lt;/title>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![Token::Text("Fish & </title>".to_string()), Token::Eof]
    );
}

#[test]
fn html1_machine_exports_definition_with_eof_matcher() {
    let definition = html1_machine().unwrap().to_definition("html1-lexer");

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
fn html1_generated_definition_preserves_lexer_profile_metadata() {
    let definition = html1_definition();

    assert_eq!(definition.profile.as_deref(), Some("lexer/v1"));
    assert_eq!(
        definition.runtime_min.as_deref(),
        Some("state-machine-tokenizer/0.1")
    );
    assert_eq!(definition.done.as_deref(), Some("done"));
    assert_eq!(definition.tokens.len(), 6);
    assert!(definition
        .registers
        .iter()
        .any(|register| register.id == "temporary_buffer"));
    assert_eq!(definition.fixtures.len(), 6);
}

#[test]
fn default_html_lexer_matches_generated_html1_fixtures() {
    let definition = html1_definition();

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

#[test]
fn html_skeleton_helpers_remain_available_for_bootstrap_comparisons() {
    let definition = html_skeleton_definition();
    assert_eq!(definition.profile.as_deref(), Some("lexer/v1"));
    assert_eq!(definition.tokens.len(), 4);

    let machine = html_skeleton_machine().unwrap();
    let exported = machine.to_definition("html-skeleton-lexer");
    assert!(exported.transitions.iter().any(|transition| {
        transition.on.as_deref() == Some(END_INPUT)
            && transition
                .actions
                .iter()
                .any(|action| action == "emit(EOF)")
    }));
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
