use std::collections::HashSet;

use state_machine::{EffectfulMatcher, EffectfulStateMachine, EffectfulTransition};
use state_machine_tokenizer::{Attribute, Token, Tokenizer, TokenizerError};

#[test]
fn tokenizer_rejects_unknown_actions() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data"]),
            HashSet::new(),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Any, "data")
                    .with_effects(&["host_callback()"]),
            ],
            "data".to_string(),
            HashSet::new(),
        )
        .unwrap(),
    );

    let error = tokenizer.push("x").unwrap_err();

    assert_eq!(
        error,
        TokenizerError::UnknownAction("host_callback()".to_string())
    );
}

#[test]
fn tokenizer_bounds_non_consuming_transition_loops() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data"]),
            set(&["x"]),
            vec![EffectfulTransition::new("data", EffectfulMatcher::Any, "data").consuming(false)],
            "data".to_string(),
            HashSet::new(),
        )
        .unwrap(),
    )
    .with_max_steps_per_input(3);

    let error = tokenizer.push("x").unwrap_err();

    assert!(matches!(
        error,
        TokenizerError::StepLimitExceeded { limit: 3, .. }
    ));
}

#[test]
fn tokenizer_builds_start_tag_attributes_and_self_closing_markers() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&[
                "data",
                "tag_open",
                "before_attr",
                "attr_name",
                "attr_value",
                "before_close",
                "done",
            ]),
            set(&["<", "A", " ", "H", "=", "X", "/", ">"]),
            vec![
                EffectfulTransition::new(
                    "data",
                    EffectfulMatcher::Event("<".to_string()),
                    "tag_open",
                ),
                EffectfulTransition::new(
                    "tag_open",
                    EffectfulMatcher::Event("A".to_string()),
                    "before_attr",
                )
                .with_effects(&["create_start_tag", "append_tag_name(current_lowercase)"]),
                EffectfulTransition::new(
                    "before_attr",
                    EffectfulMatcher::Event(" ".to_string()),
                    "attr_name",
                ),
                EffectfulTransition::new(
                    "attr_name",
                    EffectfulMatcher::Event("H".to_string()),
                    "attr_value",
                )
                .with_effects(&[
                    "start_attribute",
                    "append_attribute_name(current_lowercase)",
                    "append_attribute_name(ref)",
                ]),
                EffectfulTransition::new(
                    "attr_value",
                    EffectfulMatcher::Event("=".to_string()),
                    "attr_value",
                ),
                EffectfulTransition::new(
                    "attr_value",
                    EffectfulMatcher::Event("X".to_string()),
                    "before_close",
                )
                .with_effects(&["append_attribute_value(current)"]),
                EffectfulTransition::new(
                    "before_close",
                    EffectfulMatcher::Event("/".to_string()),
                    "before_close",
                )
                .with_effects(&[
                    "append_attribute_value(y)",
                    "commit_attribute",
                    "mark_self_closing",
                ]),
                EffectfulTransition::new(
                    "before_close",
                    EffectfulMatcher::Event(">".to_string()),
                    "done",
                )
                .with_effects(&["emit_current_token"]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("<A H=X/>").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::StartTag {
            name: "a".to_string(),
            attributes: vec![Attribute {
                name: "href".to_string(),
                value: "Xy".to_string(),
            }],
            self_closing: true,
        }]
    );
}

