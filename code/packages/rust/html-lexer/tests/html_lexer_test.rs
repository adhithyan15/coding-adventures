use coding_adventures_html_lexer::{
    apply_html_lex_context, create_html_lexer, html1_definition, html1_machine,
    html_skeleton_definition, html_skeleton_machine, lex_html, lex_html_fragment, Attribute,
    HtmlLexContext, HtmlScriptingMode, HtmlTokenizerState, Token,
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
fn default_html_lexer_drops_partial_start_tag_at_eof() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<div class=\"open").unwrap();
    lexer.finish().unwrap();

    assert_eq!(lexer.drain_tokens(), vec![Token::Eof]);
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "eof-in-tag"));
}

#[test]
fn default_html_lexer_drops_partial_end_tag_at_eof() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("</section class=x").unwrap();
    lexer.finish().unwrap();

    assert_eq!(lexer.drain_tokens(), vec![Token::Eof]);
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "eof-in-end-tag-name-state"));
}

#[test]
fn default_html_lexer_drops_partial_attribute_references_at_eof() {
    let cases = [
        (
            "<a href=&copy",
            vec!["missing-semicolon-after-character-reference", "eof-in-tag"],
        ),
        (
            "<a href=&#x41",
            vec!["missing-semicolon-after-character-reference", "eof-in-tag"],
        ),
        (
            "<a href=&#x",
            vec![
                "absence-of-digits-in-numeric-character-reference",
                "eof-in-tag",
            ],
        ),
        ("<a href=&madeup", vec!["eof-in-tag"]),
    ];

    for (input, expected_diagnostics) in cases {
        let mut lexer = create_html_lexer().unwrap();

        lexer.push(input).unwrap();
        lexer.finish().unwrap();

        assert_eq!(lexer.drain_tokens(), vec![Token::Eof], "input {input:?}");
        assert_eq!(
            lexer
                .diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            expected_diagnostics,
            "input {input:?}"
        );
    }
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
fn default_html_lexer_treats_form_feed_as_tag_whitespace() {
    let tokens = lex_html("<p\u{000C}class=test\u{000C}data-x=one\u{000C}/></p\u{000C}>").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "p".to_string(),
                attributes: vec![
                    Attribute {
                        name: "class".to_string(),
                        value: "test".to_string(),
                    },
                    Attribute {
                        name: "data-x".to_string(),
                        value: "one".to_string(),
                    },
                ],
                self_closing: true,
            },
            Token::EndTag {
                name: "p".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_normalizes_carriage_return_newlines() {
    let tokens = lex_html("<p\r\nclass=x>one\rtwo\r\nthree</p>").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "p".to_string(),
                attributes: vec![Attribute {
                    name: "class".to_string(),
                    value: "x".to_string(),
                }],
                self_closing: false,
            },
            Token::Text("one\ntwo\nthree".to_string()),
            Token::EndTag {
                name: "p".to_string()
            },
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
fn default_html_lexer_replaces_null_characters_in_tag_and_attribute_names() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<x\0 \0=v a\0b=1 first \0second=2></x\0>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::StartTag {
                name: "x\u{FFFD}".to_string(),
                attributes: vec![
                    Attribute {
                        name: "\u{FFFD}".to_string(),
                        value: "v".to_string(),
                    },
                    Attribute {
                        name: "a\u{FFFD}b".to_string(),
                        value: "1".to_string(),
                    },
                    Attribute {
                        name: "first".to_string(),
                        value: String::new(),
                    },
                    Attribute {
                        name: "\u{FFFD}second".to_string(),
                        value: "2".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::EndTag {
                name: "x\u{FFFD}".to_string(),
            },
            Token::Eof,
        ]
    );
    assert_eq!(
        lexer
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == "unexpected-null-character")
            .count(),
        5
    );
}

#[test]
fn default_html_lexer_replaces_null_characters_in_doctypes() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<!DOCTYPE\0><!DOCTYPE h\0 PUBLIC \"p\0\" \"s\0\">")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Doctype {
                name: Some("\u{FFFD}".to_string()),
                public_identifier: None,
                system_identifier: None,
                force_quirks: false,
            },
            Token::Doctype {
                name: Some("h\u{FFFD}".to_string()),
                public_identifier: Some("p\u{FFFD}".to_string()),
                system_identifier: Some("s\u{FFFD}".to_string()),
                force_quirks: false,
            },
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
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "missing-whitespace-before-doctype-name"));
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
fn default_html_lexer_reports_missing_whitespace_after_doctype_public_keyword() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<!DOCTYPE html PUBLIC\"-//IETF//DTD HTML//EN\">")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
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
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "missing-whitespace-after-doctype-public-keyword"));
}

#[test]
fn default_html_lexer_reports_missing_whitespace_between_doctype_identifiers() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<!DOCTYPE html PUBLIC '-//W3C//DTD HTML 4.01//EN'\"about:legacy-compat\">")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
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
    assert!(lexer.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == "missing-whitespace-between-doctype-public-and-system-identifiers"
    }));
}

#[test]
fn default_html_lexer_reports_missing_quote_before_doctype_public_identifier() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<!DOCTYPE html PUBLIC id>").unwrap();
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
        .any(|diagnostic| diagnostic.code == "missing-quote-before-doctype-public-identifier"));
}

#[test]
fn default_html_lexer_reports_abrupt_doctype_public_identifier() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<!DOCTYPE html PUBLIC \"unterminated>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Doctype {
                name: Some("html".to_string()),
                public_identifier: Some("unterminated".to_string()),
                system_identifier: None,
                force_quirks: true,
            },
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "abrupt-doctype-public-identifier"));
}

#[test]
fn default_html_lexer_treats_form_feed_as_doctype_whitespace() {
    let tokens = lex_html(
        "<!DOCTYPE\u{000C}html\u{000C}PUBLIC\u{000C}'-//W3C//DTD HTML 4.01//EN'\u{000C}\"about:legacy-compat\"\u{000C}>",
    )
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
fn default_html_lexer_treats_form_feed_as_legacy_reference_boundary() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("A&copy\u{000C}B&nbsp\u{000C}<a copy=&copy\u{000C}reg=&reg\u{000C}>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("A\u{00A9}\u{000C}B\u{00A0}\u{000C}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
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
        4
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
fn default_html_lexer_reports_missing_whitespace_after_doctype_system_keyword() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<!DOCTYPE html SYSTEM\"about:legacy-compat\">")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
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
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "missing-whitespace-after-doctype-system-keyword"));
}

#[test]
fn default_html_lexer_reports_missing_quote_before_doctype_system_identifier() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("<!DOCTYPE html SYSTEM id>").unwrap();
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
        .any(|diagnostic| diagnostic.code == "missing-quote-before-doctype-system-identifier"));
}

#[test]
fn default_html_lexer_reports_abrupt_doctype_system_identifier() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<!DOCTYPE html SYSTEM \"about:legacy-compat>")
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
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "abrupt-doctype-system-identifier"));
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
fn default_html_lexer_recovers_end_tag_with_trailing_solidus() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before</p/>After").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::EndTag {
                name: "p".to_string()
            },
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "end-tag-with-trailing-solidus"));
}

#[test]
fn default_html_lexer_recovers_end_tag_with_whitespace_then_trailing_solidus() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before</p />After").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::EndTag {
                name: "p".to_string()
            },
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "unexpected-whitespace-after-end-tag-name"));
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "end-tag-with-trailing-solidus"));
    assert!(!lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "end-tag-with-attributes"));
}

#[test]
fn default_html_lexer_recovers_end_tag_with_form_feed_then_trailing_solidus() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before</p\u{000C}/>After").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::EndTag {
                name: "p".to_string()
            },
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "unexpected-whitespace-after-end-tag-name"));
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "end-tag-with-trailing-solidus"));
    assert!(!lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "end-tag-with-attributes"));
}

