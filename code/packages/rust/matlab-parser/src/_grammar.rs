// AUTO-GENERATED FILE — DO NOT EDIT
// Source: matlab.grammar
// Regenerate with: grammar-tools compile-grammar matlab.grammar
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
                line_number: 36,
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
                                GrammarElement::RuleReference {
                                    name: r#"stmt_term"#.to_string(),
                                },
                            ],
                        },
                        GrammarElement::RuleReference {
                            name: r#"statement"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"stmt_term"#.to_string(),
                        },
                    ],
                },
                line_number: 41,
            },
            GrammarRule {
                name: r#"stmt_term"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference {
                            name: r#"NEWLINE"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"SEMICOLON"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"COMMA"#.to_string(),
                        },
                    ],
                },
                line_number: 45,
            },
            GrammarRule {
                name: r#"statement"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::RuleReference {
                            name: r#"func_def"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"if_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"for_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"while_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"switch_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"try_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"break_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"continue_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"return_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"global_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 47,
            },
            GrammarRule {
                name: r#"block_body"#.to_string(),
                body: GrammarElement::Repetition {
                    element: Box::new(GrammarElement::RuleReference {
                        name: r#"statement_line"#.to_string(),
                    }),
                },
                line_number: 62,
            },
            GrammarRule {
                name: r#"if_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"if"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"elseif_clause"#.to_string(),
                            }),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"else_clause"#.to_string(),
                            }),
                        },
                        GrammarElement::Literal {
                            value: r#"end"#.to_string(),
                        },
                    ],
                },
                line_number: 68,
            },
            GrammarRule {
                name: r#"elseif_clause"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"elseif"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                    ],
                },
                line_number: 69,
            },
            GrammarRule {
                name: r#"else_clause"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"else"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                    ],
                },
                line_number: 70,
            },
            GrammarRule {
                name: r#"for_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"for"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"EQ"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"end"#.to_string(),
                        },
                    ],
                },
                line_number: 72,
            },
            GrammarRule {
                name: r#"while_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"while"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"end"#.to_string(),
                        },
                    ],
                },
                line_number: 74,
            },
            GrammarRule {
                name: r#"switch_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"switch"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"case_clause"#.to_string(),
                            }),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"otherwise_clause"#.to_string(),
                            }),
                        },
                        GrammarElement::Literal {
                            value: r#"end"#.to_string(),
                        },
                    ],
                },
                line_number: 76,
            },
            GrammarRule {
                name: r#"case_clause"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"case"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                    ],
                },
                line_number: 77,
            },
            GrammarRule {
                name: r#"otherwise_clause"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"otherwise"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                    ],
                },
                line_number: 78,
            },
            GrammarRule {
                name: r#"try_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"try"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"catch_clause"#.to_string(),
                            }),
                        },
                        GrammarElement::Literal {
                            value: r#"end"#.to_string(),
                        },
                    ],
                },
                line_number: 80,
            },
            GrammarRule {
                name: r#"catch_clause"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"catch"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::TokenReference {
                                name: r#"NAME"#.to_string(),
                            }),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                    ],
                },
                line_number: 81,
            },
            GrammarRule {
                name: r#"break_stmt"#.to_string(),
                body: GrammarElement::Literal {
                    value: r#"break"#.to_string(),
                },
                line_number: 83,
            },
            GrammarRule {
                name: r#"continue_stmt"#.to_string(),
                body: GrammarElement::Literal {
                    value: r#"continue"#.to_string(),
                },
                line_number: 84,
            },
            GrammarRule {
                name: r#"return_stmt"#.to_string(),
                body: GrammarElement::Literal {
                    value: r#"return"#.to_string(),
                },
                line_number: 85,
            },
            GrammarRule {
                name: r#"global_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Group {
                            element: Box::new(GrammarElement::Alternation {
                                choices: vec![
                                    GrammarElement::Literal {
                                        value: r#"global"#.to_string(),
                                    },
                                    GrammarElement::Literal {
                                        value: r#"persistent"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::RuleReference {
                            name: r#"name_list"#.to_string(),
                        },
                    ],
                },
                line_number: 86,
            },
            GrammarRule {
                name: r#"func_def"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"function"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"func_returns"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"LPAREN"#.to_string(),
                                    },
                                    GrammarElement::Optional {
                                        element: Box::new(GrammarElement::RuleReference {
                                            name: r#"name_list"#.to_string(),
                                        }),
                                    },
                                    GrammarElement::TokenReference {
                                        name: r#"RPAREN"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"end"#.to_string(),
                        },
                    ],
                },
                line_number: 96,
            },
            GrammarRule {
                name: r#"func_returns"#.to_string(),
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
                            ],
                        },
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::TokenReference {
                                    name: r#"LBRACKET"#.to_string(),
                                },
                                GrammarElement::Optional {
                                    element: Box::new(GrammarElement::RuleReference {
                                        name: r#"name_list"#.to_string(),
                                    }),
                                },
                                GrammarElement::TokenReference {
                                    name: r#"RBRACKET"#.to_string(),
                                },
                                GrammarElement::TokenReference {
                                    name: r#"EQ"#.to_string(),
                                },
                            ],
                        },
                    ],
                },
                line_number: 97,
            },
            GrammarRule {
                name: r#"name_list"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COMMA"#.to_string(),
                                    },
                                    GrammarElement::TokenReference {
                                        name: r#"NAME"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 99,
            },
            GrammarRule {
                name: r#"expr"#.to_string(),
                body: GrammarElement::RuleReference {
                    name: r#"assignment"#.to_string(),
                },
                line_number: 105,
            },
            GrammarRule {
                name: r#"assignment"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"logical_or"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"EQ"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"assignment"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 108,
            },
            GrammarRule {
                name: r#"logical_or"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"logical_and"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"OR_OR"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"logical_and"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 110,
            },
            GrammarRule {
                name: r#"logical_and"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"bit_or"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"AND_AND"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"bit_or"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 111,
            },
            GrammarRule {
                name: r#"bit_or"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"bit_and"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"PIPE"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"bit_and"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 112,
            },
            GrammarRule {
                name: r#"bit_and"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"comparison"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"AMP"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"comparison"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 113,
            },
            GrammarRule {
                name: r#"comparison"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"colon_expr"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Group {
                                        element: Box::new(GrammarElement::Alternation {
                                            choices: vec![
                                                GrammarElement::TokenReference {
                                                    name: r#"EQ_EQ"#.to_string(),
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
                                        name: r#"colon_expr"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 115,
            },
            GrammarRule {
                name: r#"colon_expr"#.to_string(),
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
                            }),
                        },
                    ],
                },
                line_number: 119,
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
                line_number: 121,
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
                                                GrammarElement::TokenReference {
                                                    name: r#"BACKSLASH"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"ELEM_MUL"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"ELEM_RDIV"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"ELEM_LDIV"#.to_string(),
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
                line_number: 124,
            },
            GrammarRule {
                name: r#"unary"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
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
                                            GrammarElement::TokenReference {
                                                name: r#"TILDE"#.to_string(),
                                            },
                                        ],
                                    }),
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
                line_number: 127,
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
                                    GrammarElement::Group {
                                        element: Box::new(GrammarElement::Alternation {
                                            choices: vec![
                                                GrammarElement::TokenReference {
                                                    name: r#"CARET"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"ELEM_POW"#.to_string(),
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
                line_number: 131,
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
                                        name: r#"transpose_suffix"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"call_suffix"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"cell_suffix"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"field_suffix"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 134,
            },
            GrammarRule {
                name: r#"transpose_suffix"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference {
                            name: r#"TRANSPOSE"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"ELEM_TRANSPOSE"#.to_string(),
                        },
                    ],
                },
                line_number: 136,
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
                line_number: 137,
            },
            GrammarRule {
                name: r#"cell_suffix"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"LBRACE"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"arg_list"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RBRACE"#.to_string(),
                        },
                    ],
                },
                line_number: 138,
            },
            GrammarRule {
                name: r#"field_suffix"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"DOT"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                    ],
                },
                line_number: 139,
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
                line_number: 142,
            },
            GrammarRule {
                name: r#"arg"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference {
                            name: r#"COLON"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 143,
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
                        GrammarElement::RuleReference {
                            name: r#"matrix_literal"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"cell_literal"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"lambda"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"group"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                    ],
                },
                line_number: 149,
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
                line_number: 157,
            },
            GrammarRule {
                name: r#"lambda"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"AT"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"LPAREN"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"name_list"#.to_string(),
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
                line_number: 159,
            },
            GrammarRule {
                name: r#"matrix_literal"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"LBRACKET"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"matrix_rows"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RBRACKET"#.to_string(),
                        },
                    ],
                },
                line_number: 176,
            },
            GrammarRule {
                name: r#"cell_literal"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"LBRACE"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"matrix_rows"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RBRACE"#.to_string(),
                        },
                    ],
                },
                line_number: 177,
            },
            GrammarRule {
                name: r#"matrix_rows"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"matrix_row"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::RuleReference {
                                        name: r#"row_sep"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"matrix_row"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"row_sep"#.to_string(),
                            }),
                        },
                    ],
                },
                line_number: 179,
            },
            GrammarRule {
                name: r#"row_sep"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference {
                            name: r#"SEMICOLON"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NEWLINE"#.to_string(),
                        },
                    ],
                },
                line_number: 180,
            },
            GrammarRule {
                name: r#"matrix_row"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Optional {
                                        element: Box::new(GrammarElement::TokenReference {
                                            name: r#"COMMA"#.to_string(),
                                        }),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expr"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 181,
            },
        ],
        version: 0,
    }
}