#[test]
fn tokenizer_can_drop_duplicate_attributes_when_requested() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "tag_open", "attr", "done"]),
            set(&["<", "A", "H", "1", "2", ">", " "]),
            vec![
                EffectfulTransition::new(
                    "data",
                    EffectfulMatcher::Event("<".to_string()),
                    "tag_open",
                ),
                EffectfulTransition::new(
                    "tag_open",
                    EffectfulMatcher::Event("A".to_string()),
                    "attr",
                )
                .with_effects(&["create_start_tag", "append_tag_name(current_lowercase)"]),
                EffectfulTransition::new("attr", EffectfulMatcher::Event("H".to_string()), "attr")
                    .with_effects(&[
                        "start_attribute",
                        "append_attribute_name(href)",
                        "append_attribute_value(1)",
                        "commit_attribute_dedup",
                    ]),
                EffectfulTransition::new("attr", EffectfulMatcher::Event("1".to_string()), "attr")
                    .with_effects(&[
                        "start_attribute",
                        "append_attribute_name(href)",
                        "append_attribute_value(2)",
                        "commit_attribute_dedup",
                    ]),
                EffectfulTransition::new("attr", EffectfulMatcher::Event(">".to_string()), "done")
                    .with_effects(&["emit_current_token"]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("<AH1>").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::StartTag {
            name: "a".to_string(),
            attributes: vec![Attribute {
                name: "href".to_string(),
                value: "1".to_string(),
            }],
            self_closing: false,
        }]
    );
    assert_eq!(tokenizer.diagnostics().len(), 1);
    assert_eq!(tokenizer.diagnostics()[0].code, "duplicate-attribute");
}

#[test]
fn tokenizer_builds_comment_tokens_with_current_and_literal_actions() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "comment", "done"]),
            set(&["!", "O", "k", ".", ";"]),
            vec![
                EffectfulTransition::new(
                    "data",
                    EffectfulMatcher::Event("!".to_string()),
                    "comment",
                )
                .with_effects(&["create_comment"]),
                EffectfulTransition::new(
                    "comment",
                    EffectfulMatcher::Event("O".to_string()),
                    "comment",
                )
                .with_effects(&["append_comment(current_lowercase)"]),
                EffectfulTransition::new(
                    "comment",
                    EffectfulMatcher::Event("k".to_string()),
                    "comment",
                )
                .with_effects(&["append_comment(current)"]),
                EffectfulTransition::new(
                    "comment",
                    EffectfulMatcher::Event(".".to_string()),
                    "comment",
                )
                .with_effects(&["append_comment(!)"]),
                EffectfulTransition::new(
                    "comment",
                    EffectfulMatcher::Event(";".to_string()),
                    "done",
                )
                .with_effects(&["emit_current_token"]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("!Ok.;").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::Comment("ok!".to_string())]
    );
}

#[test]
fn tokenizer_appends_replacement_character_to_comments() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["!"]),
            vec![EffectfulTransition::new(
                "data",
                EffectfulMatcher::Event("!".to_string()),
                "done",
            )
            .with_effects(&[
                "create_comment",
                "append_comment(open)",
                "append_comment_replacement",
                "emit_current_token",
            ])],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("!").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::Comment("open\u{FFFD}".to_string())]
    );
}

#[test]
fn tokenizer_builds_doctypes_and_marks_force_quirks() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "doctype", "done"]),
            set(&["D", "H", "t", "!"]),
            vec![
                EffectfulTransition::new(
                    "data",
                    EffectfulMatcher::Event("D".to_string()),
                    "doctype",
                )
                .with_effects(&["create_doctype"]),
                EffectfulTransition::new(
                    "doctype",
                    EffectfulMatcher::Event("H".to_string()),
                    "doctype",
                )
                .with_effects(&["append_doctype_name(current_lowercase)"]),
                EffectfulTransition::new(
                    "doctype",
                    EffectfulMatcher::Event("t".to_string()),
                    "doctype",
                )
                .with_effects(&["append_doctype_name(current)"]),
                EffectfulTransition::new(
                    "doctype",
                    EffectfulMatcher::Event("!".to_string()),
                    "done",
                )
                .with_effects(&[
                    "append_doctype_name(ml)",
                    "mark_force_quirks",
                    "emit_current_token",
                ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("DHt!").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::Doctype {
            name: Some("html".to_string()),
            public_identifier: None,
            system_identifier: None,
            force_quirks: true,
        }]
    );
}

#[test]
fn tokenizer_appends_replacement_character_to_doctype_fields() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["D"]),
            vec![EffectfulTransition::new(
                "data",
                EffectfulMatcher::Event("D".to_string()),
                "done",
            )
            .with_effects(&[
                "create_doctype",
                "append_doctype_name(html)",
                "append_doctype_name_replacement",
                "set_doctype_public_identifier_empty",
                "append_doctype_public_identifier(pub)",
                "append_doctype_public_identifier_replacement",
                "set_doctype_system_identifier_empty",
                "append_doctype_system_identifier(sys)",
                "append_doctype_system_identifier_replacement",
                "emit_current_token",
            ])],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("D").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::Doctype {
            name: Some("html\u{FFFD}".to_string()),
            public_identifier: Some("pub\u{FFFD}".to_string()),
            system_identifier: Some("sys\u{FFFD}".to_string()),
            force_quirks: false,
        }]
    );
}