#[test]
fn default_html_lexer_recovers_end_tag_with_attributes() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Before</p class=x data-y>After").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Before".to_string()),
            Token::EndTag {
                name: "p".to_string()
            },
            Token::Text("After".to_string()),
            Token::Eof,
        ]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "unexpected-whitespace-after-end-tag-name"));
    assert_eq!(
        lexer
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == "end-tag-with-attributes")
            .count(),
        1
    );
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
fn default_html_lexer_replaces_null_characters_in_comments() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<!--a\0b--><!--\0--><!--a-\0b--><!--a--\0b--><!foo\0><!-\0>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Comment("a\u{FFFD}b".to_string()),
            Token::Comment("\u{FFFD}".to_string()),
            Token::Comment("a-\u{FFFD}b".to_string()),
            Token::Comment("a--\u{FFFD}b".to_string()),
            Token::Comment("foo\u{FFFD}".to_string()),
            Token::Comment("-\u{FFFD}".to_string()),
            Token::Eof,
        ]
    );
    assert_eq!(
        lexer
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == "unexpected-null-character")
            .count(),
        6
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
fn default_html_lexer_recovers_seeded_text_mode_end_tags_with_trailing_solidus() {
    for (initial_state, last_start_tag, text) in [
        ("rcdata", "title", "Hello"),
        ("rawtext", "style", "body {}"),
        ("script_data", "script", "if (ready)"),
        ("script_data_escaped", "script", "<!-- ok -->"),
    ] {
        let mut lexer = create_html_lexer().unwrap();
        lexer.set_initial_state(initial_state).unwrap();
        lexer.set_last_start_tag(last_start_tag);

        lexer.push(&format!("{text}</{last_start_tag}/>")).unwrap();
        lexer.finish().unwrap();

        assert_eq!(
            lexer.drain_tokens(),
            vec![
                Token::Text(text.to_string()),
                Token::EndTag {
                    name: last_start_tag.to_string()
                },
                Token::Eof,
            ],
            "state {initial_state} should emit the appropriate end tag"
        );
        assert!(
            lexer
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "end-tag-with-trailing-solidus"),
            "state {initial_state} should report trailing solidus recovery"
        );
    }
}

#[test]
fn default_html_lexer_recovers_seeded_text_mode_end_tags_with_whitespace_then_trailing_solidus() {
    for (initial_state, last_start_tag, text) in [
        ("rcdata", "title", "Hello"),
        ("rawtext", "style", "body {}"),
        ("script_data", "script", "if (ready)"),
        ("script_data_escaped", "script", "<!-- ok -->"),
    ] {
        let mut lexer = create_html_lexer().unwrap();
        lexer.set_initial_state(initial_state).unwrap();
        lexer.set_last_start_tag(last_start_tag);

        lexer.push(&format!("{text}</{last_start_tag} />")).unwrap();
        lexer.finish().unwrap();

        assert_eq!(
            lexer.drain_tokens(),
            vec![
                Token::Text(text.to_string()),
                Token::EndTag {
                    name: last_start_tag.to_string()
                },
                Token::Eof,
            ],
            "state {initial_state} should emit the appropriate end tag"
        );
        assert!(
            lexer
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "end-tag-with-trailing-solidus"),
            "state {initial_state} should report trailing solidus recovery"
        );
        assert!(
            !lexer
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "end-tag-with-attributes"),
            "state {initial_state} should not treat the solidus as attributes"
        );
    }
}

#[test]
fn default_html_lexer_recovers_seeded_text_mode_end_tags_with_form_feed() {
    for (initial_state, last_start_tag, text) in [
        ("rcdata", "title", "Hello"),
        ("rawtext", "style", "body {}"),
        ("script_data", "script", "if (ready)"),
        ("script_data_escaped", "script", "<!-- ok -->"),
    ] {
        let mut lexer = create_html_lexer().unwrap();
        lexer.set_initial_state(initial_state).unwrap();
        lexer.set_last_start_tag(last_start_tag);

        lexer
            .push(&format!("{text}</{last_start_tag}\u{000C}>"))
            .unwrap();
        lexer.finish().unwrap();

        assert_eq!(
            lexer.drain_tokens(),
            vec![
                Token::Text(text.to_string()),
                Token::EndTag {
                    name: last_start_tag.to_string()
                },
                Token::Eof,
            ],
            "state {initial_state} should emit the appropriate end tag"
        );
        assert!(
            lexer
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "unexpected-whitespace-after-end-tag-name"),
            "state {initial_state} should report form-feed whitespace recovery"
        );
    }
}

#[test]
fn default_html_lexer_recovers_seeded_text_mode_end_tags_with_form_feed_then_trailing_solidus() {
    for (initial_state, last_start_tag, text) in [
        ("rcdata", "title", "Hello"),
        ("rawtext", "style", "body {}"),
        ("script_data", "script", "if (ready)"),
        ("script_data_escaped", "script", "<!-- ok -->"),
    ] {
        let mut lexer = create_html_lexer().unwrap();
        lexer.set_initial_state(initial_state).unwrap();
        lexer.set_last_start_tag(last_start_tag);

        lexer
            .push(&format!("{text}</{last_start_tag}\u{000C}/>"))
            .unwrap();
        lexer.finish().unwrap();

        assert_eq!(
            lexer.drain_tokens(),
            vec![
                Token::Text(text.to_string()),
                Token::EndTag {
                    name: last_start_tag.to_string()
                },
                Token::Eof,
            ],
            "state {initial_state} should emit the appropriate end tag"
        );
        assert!(
            lexer
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "end-tag-with-trailing-solidus"),
            "state {initial_state} should report trailing solidus recovery"
        );
    }
}

#[test]
fn default_html_lexer_keeps_mismatched_text_mode_end_tag_solidus_literal() {
    for close in ["</style/>", "</style />"] {
        let mut lexer = create_html_lexer().unwrap();
        lexer.set_initial_state("script_data").unwrap();
        lexer.set_last_start_tag("script");

        lexer.push(&format!("x{close}y</script>")).unwrap();
        lexer.finish().unwrap();

        assert_eq!(
            lexer.drain_tokens(),
            vec![
                Token::Text(format!("x{close}y")),
                Token::EndTag {
                    name: "script".to_string()
                },
                Token::Eof,
            ]
        );
        assert!(!lexer
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == "end-tag-with-trailing-solidus"));
    }
}

#[test]
fn default_html_lexer_recovers_seeded_text_mode_end_tags_with_whitespace() {
    for (initial_state, last_start_tag, text, close) in [
        ("rcdata", "title", "Hello", "</title >"),
        ("rawtext", "style", "body {}", "</style\t>"),
        ("script_data", "script", "if (ready)", "</script\n>"),
        (
            "script_data_escaped",
            "script",
            "<!-- ok -->",
            "</script\r>",
        ),
    ] {
        let mut lexer = create_html_lexer().unwrap();
        lexer.set_initial_state(initial_state).unwrap();
        lexer.set_last_start_tag(last_start_tag);

        lexer.push(&format!("{text}{close}")).unwrap();
        lexer.finish().unwrap();

        assert_eq!(
            lexer.drain_tokens(),
            vec![
                Token::Text(text.to_string()),
                Token::EndTag {
                    name: last_start_tag.to_string()
                },
                Token::Eof,
            ],
            "state {initial_state} should emit the appropriate end tag"
        );
        assert!(
            lexer
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "unexpected-whitespace-after-end-tag-name"),
            "state {initial_state} should report whitespace recovery"
        );
    }
}

#[test]
fn default_html_lexer_keeps_mismatched_text_mode_end_tag_whitespace_literal() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("script_data").unwrap();
    lexer.set_last_start_tag("script");

    lexer.push("x</style >y</script>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("x</style >y".to_string()),
            Token::EndTag {
                name: "script".to_string()
            },
            Token::Eof,
        ]
    );
    assert!(!lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "unexpected-whitespace-after-end-tag-name"));
}

#[test]
fn default_html_lexer_recovers_seeded_text_mode_end_tags_with_attributes() {
    for (initial_state, last_start_tag, text, close) in [
        ("rcdata", "title", "Hello", "</title class=x>"),
        ("rawtext", "style", "body {}", "</style data-x>"),
        ("script_data", "script", "if (ready)", "</script ignored=1>"),
        (
            "script_data_escaped",
            "script",
            "<!-- ok -->",
            "</script type=text/javascript>",
        ),
    ] {
        let mut lexer = create_html_lexer().unwrap();
        lexer.set_initial_state(initial_state).unwrap();
        lexer.set_last_start_tag(last_start_tag);

        lexer.push(&format!("{text}{close}")).unwrap();
        lexer.finish().unwrap();

        assert_eq!(
            lexer.drain_tokens(),
            vec![
                Token::Text(text.to_string()),
                Token::EndTag {
                    name: last_start_tag.to_string()
                },
                Token::Eof,
            ],
            "state {initial_state} should emit the appropriate end tag"
        );
        assert!(
            lexer
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "end-tag-with-attributes"),
            "state {initial_state} should report attribute recovery"
        );
    }
}

