// AUTO-GENERATED FILE — DO NOT EDIT
// Source: s.grammar
// Regenerate with: grammar-tools compile-grammar s.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
            GrammarRule {
                name: r#"program"#.to_string(),
                body: GrammarElement::Repetition {
                    element: Box::new(GrammarElement::RuleReference {
                        name: r#"statement_line"#.to_string(),
                    }),
                },
                line_number: 51,
            },
            GrammarRule {
                name: r#"statement_line"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::RuleReference {
                                    name: r#"statement"#.to_string(),
                                },
                                GrammarElement::Group {
                                    element: Box::new(GrammarElement::Alternation {
                                        choices: vec![
                                            GrammarElement::TokenReference {
                                                name: r#"NEWLINE"#.to_string(),
                                            },
                                            GrammarElement::TokenReference {
                                                name: r#"SEMICOLON"#.to_string(),
                                            },
                                        ],
                                    }),
                                },
                            ],
                        },
                        GrammarElement::RuleReference {
                            name: r#"statement"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NEWLINE"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"SEMICOLON"#.to_string(),
                        },
                    ],
                },
                line_number: 53,
            },
            GrammarRule {
                name: r#"statement"#.to_string(),
                body: GrammarElement::RuleReference {
                    name: r#"expr"#.to_string(),
                },
                line_number: 59,
            },
            GrammarRule {
                name: r#"expr"#.to_string(),
                body: GrammarElement::RuleReference {
                    name: r#"assignment"#.to_string(),
                },
                line_number: 61,
            },
            GrammarRule {
                name: r#"assignment"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"comparison"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Group {
                                        element: Box::new(GrammarElement::Alternation {
                                            choices: vec![
                                                GrammarElement::TokenReference {
                                                    name: r#"ASSIGN"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"UNDERSCORE"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"SUPER_ASSIGN"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"RIGHT_ASSIGN"#.to_string(),
                                                },
                                            ],
                                        }),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"assignment"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 74,
            },
            GrammarRule {
                name: r#"comparison"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"range"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Group {
                                        element: Box::new(GrammarElement::Alternation {
                                            choices: vec![
                                                GrammarElement::TokenReference {
                                                    name: r#"EQEQ"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"NE"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"LE"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"GE"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"LT"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"GT"#.to_string(),
                                                },
                                            ],
                                        }),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"range"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 81,
            },
            GrammarRule {
                name: r#"range"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"additive"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COLON"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"additive"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 87,
            },
            GrammarRule {
                name: r#"additive"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"multiplicative"#.to_string(),
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
                                        name: r#"multiplicative"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 93,
            },
            GrammarRule {
                name: r#"multiplicative"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"unary"#.to_string(),
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
                                        name: r#"unary"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 95,
            },
            GrammarRule {
                name: r#"unary"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::TokenReference {
                                    name: r#"MINUS"#.to_string(),
                                },
                                GrammarElement::RuleReference {
                                    name: r#"unary"#.to_string(),
                                },
                            ],
                        },
                        GrammarElement::RuleReference {
                            name: r#"power"#.to_string(),
                        },
                    ],
                },
                line_number: 101,
            },
            GrammarRule {
                name: r#"power"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"postfix"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"CARET"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"unary"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 109,
            },
            GrammarRule {
                name: r#"postfix"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"primary"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Alternation {
                                choices: vec![
                                    GrammarElement::RuleReference {
                                        name: r#"call_suffix"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"index_suffix"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 118,
            },
            GrammarRule {
                name: r#"call_suffix"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"LPAREN"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"arg_list"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RPAREN"#.to_string(),
                        },
                    ],
                },
                line_number: 120,
            },
            GrammarRule {
                name: r#"index_suffix"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"LBRACKET"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"arg_list"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RBRACKET"#.to_string(),
                        },
                    ],
                },
                line_number: 121,
            },
            GrammarRule {
                name: r#"arg_list"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"arg"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COMMA"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"arg"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 123,
            },
            GrammarRule {
                name: r#"arg"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::TokenReference {
                                    name: r#"NAME"#.to_string(),
                                },
                                GrammarElement::TokenReference {
                                    name: r#"EQ"#.to_string(),
                                },
                                GrammarElement::RuleReference {
                                    name: r#"expr"#.to_string(),
                                },
                            ],
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 127,
            },
            GrammarRule {
                name: r#"primary"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference {
                            name: r#"NUMBER"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"STRING"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"TRUE"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"FALSE"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"T"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"F"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"NULL"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"NA"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"Inf"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"NaN"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"func_def"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"if_expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"for_expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"while_expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"repeat_expr"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"break"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"next"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"group"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                    ],
                },
                line_number: 138,
            },
            GrammarRule {
                name: r#"group"#.to_string(),
                body: GrammarElement::Sequence {
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
                line_number: 159,
            },
            GrammarRule {
                name: r#"block"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"LBRACE"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"statement_line"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RBRACE"#.to_string(),
                        },
                    ],
                },
                line_number: 161,
            },
            GrammarRule {
                name: r#"func_def"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"function"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"LPAREN"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"param_list"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RPAREN"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 170,
            },
            GrammarRule {
                name: r#"param_list"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"param"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COMMA"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"param"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 172,
            },
            GrammarRule {
                name: r#"param"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::TokenReference {
                                    name: r#"NAME"#.to_string(),
                                },
                                GrammarElement::TokenReference {
                                    name: r#"EQ"#.to_string(),
                                },
                                GrammarElement::RuleReference {
                                    name: r#"expr"#.to_string(),
                                },
                            ],
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                    ],
                },
                line_number: 174,
            },
            GrammarRule {
                name: r#"if_expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"if"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"LPAREN"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RPAREN"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Literal {
                                        value: r#"else"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expr"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 177,
            },
            GrammarRule {
                name: r#"for_expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"for"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"LPAREN"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"in"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RPAREN"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 179,
            },
            GrammarRule {
                name: r#"while_expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"while"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"LPAREN"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RPAREN"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 181,
            },
            GrammarRule {
                name: r#"repeat_expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"repeat"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 183,
            },
        ],
        version: 1,
    }
}