#[test]
fn tokenizer_uses_temporary_buffer_actions() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "buffering", "done"]),
            set(&["A", "a", ";"]),
            vec![
                EffectfulTransition::new(
                    "data",
                    EffectfulMatcher::Event("A".to_string()),
                    "buffering",
                )
                .with_effects(&[
                    "clear_temporary_buffer",
                    "append_temporary_buffer(current_lowercase)",
                ]),
                EffectfulTransition::new(
                    "buffering",
                    EffectfulMatcher::Event("a".to_string()),
                    "buffering",
                )
                .with_effects(&["append_temporary_buffer(current)"]),
                EffectfulTransition::new(
                    "buffering",
                    EffectfulMatcher::Event(";".to_string()),
                    "done",
                )
                .with_effects(&[
                    "append_temporary_buffer(!)",
                    "append_temporary_buffer_to_text",
                    "flush_text",
                ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("Aa;").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::Text("aa!".to_string())]
    );
}

#[test]
fn tokenizer_appends_temporary_buffer_to_attribute_value() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "buffering", "done"]),
            set(&["A", "x", ";"]),
            vec![
                EffectfulTransition::new(
                    "data",
                    EffectfulMatcher::Event("A".to_string()),
                    "buffering",
                )
                .with_effects(&[
                    "create_start_tag",
                    "append_tag_name(current_lowercase)",
                    "start_attribute",
                    "append_attribute_name(href)",
                    "clear_temporary_buffer",
                    "append_temporary_buffer(&)",
                ]),
                EffectfulTransition::new(
                    "buffering",
                    EffectfulMatcher::Event("x".to_string()),
                    "buffering",
                )
                .with_effects(&["append_temporary_buffer(current)"]),
                EffectfulTransition::new(
                    "buffering",
                    EffectfulMatcher::Event(";".to_string()),
                    "done",
                )
                .with_effects(&[
                    "append_temporary_buffer_to_attribute_value",
                    "commit_attribute",
                    "emit_current_token",
                ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("Ax;").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::StartTag {
            name: "a".to_string(),
            attributes: vec![Attribute {
                name: "href".to_string(),
                value: "&x".to_string(),
            }],
            self_closing: false,
        }]
    );
}

#[test]
fn tokenizer_appends_replacement_character_to_attribute_value() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["A"]),
            vec![EffectfulTransition::new(
                "data",
                EffectfulMatcher::Event("A".to_string()),
                "done",
            )
            .with_effects(&[
                "create_start_tag",
                "append_tag_name(current_lowercase)",
                "start_attribute",
                "append_attribute_name(title)",
                "append_attribute_value_replacement",
                "commit_attribute",
                "emit_current_token",
            ])],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("A").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::StartTag {
            name: "a".to_string(),
            attributes: vec![Attribute {
                name: "title".to_string(),
                value: "\u{FFFD}".to_string(),
            }],
            self_closing: false,
        }]
    );
}

#[test]
fn tokenizer_appends_replacement_character_to_tag_and_attribute_names() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["A"]),
            vec![EffectfulTransition::new(
                "data",
                EffectfulMatcher::Event("A".to_string()),
                "done",
            )
            .with_effects(&[
                "create_start_tag",
                "append_tag_name(current_lowercase)",
                "append_tag_name_replacement",
                "start_attribute",
                "append_attribute_name_replacement",
                "append_attribute_name(title)",
                "append_attribute_value(ok)",
                "commit_attribute",
                "emit_current_token",
            ])],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("A").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::StartTag {
            name: "a\u{FFFD}".to_string(),
            attributes: vec![Attribute {
                name: "\u{FFFD}title".to_string(),
                value: "ok".to_string(),
            }],
            self_closing: false,
        }]
    );
}

