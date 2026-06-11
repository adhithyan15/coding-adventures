// AUTO-GENERATED FILE — DO NOT EDIT
// Source: adj_lang.grammar
// Regenerate with: grammar-tools compile-grammar adj_lang.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
            GrammarRule {
                name: r#"program"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"statement"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"EOF"#.to_string(),
                        },
                    ],
                },
                line_number: 20,
            },
            GrammarRule {
                name: r#"statement"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::RuleReference {
                            name: r#"prior_decl"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"contributes_decl"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"interacts_decl"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"uncertain_decl"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"observe_decl"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"query_decl"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"let_decl"#.to_string(),
                        },
                    ],
                },
                line_number: 22,
            },
            GrammarRule {
                name: r#"prior_decl"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"prior"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NUMBER"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"for"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"term"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"annotation"#.to_string(),
                            }),
                        },
                    ],
                },
                line_number: 36,
            },
            GrammarRule {
                name: r#"contributes_decl"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"contributes"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NUMBER"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"from"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"evidence"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"to"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"term"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"annotation"#.to_string(),
                            }),
                        },
                    ],
                },
                line_number: 38,
            },
            GrammarRule {
                name: r#"evidence"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::RuleReference {
                            name: r#"predicate"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"term"#.to_string(),
                        },
                    ],
                },
                line_number: 48,
            },
            GrammarRule {
                name: r#"predicate"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"IDENT"#.to_string(),
                        },
                        GrammarElement::Group {
                            element: Box::new(GrammarElement::Alternation {
                                choices: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"GE"#.to_string(),
                                    },
                                    GrammarElement::TokenReference {
                                        name: r#"LE"#.to_string(),
                                    },
                                    GrammarElement::TokenReference {
                                        name: r#"GT"#.to_string(),
                                    },
                                    GrammarElement::TokenReference {
                                        name: r#"LT"#.to_string(),
                                    },
                                    GrammarElement::TokenReference {
                                        name: r#"EQEQ"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NUMBER"#.to_string(),
                        },
                    ],
                },
                line_number: 50,
            },
            GrammarRule {
                name: r#"interacts_decl"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"interacts"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NUMBER"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"when"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"term"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"and"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"term"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Literal {
                                        value: r#"and"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"term"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::Literal {
                            value: r#"for"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"term"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"annotation"#.to_string(),
                            }),
                        },
                    ],
                },
                line_number: 52,
            },
            GrammarRule {
                name: r#"uncertain_decl"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"uncertain"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"LBRACE"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"term"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COMMA"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"term"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RBRACE"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"for"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"term"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"annotation"#.to_string(),
                            }),
                        },
                    ],
                },
                line_number: 54,
            },
            GrammarRule {
                name: r#"observe_decl"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"observe"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"term"#.to_string(),
                        },
                    ],
                },
                line_number: 56,
            },
            GrammarRule {
                name: r#"query_decl"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"QUESTION"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"term"#.to_string(),
                        },
                    ],
                },
                line_number: 58,
            },
            GrammarRule {
                name: r#"let_decl"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"let"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"IDENT"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"EQUALS"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 72,
            },
            GrammarRule {
                name: r#"expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"term_expr"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Group {
                                        element: Box::new(GrammarElement::Alternation {
                                            choices: vec![
                                                GrammarElement::TokenReference {
                                                    name: r#"PLUS"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"MINUS"#.to_string(),
                                                },
                                            ],
                                        }),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"term_expr"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 74,
            },
            GrammarRule {
                name: r#"term_expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"factor"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Group {
                                        element: Box::new(GrammarElement::Alternation {
                                            choices: vec![
                                                GrammarElement::TokenReference {
                                                    name: r#"STAR"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"SLASH"#.to_string(),
                                                },
                                            ],
                                        }),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"factor"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 76,
            },
            GrammarRule {
                name: r#"factor"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::RuleReference {
                            name: r#"agg"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NUMBER"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"IDENT"#.to_string(),
                        },
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::TokenReference {
                                    name: r#"LPAREN"#.to_string(),
                                },
                                GrammarElement::RuleReference {
                                    name: r#"expr"#.to_string(),
                                },
                                GrammarElement::TokenReference {
                                    name: r#"RPAREN"#.to_string(),
                                },
                            ],
                        },
                    ],
                },
                line_number: 78,
            },
            GrammarRule {
                name: r#"agg"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Group {
                            element: Box::new(GrammarElement::Alternation {
                                choices: vec![
                                    GrammarElement::Literal {
                                        value: r#"sum"#.to_string(),
                                    },
                                    GrammarElement::Literal {
                                        value: r#"count"#.to_string(),
                                    },
                                    GrammarElement::Literal {
                                        value: r#"min"#.to_string(),
                                    },
                                    GrammarElement::Literal {
                                        value: r#"max"#.to_string(),
                                    },
                                    GrammarElement::Literal {
                                        value: r#"avg"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"LPAREN"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"IDENT"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RPAREN"#.to_string(),
                        },
                    ],
                },
                line_number: 80,
            },
            GrammarRule {
                name: r#"annotation"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::RuleReference {
                            name: r#"source_annotation"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"locator_annotation"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"trust_annotation"#.to_string(),
                        },
                    ],
                },
                line_number: 88,
            },
            GrammarRule {
                name: r#"source_annotation"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"source"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"STRING"#.to_string(),
                        },
                    ],
                },
                line_number: 93,
            },
            GrammarRule {
                name: r#"locator_annotation"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"locator"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"STRING"#.to_string(),
                        },
                    ],
                },
                line_number: 94,
            },
            GrammarRule {
                name: r#"trust_annotation"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"trust"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"trust_tier"#.to_string(),
                        },
                    ],
                },
                line_number: 95,
            },
            GrammarRule {
                name: r#"trust_tier"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Literal {
                            value: r#"consensus"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"authoritative"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"empirical"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"inferred"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"unattributed"#.to_string(),
                        },
                    ],
                },
                line_number: 97,
            },
            GrammarRule {
                name: r#"term"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"IDENT"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"LPAREN"#.to_string(),
                                    },
                                    GrammarElement::Group {
                                        element: Box::new(GrammarElement::Alternation {
                                            choices: vec![
                                                GrammarElement::RuleReference {
                                                    name: r#"term"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"NUMBER"#.to_string(),
                                                },
                                            ],
                                        }),
                                    },
                                    GrammarElement::Repetition {
                                        element: Box::new(GrammarElement::Sequence {
                                            elements: vec![
                                                GrammarElement::TokenReference {
                                                    name: r#"COMMA"#.to_string(),
                                                },
                                                GrammarElement::Group {
                                                    element: Box::new(
                                                        GrammarElement::Alternation {
                                                            choices: vec![
                                                                GrammarElement::RuleReference {
                                                                    name: r#"term"#.to_string(),
                                                                },
                                                                GrammarElement::TokenReference {
                                                                    name: r#"NUMBER"#.to_string(),
                                                                },
                                                            ],
                                                        },
                                                    ),
                                                },
                                            ],
                                        }),
                                    },
                                    GrammarElement::TokenReference {
                                        name: r#"RPAREN"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 114,
            },
        ],
        version: 1,
    }
}
