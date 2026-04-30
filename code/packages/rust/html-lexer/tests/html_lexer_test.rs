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
                public_identifier: None,
                system_identifier: None,
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
fn default_html_lexer_reports_and_drops_duplicate_attributes() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<a href=one HREF=two title=ok>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "href".to_string(),
                        value: "one".to_string(),
                    },
                    Attribute {
                        name: "title".to_string(),
                        value: "ok".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
    assert_eq!(
        lexer
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == "duplicate-attribute")
            .count(),
        1
    );
}

#[test]
fn default_html_lexer_reports_unexpected_chars_in_unquoted_attribute_values() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<a data=one=two sq=x'y lt=x<y eq=x=y tick=x`y dq=x\"y>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "data".to_string(),
                        value: "one=two".to_string(),
                    },
                    Attribute {
                        name: "sq".to_string(),
                        value: "x'y".to_string(),
                    },
                    Attribute {
                        name: "lt".to_string(),
                        value: "x<y".to_string(),
                    },
                    Attribute {
                        name: "eq".to_string(),
                        value: "x=y".to_string(),
                    },
                    Attribute {
                        name: "tick".to_string(),
                        value: "x`y".to_string(),
                    },
                    Attribute {
                        name: "dq".to_string(),
                        value: "x\"y".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
    assert_eq!(
        lexer
            .diagnostics()
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "unexpected-character-in-unquoted-attribute-value"
            })
            .count(),
        6
    );
}

#[test]
fn default_html_lexer_replaces_null_characters_in_text_and_attributes() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("A\0<a title=\"x\0y\" data=x\0y bare=\0>z")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("A\u{FFFD}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "title".to_string(),
                        value: "x\u{FFFD}y".to_string(),
                    },
                    Attribute {
                        name: "data".to_string(),
                        value: "x\u{FFFD}y".to_string(),
                    },
                    Attribute {
                        name: "bare".to_string(),
                        value: "\u{FFFD}".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Text("z".to_string()),
            Token::Eof,
        ]
    );
    assert_eq!(
        lexer
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == "unexpected-null-character")
            .count(),
        4
    );
}

#[test]
fn default_html_lexer_marks_missing_doctype_name_force_quirks() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<!DOCTYPE>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Doctype {
                name: None,
                public_identifier: None,
                system_identifier: None,
                force_quirks: true,
            },
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "missing-doctype-name"));
}

#[test]
fn default_html_lexer_marks_whitespace_only_doctype_force_quirks() {
    let tokens = lex_html("<!DOCTYPE >").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Doctype {
                name: None,
                public_identifier: None,
                system_identifier: None,
                force_quirks: true,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_marks_doctype_name_eof_force_quirks() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<!DOCTYPE html").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Doctype {
                name: Some("html".to_string()),
                public_identifier: None,
                system_identifier: None,
                force_quirks: true,
            },
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "eof-in-doctype"));
}

#[test]
fn default_html_lexer_marks_doctype_keyword_eof_force_quirks() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<!DOC").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Doctype {
                name: None,
                public_identifier: None,
                system_identifier: None,
                force_quirks: true,
            },
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "eof-in-doctype"));
}

#[test]
fn default_html_lexer_marks_invalid_doctype_keyword_force_quirks() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<!DOX>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Doctype {
                name: Some("dox".to_string()),
                public_identifier: None,
                system_identifier: None,
                force_quirks: true,
            },
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "invalid-doctype-keyword"));
}