#[test]
fn tokenizer_decodes_numeric_character_references_from_temporary_buffer() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&#x41)",
                        "append_numeric_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&#65)",
                        "append_numeric_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("A".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "A".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_reports_invalid_numeric_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["N"]),
            vec![EffectfulTransition::new(
                "data",
                EffectfulMatcher::Event("N".to_string()),
                "done",
            )
            .with_effects(&[
                "clear_temporary_buffer",
                "append_temporary_buffer(&#0)",
                "append_numeric_character_reference_to_text",
                "append_text( )",
                "append_temporary_buffer(&#xD800)",
                "append_numeric_character_reference_to_text",
                "append_text( )",
                "append_temporary_buffer(&#x110000)",
                "append_numeric_character_reference_to_text",
                "append_text( )",
                "append_temporary_buffer(&#xFDD0)",
                "append_numeric_character_reference_to_text",
                "append_text( )",
                "append_temporary_buffer(&#x80)",
                "append_numeric_character_reference_to_text",
                "flush_text",
            ])],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("N").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::Text(
            "\u{FFFD} \u{FFFD} \u{FFFD} \u{FDD0} \u{20AC}".to_string()
        )]
    );
    assert_eq!(
        tokenizer
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
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
fn tokenizer_decodes_named_character_references_from_temporary_buffer() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&nbsp)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&copy)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{00A0}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{00A9}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_case_sensitive_latin1_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&Agrave;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&agrave;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{00C0}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{00E0}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_spacing_and_invisible_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&ThickSpace;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&NoBreak;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{205F}\u{200A}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{2060}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_relational_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&NotEqualTilde;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&NotNestedGreaterGreater;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{2242}\u{0338}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{2AA2}\u{0338}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_equality_and_parallel_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&Congruent;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&nparsl;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{2261}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{2AFD}\u{20E5}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_greater_less_comparison_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&GreaterEqualLess;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&nLtv;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{22DB}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{226A}\u{0338}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_precedence_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&PrecedesEqual;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&succnapprox;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{2AAF}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{2ABA}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_arrow_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&LongLeftRightArrow;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&RightDownVectorBar;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{27F7}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{2955}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_extended_arrow_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&hookleftarrow;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&nLeftrightarrow;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{21A9}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{21CE}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_greek_variant_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&varepsilon;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&straightphi;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{03F5}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{03D5}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_set_and_logic_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&Intersection;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&NotSubset;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{22C2}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{2282}\u{20D2}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_operator_and_shape_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&CircleDot;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&blacklozenge;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{2299}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{29EB}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_box_drawing_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&boxVH;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&boxvr;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{256C}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{251C}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_angle_and_fence_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&angmsd;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&LeftDoubleBracket;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{2221}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{27E6}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_latin_extended_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&Amacr;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&ccaron;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{0100}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{010D}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_mathematical_alphabet_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&Aopf;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&zfr;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{1D538}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{1D537}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_cyrillic_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&Acy;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&zhcy;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{0410}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{0436}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_decodes_whatwg_remaining_arrow_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&DownLeftRightVector;)",
                        "append_named_character_reference_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&nrarrc;)",
                        "append_named_character_reference_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("\u{2950}".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{2933}\u{0338}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_falls_back_for_unknown_named_character_references() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "done"]),
            set(&["T", "A"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Event("T".to_string()), "data")
                    .with_effects(&[
                        "clear_temporary_buffer",
                        "append_temporary_buffer(&bogus;)",
                        "append_named_character_reference_or_temporary_buffer_to_text",
                        "flush_text",
                    ]),
                EffectfulTransition::new("data", EffectfulMatcher::Event("A".to_string()), "done")
                    .with_effects(&[
                        "create_start_tag",
                        "append_tag_name(current_lowercase)",
                        "start_attribute",
                        "append_attribute_name(title)",
                        "append_temporary_buffer(&copy)",
                        "append_named_character_reference_or_temporary_buffer_to_attribute_value",
                        "commit_attribute",
                        "emit_current_token",
                    ]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("TA").unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("&bogus;".to_string()),
            Token::StartTag {
                name: "a".to_string(),
                attributes: vec![Attribute {
                    name: "title".to_string(),
                    value: "\u{00A9}".to_string(),
                }],
                self_closing: false,
            },
        ]
    );
}

