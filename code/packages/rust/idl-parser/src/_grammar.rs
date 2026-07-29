// AUTO-GENERATED FILE — DO NOT EDIT
// Source: idl.grammar
// Regenerate with: grammar-tools compile-grammar idl.grammar
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
                        name: r#"top_level_item"#.to_string(),
                    }),
                },
                line_number: 279,
            },
            GrammarRule {
                name: r#"top_level_item"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::RuleReference {
                            name: r#"pro_def"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"func_def"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"statement_line"#.to_string(),
                        },
                    ],
                },
                line_number: 281,
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
                                GrammarElement::Repetition {
                                    element: Box::new(GrammarElement::Sequence {
                                        elements: vec![
                                            GrammarElement::TokenReference {
                                                name: r#"STMT_SEP"#.to_string(),
                                            },
                                            GrammarElement::RuleReference {
                                                name: r#"statement"#.to_string(),
                                            },
                                        ],
                                    }),
                                },
                                GrammarElement::Optional {
                                    element: Box::new(GrammarElement::TokenReference {
                                        name: r#"NEWLINE"#.to_string(),
                                    }),
                                },
                            ],
                        },
                        GrammarElement::TokenReference {
                            name: r#"NEWLINE"#.to_string(),
                        },
                    ],
                },
                line_number: 292,
            },
            GrammarRule {
                name: r#"block_body"#.to_string(),
                body: GrammarElement::Repetition {
                    element: Box::new(GrammarElement::RuleReference {
                        name: r#"statement_line"#.to_string(),
                    }),
                },
                line_number: 302,
            },
            GrammarRule {
                name: r#"statement"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
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
                            name: r#"repeat_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"begin_block"#.to_string(),
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
                            name: r#"procedure_call_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"assignment_stmt"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr_stmt"#.to_string(),
                        },
                    ],
                },
                line_number: 304,
            },
            GrammarRule {
                name: r#"if_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"IF"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"THEN"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"then_branch"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Literal {
                                        value: r#"ELSE"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"else_branch"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 324,
            },
            GrammarRule {
                name: r#"then_branch"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::Literal {
                                    value: r#"BEGIN"#.to_string(),
                                },
                                GrammarElement::RuleReference {
                                    name: r#"block_body"#.to_string(),
                                },
                                GrammarElement::Group {
                                    element: Box::new(GrammarElement::Alternation {
                                        choices: vec![
                                            GrammarElement::Literal {
                                                value: r#"ENDIF"#.to_string(),
                                            },
                                            GrammarElement::Literal {
                                                value: r#"END"#.to_string(),
                                            },
                                        ],
                                    }),
                                },
                            ],
                        },
                        GrammarElement::RuleReference {
                            name: r#"statement"#.to_string(),
                        },
                    ],
                },
                line_number: 326,
            },
            GrammarRule {
                name: r#"else_branch"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::Literal {
                                    value: r#"BEGIN"#.to_string(),
                                },
                                GrammarElement::RuleReference {
                                    name: r#"block_body"#.to_string(),
                                },
                                GrammarElement::Group {
                                    element: Box::new(GrammarElement::Alternation {
                                        choices: vec![
                                            GrammarElement::Literal {
                                                value: r#"ENDELSE"#.to_string(),
                                            },
                                            GrammarElement::Literal {
                                                value: r#"END"#.to_string(),
                                            },
                                        ],
                                    }),
                                },
                            ],
                        },
                        GrammarElement::RuleReference {
                            name: r#"statement"#.to_string(),
                        },
                    ],
                },
                line_number: 329,
            },
            GrammarRule {
                name: r#"for_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"FOR"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"EQUALS"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"COMMA"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COMMA"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expr"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::Literal {
                            value: r#"DO"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"for_body"#.to_string(),
                        },
                    ],
                },
                line_number: 335,
            },
            GrammarRule {
                name: r#"for_body"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::Literal {
                                    value: r#"BEGIN"#.to_string(),
                                },
                                GrammarElement::RuleReference {
                                    name: r#"block_body"#.to_string(),
                                },
                                GrammarElement::Group {
                                    element: Box::new(GrammarElement::Alternation {
                                        choices: vec![
                                            GrammarElement::Literal {
                                                value: r#"ENDFOR"#.to_string(),
                                            },
                                            GrammarElement::Literal {
                                                value: r#"END"#.to_string(),
                                            },
                                        ],
                                    }),
                                },
                            ],
                        },
                        GrammarElement::RuleReference {
                            name: r#"statement"#.to_string(),
                        },
                    ],
                },
                line_number: 337,
            },
            GrammarRule {
                name: r#"while_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"WHILE"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"DO"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"while_body"#.to_string(),
                        },
                    ],
                },
                line_number: 342,
            },
            GrammarRule {
                name: r#"while_body"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::Literal {
                                    value: r#"BEGIN"#.to_string(),
                                },
                                GrammarElement::RuleReference {
                                    name: r#"block_body"#.to_string(),
                                },
                                GrammarElement::Group {
                                    element: Box::new(GrammarElement::Alternation {
                                        choices: vec![
                                            GrammarElement::Literal {
                                                value: r#"ENDWHILE"#.to_string(),
                                            },
                                            GrammarElement::Literal {
                                                value: r#"END"#.to_string(),
                                            },
                                        ],
                                    }),
                                },
                            ],
                        },
                        GrammarElement::RuleReference {
                            name: r#"statement"#.to_string(),
                        },
                    ],
                },
                line_number: 344,
            },
            GrammarRule {
                name: r#"repeat_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"REPEAT"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"repeat_body"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"UNTIL"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 351,
            },
            GrammarRule {
                name: r#"repeat_body"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::Literal {
                                    value: r#"BEGIN"#.to_string(),
                                },
                                GrammarElement::RuleReference {
                                    name: r#"block_body"#.to_string(),
                                },
                                GrammarElement::Group {
                                    element: Box::new(GrammarElement::Alternation {
                                        choices: vec![
                                            GrammarElement::Literal {
                                                value: r#"ENDREP"#.to_string(),
                                            },
                                            GrammarElement::Literal {
                                                value: r#"END"#.to_string(),
                                            },
                                        ],
                                    }),
                                },
                            ],
                        },
                        GrammarElement::RuleReference {
                            name: r#"statement"#.to_string(),
                        },
                    ],
                },
                line_number: 353,
            },
            GrammarRule {
                name: r#"begin_block"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"BEGIN"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"END"#.to_string(),
                        },
                    ],
                },
                line_number: 361,
            },
            GrammarRule {
                name: r#"break_stmt"#.to_string(),
                body: GrammarElement::Literal {
                    value: r#"BREAK"#.to_string(),
                },
                line_number: 363,
            },
            GrammarRule {
                name: r#"continue_stmt"#.to_string(),
                body: GrammarElement::Literal {
                    value: r#"CONTINUE"#.to_string(),
                },
                line_number: 364,
            },
            GrammarRule {
                name: r#"return_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"RETURN"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COMMA"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expr"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 375,
            },
            GrammarRule {
                name: r#"procedure_call_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"COMMA"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"arg_list"#.to_string(),
                        },
                    ],
                },
                line_number: 393,
            },
            GrammarRule {
                name: r#"assignment_stmt"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"index_suffix"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"EQUALS"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 402,
            },
            GrammarRule {
                name: r#"expr_stmt"#.to_string(),
                body: GrammarElement::RuleReference {
                    name: r#"expr"#.to_string(),
                },
                line_number: 408,
            },
            GrammarRule {
                name: r#"pro_def"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"PRO"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COMMA"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"params"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"END"#.to_string(),
                        },
                    ],
                },
                line_number: 429,
            },
            GrammarRule {
                name: r#"func_def"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"FUNCTION"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COMMA"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"params"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::RuleReference {
                            name: r#"block_body"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"END"#.to_string(),
                        },
                    ],
                },
                line_number: 430,
            },
            GrammarRule {
                name: r#"params"#.to_string(),
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
                line_number: 432,
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
                                    name: r#"EQUALS"#.to_string(),
                                },
                                GrammarElement::TokenReference {
                                    name: r#"NAME"#.to_string(),
                                },
                            ],
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                    ],
                },
                line_number: 433,
            },
            GrammarRule {
                name: r#"expr"#.to_string(),
                body: GrammarElement::RuleReference {
                    name: r#"logical"#.to_string(),
                },
                line_number: 444,
            },
            GrammarRule {
                name: r#"logical"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"comparison"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Group {
                                        element: Box::new(GrammarElement::Alternation {
                                            choices: vec![
                                                GrammarElement::Literal {
                                                    value: r#"AND"#.to_string(),
                                                },
                                                GrammarElement::Literal {
                                                    value: r#"OR"#.to_string(),
                                                },
                                                GrammarElement::Literal {
                                                    value: r#"XOR"#.to_string(),
                                                },
                                            ],
                                        }),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"comparison"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 446,
            },
            GrammarRule {
                name: r#"comparison"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"additive"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Group {
                                        element: Box::new(GrammarElement::Alternation {
                                            choices: vec![
                                                GrammarElement::Literal {
                                                    value: r#"EQ"#.to_string(),
                                                },
                                                GrammarElement::Literal {
                                                    value: r#"NE"#.to_string(),
                                                },
                                                GrammarElement::Literal {
                                                    value: r#"LE"#.to_string(),
                                                },
                                                GrammarElement::Literal {
                                                    value: r#"LT"#.to_string(),
                                                },
                                                GrammarElement::Literal {
                                                    value: r#"GE"#.to_string(),
                                                },
                                                GrammarElement::Literal {
                                                    value: r#"GT"#.to_string(),
                                                },
                                            ],
                                        }),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"additive"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 448,
            },
            GrammarRule {
                name: r#"additive"#.to_string(),
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
                                                    name: r#"PLUS"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"MINUS"#.to_string(),
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
                line_number: 454,
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
                                            GrammarElement::Literal {
                                                value: r#"NOT"#.to_string(),
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
                            name: r#"multiplicative"#.to_string(),
                        },
                    ],
                },
                line_number: 461,
            },
            GrammarRule {
                name: r#"multiplicative"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"power"#.to_string(),
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
                                                    name: r#"HASH_HASH"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"HASH"#.to_string(),
                                                },
                                            ],
                                        }),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"power"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 472,
            },
            GrammarRule {
                name: r#"power"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"postfix"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"CARET"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"postfix"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 479,
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
                                        name: r#"index_suffix"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"call_suffix"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 487,
            },
            GrammarRule {
                name: r#"index_suffix"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"LBRACKET"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"subscript_list"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RBRACKET"#.to_string(),
                        },
                    ],
                },
                line_number: 489,
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
                line_number: 490,
            },
            GrammarRule {
                name: r#"subscript_list"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"subscript"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COMMA"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"subscript"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 499,
            },
            GrammarRule {
                name: r#"subscript"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference {
                            name: r#"STAR"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"range_subscript"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 507,
            },
            GrammarRule {
                name: r#"range_subscript"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"COLON"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"range_end"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COLON"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expr"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 515,
            },
            GrammarRule {
                name: r#"range_end"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference {
                            name: r#"STAR"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 516,
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
                line_number: 527,
            },
            GrammarRule {
                name: r#"arg"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::RuleReference {
                            name: r#"keyword_arg"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"bool_keyword_arg"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 535,
            },
            GrammarRule {
                name: r#"keyword_arg"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"EQUALS"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                    ],
                },
                line_number: 542,
            },
            GrammarRule {
                name: r#"bool_keyword_arg"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"SLASH"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                    ],
                },
                line_number: 549,
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
                            name: r#"array_literal"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"group"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                    ],
                },
                line_number: 555,
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
                line_number: 561,
            },
            GrammarRule {
                name: r#"array_literal"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"LBRACKET"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"array_elements"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RBRACKET"#.to_string(),
                        },
                    ],
                },
                line_number: 571,
            },
            GrammarRule {
                name: r#"array_elements"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"expr"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COMMA"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expr"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 572,
            },
        ],
        version: 0,
    }
}