#[test]
fn default_html_lexer_marks_after_doctype_name_eof_force_quirks() {
    let tokens = lex_html("<!DOCTYPE html ").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Doctype {
                name: Some("html".to_string()),
                public_identifier: None,
                system_identifier: None,
                force_quirks: true,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_public_doctype_identifier() {
    let tokens = lex_html("<!DOCTYPE html PUBLIC \"-//IETF//DTD HTML//EN\">").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Doctype {
                name: Some("html".to_string()),
                public_identifier: Some("-//IETF//DTD HTML//EN".to_string()),
                system_identifier: None,
                force_quirks: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_public_and_system_doctype_identifiers() {
    let tokens =
        lex_html("<!DOCTYPE html PUBLIC '-//W3C//DTD HTML 4.01//EN' \"about:legacy-compat\">")
            .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Doctype {
                name: Some("html".to_string()),
                public_identifier: Some("-//W3C//DTD HTML 4.01//EN".to_string()),
                system_identifier: Some("about:legacy-compat".to_string()),
                force_quirks: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_standalone_system_doctype_identifier() {
    let tokens = lex_html("<!DOCTYPE html SYSTEM \"about:legacy-compat\">").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Doctype {
                name: Some("html".to_string()),
                public_identifier: None,
                system_identifier: Some("about:legacy-compat".to_string()),
                force_quirks: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_marks_missing_public_identifier_force_quirks() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<!DOCTYPE html PUBLIC>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Doctype {
                name: Some("html".to_string()),
                public_identifier: None,
                system_identifier: None,
                force_quirks: true,
            },
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "missing-doctype-public-identifier"));
}

#[test]
fn default_html_lexer_marks_missing_system_identifier_force_quirks() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<!DOCTYPE html SYSTEM>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Doctype {
                name: Some("html".to_string()),
                public_identifier: None,
                system_identifier: None,
                force_quirks: true,
            },
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "missing-doctype-system-identifier"));
}

#[test]
fn default_html_lexer_marks_trailing_junk_after_system_identifier_force_quirks() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<!DOCTYPE html SYSTEM \"about:legacy-compat\" junk>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Doctype {
                name: Some("html".to_string()),
                public_identifier: None,
                system_identifier: Some("about:legacy-compat".to_string()),
                force_quirks: true,
            },
            Token::Eof,
        ]
    );
    assert!(lexer.diagnostics().iter().any(
        |diagnostic| diagnostic.code == "unexpected-character-after-doctype-system-identifier"
    ));
}

#[test]
fn default_html_lexer_recovers_question_mark_tag_open_as_bogus_comment() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before<?xml version=\"1.0\"?>After").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::Comment("?xml version=\"1.0\"?".to_string()),
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "unexpected-question-mark-instead-of-tag-name"));
}

#[test]
fn default_html_lexer_does_not_report_eof_in_bogus_comment_state() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before<?xml").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::Comment("?xml".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "unexpected-question-mark-instead-of-tag-name"));
    assert!(!lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "eof-in-comment"));
}

#[test]
fn default_html_lexer_reports_incorrectly_opened_markup_declaration() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before<!foo>After").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::Comment("foo".to_string()),
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "incorrectly-opened-comment"));
}

#[test]
fn default_html_lexer_reconsumes_empty_incorrectly_opened_markup_declaration() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before<!>After").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::Comment(String::new()),
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "incorrectly-opened-comment"));
}

#[test]
fn default_html_lexer_recovers_markup_declaration_eof_as_empty_comment() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before<!").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::Comment(String::new()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "incorrectly-opened-comment"));
    assert!(!lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "eof-in-markup-declaration-open-state"));
}

#[test]
fn default_html_lexer_reports_one_dash_markup_declaration_as_incorrectly_opened() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before<!->Middle<!-x>After<!-").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::Comment("-".to_string()),
            Token::Text("Middle".to_string()),
            Token::Comment("-x".to_string()),
            Token::Text("After".to_string()),
            Token::Comment("-".to_string()),
            Token::Eof,
        ]
    );
    assert_eq!(
        lexer
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == "incorrectly-opened-comment")
            .count(),
        3
    );
    assert!(!lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "abrupt-closing-of-empty-comment"));
    assert!(!lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "eof-in-comment"));
}

#[test]
fn default_html_lexer_recovers_invalid_tag_open_as_text() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before < after").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before ".to_string()),
            Token::Text("< after".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "invalid-first-character-of-tag-name"));
}

#[test]
fn default_html_lexer_recovers_invalid_end_tag_open_as_bogus_comment() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before</3>After").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::Comment("3".to_string()),
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "invalid-first-character-of-tag-name"));
}

#[test]
fn default_html_lexer_recovers_abrupt_empty_html_comment() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before<!-->After").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::Comment(String::new()),
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "abrupt-closing-of-empty-comment"));
}