#[test]
fn tokenizer_supports_switch_to_with_reconsume() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "switched", "done"]),
            set(&["x"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Any, "data")
                    .with_effects(&["switch_to(switched)"])
                    .consuming(false),
                EffectfulTransition::new("switched", EffectfulMatcher::Any, "done")
                    .with_effects(&["append_text(current)", "flush_text"]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("x").unwrap();

    assert_eq!(tokenizer.drain_tokens(), vec![Token::Text("x".to_string())]);
    assert_eq!(tokenizer.trace()[0].to, "switched");
}

#[test]
fn tokenizer_supports_return_state_round_trips() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "entity", "done"]),
            set(&["&", "x"]),
            vec![
                EffectfulTransition::new(
                    "data",
                    EffectfulMatcher::Event("&".to_string()),
                    "entity",
                )
                .with_effects(&["set_return_state(data)"]),
                EffectfulTransition::new("entity", EffectfulMatcher::Any, "entity")
                    .with_effects(&["append_text(current)", "switch_to_return_state"]),
                EffectfulTransition::new("data", EffectfulMatcher::End, "done")
                    .with_effects(&["flush_text", "emit(EOF)"])
                    .consuming(false),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    );

    tokenizer.push("&x").unwrap();
    assert_eq!(tokenizer.current_state(), "data");

    tokenizer.finish().unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::Text("x".to_string()), Token::Eof]
    );
}

#[test]
fn tokenizer_supports_temporary_buffer_conditional_state_switches() {
    fn tokenizer() -> Tokenizer {
        Tokenizer::new(
            EffectfulStateMachine::new(
                set(&["data", "collect", "matched", "fallback"]),
                set(&[">"]),
                vec![
                    EffectfulTransition::new("data", EffectfulMatcher::Any, "collect")
                        .with_effects(&[
                            "clear_temporary_buffer",
                            "append_temporary_buffer(current_lowercase)",
                        ]),
                    EffectfulTransition::new(
                        "collect",
                        EffectfulMatcher::Event(">".to_string()),
                        "fallback",
                    )
                    .with_effects(&[
                        "switch_to_if_temporary_buffer_equals(script, matched, fallback)",
                    ]),
                    EffectfulTransition::new("collect", EffectfulMatcher::Any, "collect")
                        .with_effects(&["append_temporary_buffer(current_lowercase)"]),
                ],
                "data".to_string(),
                set(&[]),
            )
            .unwrap(),
        )
    }

    let mut matched = tokenizer();
    matched.push("script>").unwrap();
    assert_eq!(matched.current_state(), "matched");

    let mut fallback = tokenizer();
    fallback.push("style>").unwrap();
    assert_eq!(fallback.current_state(), "fallback");
}

#[test]
fn tokenizer_can_seed_initial_state_and_last_start_tag() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data", "rcdata", "done"]),
            set(&["H"]),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Any, "done"),
                EffectfulTransition::new("rcdata", EffectfulMatcher::Any, "done")
                    .with_effects(&["append_text(current)", "flush_text"]),
            ],
            "data".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    )
    .with_initial_state("rcdata")
    .unwrap()
    .with_last_start_tag("title");

    tokenizer.push("H").unwrap();

    assert_eq!(tokenizer.current_state(), "done");
    assert_eq!(tokenizer.drain_tokens(), vec![Token::Text("H".to_string())]);
}