#[test]
fn default_html_lexer_keeps_mismatched_text_mode_end_tag_attributes_literal() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("script_data").unwrap();
    lexer.set_last_start_tag("script");

    lexer.push("x</style class=x>y</script>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("x</style class=x>y".to_string()),
            Token::EndTag {
                name: "script".to_string()
            },
            Token::Eof,
        ]
    );
    assert!(!lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "end-tag-with-attributes"));
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
fn default_html_lexer_replaces_null_characters_in_script_escaped_substates() {
    let mut escaped = create_html_lexer().unwrap();
    escaped.set_initial_state("script_data_escaped").unwrap();
    escaped.set_last_start_tag("script");

    escaped.push("a\0-\0--\0></script>").unwrap();
    escaped.finish().unwrap();

    assert_eq!(
        escaped.drain_tokens(),
        vec![
            Token::Text("a\u{FFFD}-\u{FFFD}--\u{FFFD}>".to_string()),
            Token::EndTag {
                name: "script".to_string()
            },
            Token::Eof,
        ]
    );
    assert_eq!(
        escaped
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == "unexpected-null-character")
            .count(),
        3
    );

    let mut double_escaped = create_html_lexer().unwrap();
    double_escaped
        .set_initial_state("script_data_double_escaped")
        .unwrap();
    double_escaped.set_last_start_tag("script");

    double_escaped
        .push("a\0-\0--\0></script>x</script>")
        .unwrap();
    double_escaped.finish().unwrap();

    assert_eq!(
        double_escaped.drain_tokens(),
        vec![
            Token::Text("a\u{FFFD}-\u{FFFD}--\u{FFFD}></script>x".to_string()),
            Token::EndTag {
                name: "script".to_string()
            },
            Token::Eof,
        ]
    );
    assert_eq!(
        double_escaped
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == "unexpected-null-character")
            .count(),
        3
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
fn default_html_lexer_treats_form_feed_as_script_double_escape_delimiter() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("script_data").unwrap();
    lexer.set_last_start_tag("script");

    lexer
        .push("<!-- <script\u{000C}x </script\u{000C} y --></script>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("<!-- <script\u{000C}x </script\u{000C} y -->".to_string()),
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
fn default_html_lexer_supports_html4_symbol_named_character_references_in_data() {
    let tokens =
        lex_html("Greek: &Alpha;&beta;&Omega; Math: &sum;&ne;&le;&ge; Arrows: &larr;&rArr;")
            .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Greek: \u{0391}\u{03B2}\u{03A9} Math: \u{2211}\u{2260}\u{2264}\u{2265} Arrows: \u{2190}\u{21D2}".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_html4_symbol_named_character_references_in_attributes() {
    let tokens =
        lex_html("<span title=\"&ldquo;Venture&rdquo; &mdash; &trade;\" math=&radic; set=&sube;>")
            .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "span".to_string(),
                attributes: vec![
                    Attribute {
                        name: "title".to_string(),
                        value: "\u{201C}Venture\u{201D} \u{2014} \u{2122}".to_string(),
                    },
                    Attribute {
                        name: "math".to_string(),
                        value: "\u{221A}".to_string(),
                    },
                    Attribute {
                        name: "set".to_string(),
                        value: "\u{2286}".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_html4_symbol_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push("&OElig;&oelig; &euro; &spades;</title>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("\u{0152}\u{0153} \u{20AC} \u{2660}".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_remaining_html4_math_named_character_references() {
    let tokens = lex_html("Math symbols: &alefsym;&oline; &alefsymtail &olinebar").unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Math symbols: \u{2135}\u{203E} &alefsymtail &olinebar".to_string(),),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_remaining_html4_math_named_character_references_in_attributes() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<math alef=\"&alefsym;\" overline=&oline; literal=&alefsymtail>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::StartTag {
                name: "math".to_string(),
                attributes: vec![
                    Attribute {
                        name: "alef".to_string(),
                        value: "\u{2135}".to_string(),
                    },
                    Attribute {
                        name: "overline".to_string(),
                        value: "\u{203E}".to_string(),
                    },
                    Attribute {
                        name: "literal".to_string(),
                        value: "&alefsymtail".to_string(),
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
        0
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_remaining_html4_math_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer.push("&alefsym;&oline;</title>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("\u{2135}\u{203E}".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_spacing_named_character_references() {
    let tokens = lex_html(
        "Spacing:&Tab;&NewLine;&MediumSpace;&ThickSpace;&ThinSpace;&VeryThinSpace;&ZeroWidthSpace;&NegativeMediumSpace;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text(
                "Spacing:\t\n\u{205F}\u{205F}\u{200A}\u{2009}\u{200A}\u{200B}\u{200B}".to_string(),
            ),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_invisible_named_character_references_in_attributes() {
    let tokens = lex_html(
        "<a data=\"&NoBreak;&NonBreakingSpace;&ApplyFunction;&InvisibleTimes;&InvisibleComma;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "data".to_string(),
                    value: "\u{2060}\u{00A0}\u{2061}\u{2062}\u{2063}".to_string(),
                }],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_alias_punctuation_named_character_references() {
    let tokens = lex_html(
        "&OpenCurlyQuote;hi&CloseCurlyQuote; &OpenCurlyDoubleQuote;x&CloseCurlyDoubleQuote; &CenterDot;&VerticalBar;&DoubleVerticalBar;&LeftAngleBracket;&RightAngleBracket;&Cross;&SmallCircle;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![Token::Text("‘hi’ “x” ·∣∥⟨⟩⨯∘".to_string()), Token::Eof,]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_math_constant_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&DifferentialD;&CapitalDifferentialD;&DD;&dd;&ExponentialE;&ee;&ImaginaryI;&ii;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text(
                "\u{2146}\u{2145}\u{2145}\u{2146}\u{2147}\u{2147}\u{2148}\u{2148}".to_string()
            ),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_equality_and_tilde_named_character_references() {
    let tokens = lex_html(
        "Relations:&Equal;&EqualTilde;&Tilde;&TildeEqual;&TildeFullEqual;&TildeTilde;&NotEqual;&NotEqualTilde;&NotTilde;&NotTildeEqual;&NotTildeFullEqual;&NotTildeTilde;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Relations:⩵≂∼≃≅≈≠≂\u{0338}≁≄≇≉".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_congruence_named_character_references() {
    let tokens = lex_html(
        "Congruence:&Bumpeq;&Congruent;&DotEqual;&HumpDownHump;&HumpEqual;&NotVerticalBar;&VerticalLine;&bcong;&bsim;&bsime;&bump;&bumpE;&bumpe;&bumpeq;&circeq;&coloneq;&congdot;&doteq;&doteqdot;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Congruence:≎≡≐≎≏∤|≌∽⋍≎⪮≏≏≗≔⩭≐≑".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_equality_and_parallel_named_character_references_in_attributes(
) {
    let tokens = lex_html(
        "<math eq=\"&eqcirc;&eqcolon;&eqsim;&eqslantgtr;&eqslantless;&equals;&equest;&equivDD;&eqvparsl;\" parallel=\"&mid;&midast;&midcir;&npar;&nparallel;&nparsl;&npart;&par;&parallel;&parsim;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "math".to_string(),
                attributes: vec![
                    Attribute {
                        name: "eq".to_string(),
                        value: "≖≕≂⪖⪕=≟⩸⧥".to_string(),
                    },
                    Attribute {
                        name: "parallel".to_string(),
                        value: "∣*⫰∦∦⫽\u{20E5}∂\u{0338}∥∥⫳".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_similarity_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&parsl;&shortmid;&shortparallel;&simdot;&sime;&simeq;&simg;&simgE;&siml;&simlE;&simne;&simplus;&simrarr;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("⫽∣∥⩪≃≃⪞⪠⪝⪟≆⨤⥲".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_greater_less_named_character_references_in_attributes() {
    let tokens = lex_html(
        "<math cmp=\"&GreaterEqual;&GreaterFullEqual;&GreaterGreater;&GreaterLess;&GreaterSlantEqual;&GreaterTilde;\" inv=\"&LessEqualGreater;&LessFullEqual;&LessGreater;&LessLess;&LessSlantEqual;&LessTilde;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "math".to_string(),
                attributes: vec![
                    Attribute {
                        name: "cmp".to_string(),
                        value: "≥≧⪢≷⩾≳".to_string(),
                    },
                    Attribute {
                        name: "inv".to_string(),
                        value: "⋚≦≶⪡⩽≲".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_greater_less_comparison_named_character_references() {
    let tokens = lex_html(
        "GreaterLess:&GreaterEqualLess;&gl;&glE;&gla;&glj;&gnE;&gnap;&gnapprox;&gne;&gneq;&gneqq;&gnsim;&gtrapprox;&gtrarr;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("GreaterLess:⋛≷⪒⪥⪤≩⪊⪊⪈⪈≩⋧⪆⥸".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_greater_less_comparison_named_character_references_in_attributes(
) {
    let tokens = lex_html(
        "<math cmp=\"&gtrdot;&gtreqless;&gtreqqless;&gtrless;&gtrsim;&lessapprox;&lessdot;&lesseqgtr;&lesseqqgtr;&lessgtr;&lesssim;&lg;&lgE;&lnE;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "math".to_string(),
                attributes: vec![Attribute {
                    name: "cmp".to_string(),
                    value: "⋗⋛⪌≷≳⪅⋖⋚⪋≶≲≶⪑≨".to_string(),
                }],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_negated_greater_less_named_character_references(
) {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push("&lnap;&lnapprox;&lne;&lneq;&lneqq;&lnsim;&nGg;&nGt;&nGtv;&nLl;&nLt;&nLtv;</title>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("⪉⪉⪇⪇≨⋦⋙\u{0338}≫\u{20D2}≫\u{0338}⋘\u{0338}≪\u{20D2}≪\u{0338}".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_negated_relational_named_character_references()
{
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&NotGreater;&NotGreaterEqual;&NotGreaterFullEqual;&NotGreaterGreater;&NotGreaterLess;&NotGreaterSlantEqual;&NotGreaterTilde;&NotLess;&NotLessEqual;&NotLessGreater;&NotLessLess;&NotLessSlantEqual;&NotLessTilde;&NotNestedGreaterGreater;&NotNestedLessLess;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text(
                "≯≱≧\u{0338}≫\u{0338}≹⩾\u{0338}≵≮≰≸≪\u{0338}⩽\u{0338}≴⪢\u{0338}⪡\u{0338}"
                    .to_string()
            ),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_precedence_named_character_references() {
    let tokens = lex_html(
        "Precedence:&NotPrecedes;&NotPrecedesEqual;&NotPrecedesSlantEqual;&NotSucceeds;&NotSucceedsEqual;&NotSucceedsSlantEqual;&NotSucceedsTilde;&Precedes;&PrecedesEqual;&PrecedesSlantEqual;&PrecedesTilde;&Succeeds;&SucceedsEqual;&SucceedsSlantEqual;&SucceedsTilde;&curlyeqprec;&curlyeqsucc;&nprec;&npreceq;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text(
                "Precedence:⊀⪯\u{0338}⋠⊁⪰\u{0338}⋡≿\u{0338}≺⪯≼≾≻⪰≽≿⋞⋟⊀⪯\u{0338}".to_string()
            ),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_precedence_named_character_references_in_attributes() {
    let tokens = lex_html(
        "<math prec=\"&nsucc;&nsucceq;&pr;&prE;&prap;&prcue;&pre;&prec;&precapprox;&preccurlyeq;&preceq;&precnapprox;&precneqq;&precnsim;&precsim;&prnE;&prnap;&prnsim;&prsim;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "math".to_string(),
                attributes: vec![Attribute {
                    name: "prec".to_string(),
                    value: "⊁⪰\u{0338}≺⪳⪷≼⪯≺⪷≼⪯⪹⪵⋨≾⪵⪹⋨≾".to_string(),
                }],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_successor_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&sc;&scE;&scap;&sccue;&sce;&scnE;&scnap;&scnsim;&scsim;&succ;&succapprox;&succcurlyeq;&succeq;&succnapprox;&succneqq;&succnsim;&succsim;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("≻⪴⪸≽⪰⪶⪺⋩≿≻⪸≽⪰⪺⪶⋩≿".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_basic_arrow_named_character_references() {
    let tokens = lex_html(
        "Arrows:&LeftArrow;&RightArrow;&UpArrow;&DownArrow;&LeftRightArrow;&UpDownArrow;&DoubleLeftArrow;&DoubleRightArrow;&DoubleUpArrow;&DoubleDownArrow;&DoubleLeftRightArrow;&DoubleUpDownArrow;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![Token::Text("Arrows:←→↑↓↔↕⇐⇒⇑⇓⇔⇕".to_string()), Token::Eof,]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_extended_arrow_aliases() {
    let tokens = lex_html(
        "More arrows:&Darr;&Downarrow;&Larr;&Leftarrow;&Leftrightarrow;&Rarr;&Rightarrow;&Uarr;&Uparrow;&Updownarrow;&ShortDownArrow;&ShortLeftArrow;&ShortRightArrow;&ShortUpArrow;&LowerLeftArrow;&LowerRightArrow;&UpperLeftArrow;&UpperRightArrow;&DoubleLongLeftArrow;&DoubleLongLeftRightArrow;&DoubleLongRightArrow;&DownTeeArrow;&UpTeeArrow;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("More arrows:↡⇓↞⇐⇔↠⇒↟⇑⇕↓←→↑↙↘↖↗⟸⟺⟹↧↥".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_long_and_bar_arrow_named_character_references_in_attributes()
{
    let tokens = lex_html(
        "<a long=\"&LongLeftArrow;&LongRightArrow;&LongLeftRightArrow;&Longleftarrow;&Longrightarrow;&Longleftrightarrow;\" bars=\"&LeftArrowBar;&RightArrowBar;&UpArrowBar;&DownArrowBar;&LeftArrowRightArrow;&RightArrowLeftArrow;&UpArrowDownArrow;&DownArrowUpArrow;&LeftTeeArrow;&RightTeeArrow;&map;&Map;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "long".to_string(),
                        value: "⟵⟶⟷⟸⟹⟺".to_string(),
                    },
                    Attribute {
                        name: "bars".to_string(),
                        value: "⇤⇥⤒⤓⇆⇄⇅⇵↤↦↦⤅".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_hook_tail_and_loop_arrow_aliases_in_attributes() {
    let tokens = lex_html(
        "<a plain=\"&downarrow;&leftarrow;&leftrightarrow;&longleftarrow;&longleftrightarrow;&longrightarrow;\" hooks=\"&hookleftarrow;&hookrightarrow;&leftarrowtail;&rightarrowtail;&twoheadleftarrow;&twoheadrightarrow;\" loops=\"&curvearrowleft;&curvearrowright;&circlearrowleft;&circlearrowright;&looparrowleft;&looparrowright;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "plain".to_string(),
                        value: "↓←↔⟵⟷⟶".to_string(),
                    },
                    Attribute {
                        name: "hooks".to_string(),
                        value: "↩↪↢↣↞↠".to_string(),
                    },
                    Attribute {
                        name: "loops".to_string(),
                        value: "↶↷↺↻↫↬".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_vector_arrow_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&LeftVector;&RightVector;&LeftUpVector;&RightUpVector;&LeftDownVector;&RightDownVector;&DownLeftVector;&DownRightVector;&LeftVectorBar;&RightVectorBar;&LeftUpVectorBar;&RightUpVectorBar;&LeftDownVectorBar;&RightDownVectorBar;&DownLeftVectorBar;&DownRightVectorBar;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("↼⇀↿↾⇃⇂↽⇁⥒⥓⥘⥔⥙⥕⥖⥗".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_harpoon_negated_and_mapsto_arrows() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&leftharpoonup;&leftharpoondown;&rightharpoonup;&rightharpoondown;&upharpoonleft;&upharpoonright;&downharpoonleft;&downharpoonright;&leftleftarrows;&rightrightarrows;&downdownarrows;&upuparrows;&leftrightarrows;&rightleftarrows;&leftrightharpoons;&rightleftharpoons;&rightsquigarrow;&leftrightsquigarrow;&nleftarrow;&nrightarrow;&nleftrightarrow;&nLeftarrow;&nRightarrow;&nLeftrightarrow;&mapsto;&longmapsto;&mapstoleft;&mapstoup;&mapstodown;&nearrow;&searrow;&swarrow;&nwarrow;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("↼↽⇀⇁↿↾⇃⇂⇇⇉⇊⇈⇆⇄⇋⇌↝↭↚↛↮⇍⇏⇎↦⟼↤↥↧↗↘↙↖".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_greek_variant_named_character_references() {
    let tokens = lex_html(
        "Greek variants:&Gammad;&digamma;&Upsi;&upsi;&beth;&gimel;&daleth;&backepsilon;&bepsi;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Greek variants:Ϝϝϒυℶℷℸ϶϶".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_greek_variant_named_character_references_in_attributes() {
    let tokens = lex_html(
        "<i vars=\"&epsi;&epsiv;&varepsilon;&straightepsilon;&kappav;&varkappa;&phiv;&varphi;&straightphi;\" more=\"&rhov;&varrho;&sigmav;&varsigma;&thetav;&vartheta;&varpi;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "i".to_string(),
                attributes: vec![
                    Attribute {
                        name: "vars".to_string(),
                        value: "εϵϵϵϰϰϕϕϕ".to_string(),
                    },
                    Attribute {
                        name: "more".to_string(),
                        value: "ϱϱςςϑϑϖ".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_greek_variant_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push("&varepsilon;&varkappa;&vartheta;&varrho;&varsigma;&varphi;</title>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("ϵϰϑϱςϕ".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_set_and_logic_named_character_references() {
    let tokens = lex_html(
        "Sets:&Intersection;&Union;&SquareIntersection;&SquareUnion;&Wedge;&Vee;&And;&Or;&Not;&Cup;&Cap;&CupCap;&NotCupCap;&VerticalSeparator;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![Token::Text("Sets:⋂⋃⊓⊔⋀⋁⩓⩔⫬⋓⋒≍≭❘".to_string()), Token::Eof,]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_set_and_logic_named_character_references_in_attributes() {
    let tokens = lex_html(
        "<m membership=\"&Element;&NotElement;&ReverseElement;&NotReverseElement;&Exists;&NotExists;&SuchThat;&isinv;&isinE;&notinva;&notinvb;&notinvc;&niv;&notniva;&notnivb;&notnivc;\" subsets=\"&Subset;&Supset;&SubsetEqual;&NotSubset;&NotSubsetEqual;&subE;&supE;&nsubE;&nsupE;&subne;&supne;&subnE;&supnE;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "m".to_string(),
                attributes: vec![
                    Attribute {
                        name: "membership".to_string(),
                        value: "∈∉∋∌∃∄∋∈⋹∉⋷⋶∋∌⋾⋽".to_string(),
                    },
                    Attribute {
                        name: "subsets".to_string(),
                        value: "⋐⋑⊆⊂⃒⊈⫅⫆⫅̸⫆̸⊊⊋⫋⫌".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_set_and_logic_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&emptyset;&emptyv;&varnothing;&setminus;&smallsetminus;&sqcap;&sqcup;&sqsub;&sqsup;&sqsube;&sqsupe;&cuvee;&cuwed;&xcap;&xcup;&xvee;&xwedge;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("∅∅∅∖∖⊓⊔⊏⊐⊑⊒⋎⋏⋂⋃⋁⋀".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_operator_named_character_references() {
    let tokens = lex_html(
        "Operators:&CircleDot;&CircleMinus;&CirclePlus;&CircleTimes;&ContourIntegral;&DoubleContourIntegral;&Integral;&Product;&Coproduct;&Sum;&Sqrt;&Proportional;&Therefore;&Because;&VerticalTilde;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Operators:⊙⊖⊕⊗∮∯∫∏∐∑√∝∴∵≀".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_operator_and_shape_named_character_references_in_attributes()
{
    let tokens = lex_html(
        "<g ops=\"&ominus;&odot;&osol;&ocir;&oast;&odash;&pluscir;&timesb;&sdotb;&minusb;&boxplus;&boxminus;&boxtimes;&dotsquare;&compfn;\" shapes=\"&FilledSmallSquare;&EmptySmallSquare;&EmptyVerySmallSquare;&FilledVerySmallSquare;&Square;&SquareSubset;&SquareSubsetEqual;&SquareSuperset;&SquareSupersetEqual;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "g".to_string(),
                attributes: vec![
                    Attribute {
                        name: "ops".to_string(),
                        value: "⊖⊙⊘⊚⊛⊝⨢⊠⊡⊟⊞⊟⊠⊡∘".to_string(),
                    },
                    Attribute {
                        name: "shapes".to_string(),
                        value: "◼◻▫▪□⊏⊑⊐⊒".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_shape_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&squ;&square;&squf;&squarf;&blacksquare;&lozenge;&lozf;&blacklozenge;&diamond;&diam;&diamondsuit;&malt;&maltese;&starf;&bigstar;&star;&phone;&female;&male;&spadesuit;&clubsuit;&heartsuit;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("□□▪▪▪◊⧫⧫⋄⋄♦✠✠★★☆☎♀♂♠♣♥".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_box_drawing_named_character_references() {
    let tokens = lex_html(
        "Boxes:&boxDL;&boxDR;&boxDl;&boxDr;&boxH;&boxHD;&boxHU;&boxHd;&boxHu;&boxUL;&boxUR;&boxUl;&boxUr;&boxV;&boxVH;&boxVL;&boxVR;&boxVh;&boxVl;&boxVr;&boxbox;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Boxes:╗╔╖╓═╦╩╤╧╝╚╜╙║╬╣╠╫╢╟⧉".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_box_drawing_named_character_references_in_attributes() {
    let tokens = lex_html(
        "<box double=\"&boxDL;&boxDR;&boxDl;&boxDr;&boxH;&boxHD;&boxHU;&boxHd;&boxHu;&boxUL;&boxUR;&boxUl;&boxUr;\" mixed=\"&boxV;&boxVH;&boxVL;&boxVR;&boxVh;&boxVl;&boxVr;&boxbox;&boxdL;&boxdR;&boxdl;&boxdr;&boxh;&boxhD;&boxhU;\" light=\"&boxhd;&boxhu;&boxuL;&boxuR;&boxul;&boxur;&boxv;&boxvH;&boxvL;&boxvR;&boxvh;&boxvl;&boxvr;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "box".to_string(),
                attributes: vec![
                    Attribute {
                        name: "double".to_string(),
                        value: "╗╔╖╓═╦╩╤╧╝╚╜╙".to_string(),
                    },
                    Attribute {
                        name: "mixed".to_string(),
                        value: "║╬╣╠╫╢╟⧉╕╒┐┌─╥╨".to_string(),
                    },
                    Attribute {
                        name: "light".to_string(),
                        value: "┬┴╛╘┘└│╪╡╞┼┤├".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_box_drawing_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&boxdL;&boxdR;&boxdl;&boxdr;&boxh;&boxhD;&boxhU;&boxhd;&boxhu;&boxuL;&boxuR;&boxul;&boxur;&boxv;&boxvH;&boxvL;&boxvR;&boxvh;&boxvl;&boxvr;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("╕╒┐┌─╥╨┬┴╛╘┘└│╪╡╞┼┤├".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_angle_named_character_references() {
    let tokens = lex_html(
        "Angles:&angle;&angmsd;&angsph;&angrt;&angrtvb;&angrtvbd;&angst;&angzarr;&measuredangle;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![Token::Text("Angles:∠∡∢∟⊾⦝Å⍼∡".to_string()), Token::Eof,]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_fence_named_character_references_in_attributes() {
    let tokens = lex_html(
        "<f angles=\"&lang;&rang;&langle;&rangle;&LeftAngleBracket;&RightAngleBracket;\" fences=\"&LeftCeiling;&RightCeiling;&LeftFloor;&RightFloor;&LeftDoubleBracket;&RightDoubleBracket;&lobrk;&robrk;&lbrack;&rbrack;&lbrace;&rbrace;&lpar;&rpar;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "f".to_string(),
                attributes: vec![
                    Attribute {
                        name: "angles".to_string(),
                        value: "⟨⟩⟨⟩⟨⟩".to_string(),
                    },
                    Attribute {
                        name: "fences".to_string(),
                        value: "⌈⌉⌊⌋⟦⟧⟦⟧[]{}()".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_triangle_and_corner_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&LeftTriangle;&RightTriangle;&triangleleft;&triangleright;&blacktriangleleft;&blacktriangleright;&ulcorner;&urcorner;&llcorner;&lrcorner;&OverBrace;&UnderBrace;&OverBracket;&UnderBracket;&OverParenthesis;&UnderParenthesis;&OverBar;&UnderBar;&bbrk;&bbrktbrk;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("⊲⊳◃▹◂▸⌜⌝⌞⌟⏞⏟⎴⎵⏜⏝‾_⎵⎶".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_latin_extended_named_character_references() {
    let tokens = lex_html(
        "Latin extended:&Amacr;&abreve;&Aogon;&aogon;&Cacute;&ccaron;&Dcaron;&dcaron;&Emacr;&eogon;&Gbreve;&gcirc;&Idot;&inodot;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Latin extended:ĀăĄąĆčĎďĒęĞĝİı".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_latin_extended_named_character_references_in_attributes() {
    let tokens = lex_html(
        "<p upper=\"&Hcirc;&Itilde;&Jcirc;&Kcedil;&Lacute;&Lcaron;&Lcedil;&Lmidot;&Nacute;&Ncaron;&Ncedil;\" lower=\"&hcirc;&itilde;&jcirc;&kcedil;&lacute;&lcaron;&lcedil;&lmidot;&nacute;&ncaron;&ncedil;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "p".to_string(),
                attributes: vec![
                    Attribute {
                        name: "upper".to_string(),
                        value: "ĤĨĴĶĹĽĻĿŃŇŅ".to_string(),
                    },
                    Attribute {
                        name: "lower".to_string(),
                        value: "ĥĩĵķĺľļŀńňņ".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_latin_extended_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&Omacr;&omacr;&Racute;&rcaron;&Sacute;&scedil;&Scirc;&scirc;&Tcaron;&tcedil;&Ubreve;&umacr;&Uogon;&uring;&Utilde;&wcirc;&Ycirc;&ycirc;&Zacute;&zcaron;&Zdot;&zdot;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("ŌōŔřŚşŜŝŤţŬūŲůŨŵŶŷŹžŻż".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_open_face_named_character_references() {
    let tokens = lex_html(
        "Open face:&Aopf;&Bopf;&Copf;&Dopf;&Eopf;&Fopf;&Gopf;&Hopf;&Iopf;&Jopf;&Kopf;&Lopf;&Mopf;&Nopf;&Oopf;&Popf;&Qopf;&Ropf;&Sopf;&Topf;&Uopf;&Vopf;&Wopf;&Xopf;&Yopf;&Zopf;&aopf;&bopf;&copf;&dopf;&eopf;&fopf;&gopf;&hopf;&iopf;&jopf;&kopf;&lopf;&mopf;&nopf;&oopf;&popf;&qopf;&ropf;&sopf;&topf;&uopf;&vopf;&wopf;&xopf;&yopf;&zopf;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text(
                "Open face:𝔸𝔹ℂ𝔻𝔼𝔽𝔾ℍ𝕀𝕁𝕂𝕃𝕄ℕ𝕆ℙℚℝ𝕊𝕋𝕌𝕍𝕎𝕏𝕐ℤ𝕒𝕓𝕔𝕕𝕖𝕗𝕘𝕙𝕚𝕛𝕜𝕝𝕞𝕟𝕠𝕡𝕢𝕣𝕤𝕥𝕦𝕧𝕨𝕩𝕪𝕫".to_string()
            ),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_script_named_character_references_in_attributes() {
    let tokens = lex_html(
        "<math upper=\"&Ascr;&Bscr;&Cscr;&Dscr;&Escr;&Fscr;&Gscr;&Hscr;&Iscr;&Jscr;&Kscr;&Lscr;&Mscr;&Nscr;&Oscr;&Pscr;&Qscr;&Rscr;&Sscr;&Tscr;&Uscr;&Vscr;&Wscr;&Xscr;&Yscr;&Zscr;\" lower=\"&ascr;&bscr;&cscr;&dscr;&escr;&fscr;&gscr;&hscr;&iscr;&jscr;&kscr;&lscr;&mscr;&nscr;&oscr;&pscr;&qscr;&rscr;&sscr;&tscr;&uscr;&vscr;&wscr;&xscr;&yscr;&zscr;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "math".to_string(),
                attributes: vec![
                    Attribute {
                        name: "upper".to_string(),
                        value: "𝒜ℬ𝒞𝒟ℰℱ𝒢ℋℐ𝒥𝒦ℒℳ𝒩𝒪𝒫𝒬ℛ𝒮𝒯𝒰𝒱𝒲𝒳𝒴𝒵".to_string(),
                    },
                    Attribute {
                        name: "lower".to_string(),
                        value: "𝒶𝒷𝒸𝒹ℯ𝒻ℊ𝒽𝒾𝒿𝓀𝓁𝓂𝓃ℴ𝓅𝓆𝓇𝓈𝓉𝓊𝓋𝓌𝓍𝓎𝓏".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_fraktur_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&Afr;&Bfr;&Cfr;&Dfr;&Efr;&Ffr;&Gfr;&Hfr;&Ifr;&Jfr;&Kfr;&Lfr;&Mfr;&Nfr;&Ofr;&Pfr;&Qfr;&Rfr;&Sfr;&Tfr;&Ufr;&Vfr;&Wfr;&Xfr;&Yfr;&Zfr;&afr;&bfr;&cfr;&dfr;&efr;&ffr;&gfr;&hfr;&ifr;&jfr;&kfr;&lfr;&mfr;&nfr;&ofr;&pfr;&qfr;&rfr;&sfr;&tfr;&ufr;&vfr;&wfr;&xfr;&yfr;&zfr;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("𝔄𝔅ℭ𝔇𝔈𝔉𝔊ℌℑ𝔍𝔎𝔏𝔐𝔑𝔒𝔓𝔔ℜ𝔖𝔗𝔘𝔙𝔚𝔛𝔜ℨ𝔞𝔟𝔠𝔡𝔢𝔣𝔤𝔥𝔦𝔧𝔨𝔩𝔪𝔫𝔬𝔭𝔮𝔯𝔰𝔱𝔲𝔳𝔴𝔵𝔶𝔷".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_core_cyrillic_named_character_references() {
    let tokens = lex_html(
        "Cyrillic:&Acy;&Bcy;&Vcy;&Gcy;&Dcy;&IEcy;&IOcy;&ZHcy;&Zcy;&Icy;&Jcy;&Kcy;&Lcy;&Mcy;&Ncy;&Ocy;&Pcy;&Rcy;&Scy;&Tcy;&Ucy;&Fcy;&KHcy;&TScy;&CHcy;&SHcy;&SHCHcy;&HARDcy;&Ycy;&SOFTcy;&Ecy;&YUcy;&YAcy;&acy;&bcy;&vcy;&gcy;&dcy;&iecy;&iocy;&zhcy;&zcy;&icy;&jcy;&kcy;&lcy;&mcy;&ncy;&ocy;&pcy;&rcy;&scy;&tcy;&ucy;&fcy;&khcy;&tscy;&chcy;&shcy;&shchcy;&hardcy;&ycy;&softcy;&ecy;&yucy;&yacy;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text(
                "Cyrillic:АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯабвгдеёжзийклмнопрстуфхцчшщъыьэюя"
                    .to_string()
            ),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_extended_cyrillic_named_character_references_in_attributes() {
    let tokens = lex_html(
        "<span upper=\"&DJcy;&DScy;&DZcy;&GJcy;&Iukcy;&Jsercy;&Jukcy;&KJcy;&LJcy;&NJcy;&TSHcy;&Ubrcy;&YIcy;\" lower=\"&djcy;&dscy;&dzcy;&gjcy;&iukcy;&jsercy;&jukcy;&kjcy;&ljcy;&njcy;&tshcy;&ubrcy;&yicy;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "span".to_string(),
                attributes: vec![
                    Attribute {
                        name: "upper".to_string(),
                        value: "ЂЅЏЃІЈЄЌЉЊЋЎЇ".to_string(),
                    },
                    Attribute {
                        name: "lower".to_string(),
                        value: "ђѕџѓіјєќљњћўї".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_cyrillic_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&Acy;&IEcy;&ZHcy;&SHCHcy;&SOFTcy;&YAcy;&acy;&iecy;&zhcy;&shchcy;&softcy;&yacy;&DJcy;&TSHcy;&Ubrcy;&YIcy;&djcy;&tshcy;&ubrcy;&yicy;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("АЕЖЩЬЯаежщьяЂЋЎЇђћўї".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_remaining_vector_named_character_references() {
    let tokens = lex_html(
        "Vectors:&DownLeftRightVector;&DownLeftTeeVector;&DownRightTeeVector;&LeftDownTeeVector;&LeftRightVector;&LeftTeeVector;&LeftUpDownVector;&LeftUpTeeVector;&RightDownTeeVector;&RightTeeVector;&RightUpDownVector;&RightUpTeeVector;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![Token::Text("Vectors:⥐⥞⥟⥡⥎⥚⥑⥠⥝⥛⥏⥜".to_string()), Token::Eof]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_remaining_arrow_named_character_references_in_attributes() {
    let tokens = lex_html(
        "<nav upper=\"&Lleftarrow;&Rrightarrow;&RBarr;&Rarrtl;&Uarrocir;&neArr;&nwArr;&seArr;&swArr;&xhArr;&xlArr;&xrArr;\" lower=\"&lAarr;&rAarr;&lbarr;&rbarr;&bkarow;&dbkarow;&drbkarow;&nearr;&nwarr;&searr;&swarr;&zigrarr;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "nav".to_string(),
                attributes: vec![
                    Attribute {
                        name: "upper".to_string(),
                        value: "⇚⇛⤐⤖⥉⇗⇖⇘⇙⟺⟸⟹".to_string(),
                    },
                    Attribute {
                        name: "lower".to_string(),
                        value: "⇚⇛⤌⤍⤍⤏⤐↗↖↘↙⇝".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_remaining_harpoon_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&dHar;&lHar;&rHar;&uHar;&duhar;&lrhar;&rlhar;&lharu;&lhard;&rharu;&rhard;&uharl;&uharr;&dharl;&dharr;&nrarrc;&nrarrw;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("⥥⥢⥤⥣⥯⇋⇌↼↽⇀⇁↿↾⇃⇂⤳̸↝̸".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_remaining_cap_cup_named_character_references() {
    let tokens = lex_html(
        "Sets:&bigcap;&bigcup;&bigsqcup;&UnionPlus;&capand;&capbrcup;&capcap;&capcup;&capdot;&caps;&ccaps;&ccups;&ccupssm;&cupbrcap;&cupcap;&cupcup;&cupdot;&cupor;&cups;&ncap;&ncup;&xsqcup;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Sets:⋂⋃⨆⊎⩄⩉⩋⩇⩀∩︀⩍⩌⩐⩈⩆⩊⊍⩅∪︀⩃⩂⨆".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_remaining_subset_superset_named_character_references_in_attributes(
) {
    let tokens = lex_html(
        "<set sub=\"&Sub;&csub;&csube;&subdot;&subedot;&submult;&subplus;&subset;&subseteq;&subseteqq;&subsetneq;&subsetneqq;&subsim;&subsub;&subsup;\" sup=\"&Sup;&Superset;&SupersetEqual;&csup;&csupe;&supdot;&supdsub;&supedot;&suphsol;&suphsub;&supmult;&supplus;&supset;&supseteq;&supseteqq;&supsetneq;&supsetneqq;&supsim;&supsub;&supsup;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "set".to_string(),
                attributes: vec![
                    Attribute {
                        name: "sub".to_string(),
                        value: "⋐⫏⫑⪽⫃⫁⪿⊂⊆⫅⊊⫋⫇⫕⫓".to_string(),
                    },
                    Attribute {
                        name: "sup".to_string(),
                        value: "⋑⊃⊇⫐⫒⪾⫘⫄⟉⫗⫂⫀⊃⊇⫆⊋⫌⫈⫔⫖".to_string(),
                    },
                ],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_remaining_negated_set_named_character_references(
) {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&NotSquareSubset;&NotSquareSubsetEqual;&NotSquareSuperset;&NotSquareSupersetEqual;&NotSuperset;&NotSupersetEqual;&nsubset;&nsubseteq;&nsubseteqq;&nsupset;&nsupseteq;&nsupseteqq;&varsubsetneq;&varsubsetneqq;&varsupsetneq;&varsupsetneqq;&vnsub;&vnsup;&vsubnE;&vsubne;&vsupnE;&vsupne;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("⊏̸⋢⊐̸⋣⊃⃒⊉⊂⃒⊈⫅̸⊃⃒⊉⫆̸⊊︀⫋︀⊋︀⫌︀⊂⃒⊃⃒⫋︀⊊︀⫌︀⊋︀".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_remaining_square_set_named_character_references() {
    let tokens = lex_html(
        "Squares:&sqcaps;&sqcups;&sqsubset;&sqsubseteq;&sqsupset;&sqsupseteq;&nsqsube;&nsqsupe;&setmn;&ssetmn;&bsolhsub;&suphsol;&lsqb;&rsqb;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Squares:⊓︀⊔︀⊏⊑⊐⊒⋢⋣∖∖⟈⟉[]".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_remaining_integral_named_character_references() {
    let tokens = lex_html(
        "Integrals:&Conint;&Cconint;&ClockwiseContourIntegral;&CounterClockwiseContourIntegral;&Int;&awconint;&awint;&cirfnint;&conint;&cwconint;&cwint;&fpartint;&iiiint;&iiint;&intlarhk;&npolint;&oint;&pointint;&qint;&quatint;&rppolint;&scpolint;&tint;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Text("Integrals:∯∰∲∳∬∳⨑⨐∮∲∱⨍⨌∭⨗⨔∮⨕⨌⨖⨒⨓∭".to_string()),
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_remaining_operator_named_character_references_in_attributes()
{
    let tokens = lex_html(
        "<ops value=\"&MinusPlus;&Otimes;&PlusMinus;&bigcirc;&bigodot;&bigoplus;&bigotimes;&biguplus;&circledast;&circledcirc;&circleddash;&coprod;&divideontimes;&dotminus;&dotplus;&eplus;&loplus;&lotimes;&ltimes;&minusd;&minusdu;&mnplus;&otimesas;&plus;&plusacir;&plusb;&plusdo;&plusdu;&pluse;&plussim;&plustwo;&roplus;&rotimes;&rtimes;&timesbar;&timesd;&triminus;&triplus;&uplus;&xcirc;&xodot;&xoplus;&xuplus;\">",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::StartTag {
                name: "ops".to_string(),
                attributes: vec![Attribute {
                    name: "value".to_string(),
                    value: "∓⨷±◯⨀⨁⨂⨄⊛⊚⊝∐⋇∸∔⩱⨭⨴⋉∸⨪∓⨶+⨣⊞∔⨥⩲⨦⨧⨮⨵⋊⨱⨰⨺⨹⊎◯⨀⨁⨄".to_string(),
                }],
                self_closing: false,
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_seeded_rcdata_whatwg_remaining_dot_named_character_references() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer
        .push(
            "&DDotrahd;&Dot;&DotDot;&DoubleDot;&TripleDot;&centerdot;&ctdot;&ddotseq;&dot;&dtdot;&eDDot;&eDot;&efDot;&egsdot;&elsdot;&erDot;&esdot;&fallingdotseq;&gesdot;&gesdoto;&gesdotol;&gtdot;&isindot;&lesdot;&lesdoto;&lesdotor;&ltdot;&mDDot;&ncongdot;&nedot;&notindot;&risingdotseq;&sdote;&tdot;&tridot;&utdot;</title>",
        )
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("⤑¨⃜¨⃛·⋯⩷˙⋱⩷≑≒⪘⪗≓≐≒⪀⪂⪄⋗⋵⩿⪁⪃⋖∺⩭̸≐̸⋵̸≓⩦⃛◬⋰".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof,
        ]
    );
}

#[test]
fn default_html_lexer_supports_whatwg_remaining_operator_misc_named_character_references() {
    let tokens = lex_html(
        "Misc:&Mellintrf;&infintie;&intcal;&integers;&intercal;&intprod;&iprod;&leftthreetimes;&rightthreetimes;&elinters;",
    )
    .unwrap();

    assert_eq!(
        tokens,
        vec![Token::Text("Misc:ℳ⧝⊺ℤ⊺⨼⨼⋋⋌⏧".to_string()), Token::Eof]
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
fn default_html_lexer_restricts_semicolonless_named_references_to_legacy_names() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("Text &notin &trade <a value=&notin legacy=&Agrave data=&copy/>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Text \u{00AC}in &trade ".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "value".to_string(),
                        value: "&notin".to_string(),
                    },
                    Attribute {
                        name: "legacy".to_string(),
                        value: "\u{00C0}".to_string(),
                    },
                    Attribute {
                        name: "data".to_string(),
                        value: "\u{00A9}".to_string(),
                    },
                ],
                self_closing: true,
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
fn default_html_lexer_uses_longest_named_character_reference_prefix_in_text() {
    let mut lexer = create_html_lexer().unwrap();

    lexer.push("Text &notit; &copycat &sumtotal").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Text \u{00AC}it; \u{00A9}cat &sumtotal".to_string()),
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
        2
    );
}

#[test]
fn default_html_lexer_preserves_ambiguous_ampersands_in_attributes() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<a title=\"&notit; &copycat &copy\" rel=&notin data=&notin;>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "title".to_string(),
                        value: "&notit; &copycat \u{00A9}".to_string(),
                    },
                    Attribute {
                        name: "rel".to_string(),
                        value: "&notin".to_string(),
                    },
                    Attribute {
                        name: "data".to_string(),
                        value: "\u{2209}".to_string(),
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
        1
    );
}

#[test]
fn default_html_lexer_uses_longest_named_character_reference_prefix_in_rcdata() {
    let mut lexer = create_html_lexer().unwrap();
    lexer.set_initial_state("rcdata").unwrap();
    lexer.set_last_start_tag("title");

    lexer.push("&notit; &copycat</title>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("\u{00AC}it; \u{00A9}cat".to_string()),
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
        2
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
fn default_html_lexer_reports_invalid_numeric_character_references() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("<a data='&#0; &#xD800; &#x110000; &#xFDD0; &#x80;'>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "data".to_string(),
                    value: "\u{FFFD} \u{FFFD} \u{FFFD} \u{FDD0} \u{20AC}".to_string(),
                }],
                self_closing: false,
            },
            Token::Eof,
        ]
    );

    let diagnostics = lexer
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        diagnostics,
        vec![
            "null-character-reference",
            "surrogate-character-reference",
            "character-reference-outside-unicode-range",
            "noncharacter-character-reference",
            "control-character-reference",
        ]
    );
}

#[test]
fn default_html_lexer_reports_numeric_character_references_without_digits() {
    let mut lexer = create_html_lexer().unwrap();

    lexer
        .push("Bad &#; &#x; <a title='&#x;' data=&#;>")
        .unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("Bad &#; &#x; ".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![
                    Attribute {
                        name: "title".to_string(),
                        value: "&#x;".to_string(),
                    },
                    Attribute {
                        name: "data".to_string(),
                        value: "&#;".to_string(),
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
                diagnostic.code == "absence-of-digits-in-numeric-character-reference"
            })
            .count(),
        4
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

#[test]
fn parser_facing_context_maps_rcdata_elements() {
    let context = HtmlLexContext::for_element_text("TITLE").unwrap();

    assert_eq!(context.initial_state, HtmlTokenizerState::Rcdata);
    assert_eq!(context.last_start_tag.as_deref(), Some("title"));
    assert_eq!(
        lex_html_fragment("Tom &amp; Jerry</title>", &context).unwrap(),
        vec![
            Token::Text("Tom & Jerry".to_string()),
            Token::EndTag {
                name: "title".to_string()
            },
            Token::Eof
        ]
    );
}

#[test]
fn parser_facing_context_maps_rawtext_elements() {
    let context = HtmlLexContext::for_element_text("style").unwrap();

    assert_eq!(context.initial_state, HtmlTokenizerState::Rawtext);
    assert_eq!(context.last_start_tag.as_deref(), Some("style"));
    assert_eq!(
        lex_html_fragment("a < b &amp; c</style>", &context).unwrap(),
        vec![
            Token::Text("a ".to_string()),
            Token::Text("< b &amp; c".to_string()),
            Token::EndTag {
                name: "style".to_string()
            },
            Token::Eof
        ]
    );
}

#[test]
fn parser_facing_context_maps_noscript_based_on_scripting_mode() {
    let enabled =
        HtmlLexContext::for_element_text_with_scripting("noscript", HtmlScriptingMode::Enabled)
            .unwrap();

    assert_eq!(enabled.initial_state, HtmlTokenizerState::Rawtext);
    assert_eq!(enabled.last_start_tag.as_deref(), Some("noscript"));
    assert_eq!(
        lex_html_fragment("<p>&amp;</p></noscript>", &enabled).unwrap(),
        vec![
            Token::Text("<p>&amp;".to_string()),
            Token::Text("</p>".to_string()),
            Token::EndTag {
                name: "noscript".to_string()
            },
            Token::Eof
        ]
    );

    assert_eq!(
        HtmlLexContext::for_element_text_with_scripting("noscript", HtmlScriptingMode::Disabled),
        None
    );
}

#[test]
fn parser_facing_context_maps_script_and_plaintext_elements() {
    let script = HtmlLexContext::for_element_text("script").unwrap();
    assert_eq!(script.initial_state, HtmlTokenizerState::ScriptData);
    assert_eq!(script.last_start_tag.as_deref(), Some("script"));
    assert_eq!(
        lex_html_fragment("if (a < b) alert('&amp;');</script>", &script).unwrap(),
        vec![
            Token::Text("if (a < b) alert('&amp;');".to_string()),
            Token::EndTag {
                name: "script".to_string()
            },
            Token::Eof
        ]
    );

    let plaintext = HtmlLexContext::for_element_text("plaintext").unwrap();
    assert_eq!(plaintext.initial_state, HtmlTokenizerState::Plaintext);
    assert_eq!(plaintext.last_start_tag, None);
    assert_eq!(
        lex_html_fragment("<b>&amp;</b>", &plaintext).unwrap(),
        vec![Token::Text("<b>&amp;</b>".to_string()), Token::Eof]
    );
}

#[test]
fn parser_facing_context_leaves_normal_elements_in_data_state() {
    assert_eq!(HtmlLexContext::for_element_text("p"), None);
    assert!(HtmlLexContext::data().is_data());
    assert_eq!(
        HtmlLexContext::data().initial_state.as_machine_state(),
        "data"
    );
    let cdata = HtmlLexContext::cdata_section();
    assert_eq!(cdata.initial_state, HtmlTokenizerState::CdataSection);
    assert_eq!(cdata.last_start_tag, None);
    assert_eq!(
        lex_html_fragment("<svg:title>&amp;</svg:title>]]><p>x</p>", &cdata).unwrap(),
        vec![
            Token::Text("<svg:title>&amp;</svg:title>".to_string()),
            Token::StartTag {
                name: "p".to_string(),
                attributes: Vec::new(),
                self_closing: false
            },
            Token::Text("x".to_string()),
            Token::EndTag {
                name: "p".to_string()
            },
            Token::Eof
        ]
    );
    assert_eq!(
        HtmlTokenizerState::ScriptDataDoubleEscapedLessThanSign.as_machine_state(),
        "script_data_double_escaped_less_than_sign"
    );
}

#[test]
fn parser_facing_context_can_reconfigure_an_existing_lexer() {
    let mut lexer = create_html_lexer().unwrap();

    apply_html_lex_context(
        &mut lexer,
        &HtmlLexContext::for_element_text("script").unwrap(),
    )
    .unwrap();
    lexer.push("if (a < b)</script>").unwrap();
    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::Text("if (a < b)".to_string()),
            Token::EndTag {
                name: "script".to_string()
            }
        ]
    );

    apply_html_lex_context(&mut lexer, &HtmlLexContext::data()).unwrap();
    assert_eq!(lexer.current_state(), "data");
    lexer.push("<p>after</p>").unwrap();
    assert_eq!(
        lexer.drain_tokens(),
        vec![
            Token::StartTag {
                name: "p".to_string(),
                attributes: Vec::new(),
                self_closing: false
            },
            Token::Text("after".to_string()),
            Token::EndTag {
                name: "p".to_string()
            }
        ]
    );

    apply_html_lex_context(&mut lexer, &HtmlLexContext::cdata_section()).unwrap();
    assert_eq!(lexer.current_state(), "cdata_section");
    lexer.push("<literal>&amp;]]>").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![Token::Text("<literal>&amp;".to_string()), Token::Eof]
    );
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