#[test]
fn default_html_lexer_closes_dash_prefixed_empty_html_comment() {
    let tokens = lex_html("Before<!--->After").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Before".to_string()),
            Token::Comment(String::new()),
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_reports_nested_comment_opener() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before<!--a <!-- b -->After").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::Comment("a <!-- b ".to_string()),
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "nested-comment"));
}

#[test]
fn default_html_lexer_recovers_incorrectly_closed_comment() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before<!--x--!>After").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::Comment("x".to_string()),
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "incorrectly-closed-comment"));
}

#[test]
fn default_html_lexer_preserves_bang_after_comment_end_when_not_closing() {
    let tokens = lex_html("Before<!--x--!y-->After").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Before".to_string()),
            Token::Comment("x--!y".to_string()),
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
fn default_html_lexer_replaces_null_characters_in_text_submodes() {
    for initial_state in [
        "rcdata",
        "rawtext",
        "script_data",
        "plaintext",
        "cdata_section",
    ] {
        let mut lexer = create_html_lexer().unwrap();
        lexer.set_initial_state(initial_state).unwrap();

        lexer.push("a\0b").unwrap();
        lexer.finish().unwrap();

        assert_eq!(
            lexer.drain_tokens(),
            vec![Token::Text("a\u{FFFD}b".to_string()), Token::Eof],
            "state {initial_state} should replace NULL characters"
        );
        assert_eq!(
            lexer
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.code == "unexpected-null-character")
                .count(),
            1,
            "state {initial_state} should report NULL recovery"
        );
    }
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
fn default_html_lexer_supports_seeded_script_data_end_tags() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("script_data").unwrap();
    lexer.set_last_start_tag("script");

    lexer.push("if (a < b) alert('&amp;');</script>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("if (a < b) alert('&amp;');".to_string()),
            Token::EndTag {
                name: "script".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_keeps_non_matching_script_end_tags_as_text() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("script_data").unwrap();
    lexer.set_last_start_tag("script");

    lexer.push("x</style>y").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![Token::Text("x</style>y".to_string()), Token::Eof]
    );
}

#[test]
fn default_html_lexer_supports_script_data_escaped_comment_text() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("script_data").unwrap();
    lexer.set_last_start_tag("script");

    lexer.push("<!-- if (a < b) --></script>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("<!-- if (a < b) -->".to_string()),
            Token::EndTag {
                name: "script".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_script_data_escaped_state() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("script_data_escaped").unwrap();
    lexer.set_last_start_tag("script");

    lexer.push("if (a < b) </script>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("if (a < b) ".to_string()),
            Token::EndTag {
                name: "script".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_script_data_double_escaped_text() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("script_data").unwrap();
    lexer.set_last_start_tag("script");

    lexer
        .push("<!-- <script>ignored </script> still escaped --></script>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("<!-- <script>ignored </script> still escaped -->".to_string()),
            Token::EndTag {
                name: "script".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_script_data_double_escaped_state() {
    let mut lexer = create_html_lexer().unwrap();
    lexer
        .set_initial_state("script_data_double_escaped")
        .unwrap();
    lexer.set_last_start_tag("script");

    lexer.push("x </script> y --></script>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("x </script> y -->".to_string()),
            Token::EndTag {
                name: "script".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_plaintext_state() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("plaintext").unwrap();

    lexer.push("hello <b>still text</b> &amp; literal").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("hello <b>still text</b> &amp; literal".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_cdata_section_state() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("cdata_section").unwrap();

    lexer.push("a <b> &amp; ]]><p>x</p>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("a <b> &amp; ".to_string()),
            Token::StartTag {
                name: "p".to_string(),
                attributes: Vec::new(),
                self_closing: false,
            },
            Token::Text("x".to_string()),
            Token::EndTag {
                name: "p".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_markup_cdata_section_flow() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("Before<![CDATA[<not-markup> &amp; ]]>After")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::Text("<not-markup> &amp; After".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_recovers_malformed_cdata_open_as_bogus_comment() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<![CDX>after").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Comment("[CDX".to_string()),
            Token::Text("after".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_keeps_unclosed_cdata_brackets_as_text_at_eof() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("cdata_section").unwrap();

    lexer.push("a]]").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![Token::Text("a]]".to_string()), Token::Eof]
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
fn default_html_lexer_supports_legacy_named_character_references() {
    let tokens = lex_html("Legacy&nbsp;symbols: &copy; &reg;").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Legacy\u{00A0}symbols: \u{00A9} \u{00AE}".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_legacy_named_character_references_in_attributes() {
    let tokens = lex_html("<a title=\"A&nbsp;B\" copy='&copy;' reg=&reg;>").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "title".to_string(),
                        value: "A\u{00A0}B".to_string(),
                    },
                    Attribute {
                        name: "copy".to_string(),
                        value: "\u{00A9}".to_string(),
                    },
                    Attribute {
                        name: "reg".to_string(),
                        value: "\u{00AE}".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_legacy_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer.push("Venture&nbsp;&copy;&reg;</title>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Venture\u{00A0}\u{00A9}\u{00AE}".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_latin1_named_character_references_in_data() {
    let tokens = lex_html("Latin-1: &Agrave;&agrave; &frac12; &yen;").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Latin-1: \u{00C0}\u{00E0} \u{00BD} \u{00A5}".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_latin1_named_character_references_in_attributes() {
    let tokens = lex_html("<a title=\"&AElig;&aelig;\" currency=&pound;>").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "title".to_string(),
                        value: "\u{00C6}\u{00E6}".to_string(),
                    },
                    Attribute {
                        name: "currency".to_string(),
                        value: "\u{00A3}".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_latin1_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer.push("&Ntilde;&ntilde;</title>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("\u{00D1}\u{00F1}".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_semicolonless_legacy_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("Legacy&nbsp symbols: &copy/&reg <a title=\"A&nbsp B\" copy=&copy reg=&reg>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Legacy\u{00A0} symbols: \u{00A9}/\u{00AE} ".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "title".to_string(),
                        value: "A\u{00A0} B".to_string(),
                    },
                    Attribute {
                        name: "copy".to_string(),
                        value: "\u{00A9}".to_string(),
                    },
                    Attribute {
                        name: "reg".to_string(),
                        value: "\u{00AE}".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
    assert_eq!(
        lexer
            .diagnostics()
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "missing-semicolon-after-character-reference"
            })
            .count(),
        6
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_semicolonless_legacy_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer.push("Venture&nbsp &copy &reg</title>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Venture\u{00A0} \u{00A9} \u{00AE}".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
    assert_eq!(
        lexer
            .diagnostics()
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "missing-semicolon-after-character-reference"
            })
            .count(),
        3
    );
}

#[test]
fn default_html_lexer_falls_back_for_unknown_named_character_references() {
    let tokens =
        lex_html("Known &AMP; unknown &madeup; <a title=\"&copy;\" bogus=&madeup;>").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Known & unknown &madeup; ".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "title".to_string(),
                        value: "\u{00A9}".to_string(),
                    },
                    Attribute {
                        name: "bogus".to_string(),
                        value: "&madeup;".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_numeric_character_references_in_data() {
    let tokens = lex_html("Letters: &#65; &#x42; &#X43; &#0;").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Letters: A B C \u{FFFD}".to_string()),
            Token::Eof
        ]
    );
}

#[test]
fn default_html_lexer_supports_numeric_character_references_in_attributes() {
    let tokens = lex_html("<a title=\"&#65;&#x42;\" data-x='&#X43;' note=&#0;>").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "title".to_string(),
                        value: "AB".to_string(),
                    },
                    Attribute {
                        name: "data-x".to_string(),
                        value: "C".to_string(),
                    },
                    Attribute {
                        name: "note".to_string(),
                        value: "\u{FFFD}".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_numeric_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer.push("Fish &#38; &#x3C;/title>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![Token::Text("Fish & </title>".to_string()), Token::Eof]
    );
}

#[test]
fn default_html_lexer_supports_missing_semicolon_numeric_character_references() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("Letters: &#65 &#x42Z <a title=&#67 data-x=&#x44>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Letters: A BZ ".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "title".to_string(),
                        value: "C".to_string(),
                    },
                    Attribute {
                        name: "data-x".to_string(),
                        value: "D".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
    assert_eq!(
        lexer
            .diagnostics()
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "missing-semicolon-after-character-reference"
            })
            .count(),
        4
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
    assert_eq!(definition.fixtures.len(), 11);
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