#[test]
fn tokenizer_supports_rcdata_end_tag_candidate_fallback_action() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&[
                "rcdata",
                "less_than",
                "end_tag_open",
                "end_tag_name",
                "done",
            ]),
            set(&["H", "e", "l", "o", "<", "/", "t", "i", ">"]),
            vec![
                EffectfulTransition::new(
                    "rcdata",
                    EffectfulMatcher::Event("H".to_string()),
                    "rcdata",
                )
                .with_effects(&["append_text(current)"]),
                EffectfulTransition::new(
                    "rcdata",
                    EffectfulMatcher::Event("<".to_string()),
                    "less_than",
                ),
                EffectfulTransition::new("rcdata", EffectfulMatcher::Any, "rcdata")
                    .with_effects(&["append_text(current)"]),
                EffectfulTransition::new("rcdata", EffectfulMatcher::End, "done")
                    .with_effects(&["flush_text", "emit(EOF)"])
                    .consuming(false),
                EffectfulTransition::new(
                    "less_than",
                    EffectfulMatcher::Event("/".to_string()),
                    "end_tag_open",
                )
                .with_effects(&["clear_temporary_buffer"]),
                EffectfulTransition::new("end_tag_open", EffectfulMatcher::Any, "end_tag_name")
                    .with_effects(&[
                        "create_end_tag",
                        "append_tag_name(current_lowercase)",
                        "append_temporary_buffer(current_lowercase)",
                    ]),
                EffectfulTransition::new(
                    "end_tag_name",
                    EffectfulMatcher::Event(">".to_string()),
                    "rcdata",
                )
                .with_effects(&["emit_rcdata_end_tag_or_text"]),
                EffectfulTransition::new("end_tag_name", EffectfulMatcher::End, "done")
                    .with_effects(&["discard_current_token", "flush_text", "emit(EOF)"])
                    .consuming(false),
                EffectfulTransition::new("end_tag_name", EffectfulMatcher::Any, "end_tag_name")
                    .with_effects(&[
                        "append_tag_name(current_lowercase)",
                        "append_temporary_buffer(current_lowercase)",
                    ]),
            ],
            "rcdata".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    )
    .with_last_start_tag("title");

    tokenizer.push("Hello</title>").unwrap();
    tokenizer.finish().unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("Hello".to_string()),
            Token::EndTag {
                name: "title".to_string(),
            },
            Token::Eof,
        ]
    );

    let mut mismatch = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&[
                "rcdata",
                "less_than",
                "end_tag_open",
                "end_tag_name",
                "done",
            ]),
            set(&["H", "e", "l", "o", "<", "/", "s", "t", "y", ">"]),
            vec![
                EffectfulTransition::new(
                    "rcdata",
                    EffectfulMatcher::Event("H".to_string()),
                    "rcdata",
                )
                .with_effects(&["append_text(current)"]),
                EffectfulTransition::new(
                    "rcdata",
                    EffectfulMatcher::Event("<".to_string()),
                    "less_than",
                ),
                EffectfulTransition::new("rcdata", EffectfulMatcher::Any, "rcdata")
                    .with_effects(&["append_text(current)"]),
                EffectfulTransition::new("rcdata", EffectfulMatcher::End, "done")
                    .with_effects(&["flush_text", "emit(EOF)"])
                    .consuming(false),
                EffectfulTransition::new(
                    "less_than",
                    EffectfulMatcher::Event("/".to_string()),
                    "end_tag_open",
                )
                .with_effects(&["clear_temporary_buffer"]),
                EffectfulTransition::new("end_tag_open", EffectfulMatcher::Any, "end_tag_name")
                    .with_effects(&[
                        "create_end_tag",
                        "append_tag_name(current_lowercase)",
                        "append_temporary_buffer(current_lowercase)",
                    ]),
                EffectfulTransition::new(
                    "end_tag_name",
                    EffectfulMatcher::Event(">".to_string()),
                    "rcdata",
                )
                .with_effects(&["emit_rcdata_end_tag_or_text"]),
                EffectfulTransition::new("end_tag_name", EffectfulMatcher::End, "done")
                    .with_effects(&["discard_current_token", "flush_text", "emit(EOF)"])
                    .consuming(false),
                EffectfulTransition::new("end_tag_name", EffectfulMatcher::Any, "end_tag_name")
                    .with_effects(&[
                        "append_tag_name(current_lowercase)",
                        "append_temporary_buffer(current_lowercase)",
                    ]),
            ],
            "rcdata".to_string(),
            set(&["done"]),
        )
        .unwrap(),
    )
    .with_last_start_tag("title");

    mismatch.push("Hello</style>").unwrap();
    mismatch.finish().unwrap();

    assert_eq!(
        mismatch.drain_tokens(),
        vec![Token::Text("Hello</style>".to_string()), Token::Eof]
    );
}

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}
