// AUTO-GENERATED FILE — DO NOT EDIT
// Source: macsyma.grammar
// Regenerate with: grammar-tools compile-grammar macsyma.grammar
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
                        name: r#"statement"#.to_string(),
                    }),
                },
                line_number: 31,
            },
            GrammarRule {
                name: r#"statement"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                        GrammarElement::Group {
                            element: Box::new(GrammarElement::Alternation {
                                choices: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"SEMI"#.to_string(),
                                    },
                                    GrammarElement::TokenReference {
                                        name: r#"DOLLAR"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 33,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
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
                            name: r#"block_expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"return_expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"assign"#.to_string(),
                        },
                    ],
                },
                line_number: 44,
            },
            GrammarRule {
                name: r#"if_expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"if"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"then"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Literal {
                                        value: r#"elseif"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expression"#.to_string(),
                                    },
                                    GrammarElement::Literal {
                                        value: r#"then"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expression"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Literal {
                                        value: r#"else"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expression"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 54,
            },
            GrammarRule {
                name: r#"for_expr"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::RuleReference {
                            name: r#"for_each_expr"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"for_range_expr"#.to_string(),
                        },
                    ],
                },
                line_number: 67,
            },
            GrammarRule {
                name: r#"for_each_expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"for"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"in"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"do"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                    ],
                },
                line_number: 69,
            },
            GrammarRule {
                name: r#"for_range_expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"for"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Literal {
                                        value: r#":"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expression"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Literal {
                                        value: r#"step"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expression"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::Group {
                            element: Box::new(GrammarElement::Alternation {
                                choices: vec![
                                    GrammarElement::Literal {
                                        value: r#"thru"#.to_string(),
                                    },
                                    GrammarElement::Literal {
                                        value: r#"while"#.to_string(),
                                    },
                                    GrammarElement::Literal {
                                        value: r#"unless"#.to_string(),
                                    },
                                ],
                            }),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"do"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                    ],
                },
                line_number: 71,
            },
            GrammarRule {
                name: r#"while_expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"while"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"do"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                    ],
                },
                line_number: 76,
            },
            GrammarRule {
                name: r#"block_expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"block"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"("#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"arglist"#.to_string(),
                            }),
                        },
                        GrammarElement::Literal {
                            value: r#")"#.to_string(),
                        },
                    ],
                },
                line_number: 82,
            },
            GrammarRule {
                name: r#"return_expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"return"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"("#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#")"#.to_string(),
                        },
                    ],
                },
                line_number: 87,
            },
            GrammarRule {
                name: r#"assign"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"logical_or"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Group {
                                        element: Box::new(GrammarElement::Alternation {
                                            choices: vec![
                                                GrammarElement::TokenReference {
                                                    name: r#"COLON"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"COLONEQ"#.to_string(),
                                                },
                                            ],
                                        }),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"assign"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 92,
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
                                    GrammarElement::Literal {
                                        value: r#"or"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"logical_and"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 97,
            },
            GrammarRule {
                name: r#"logical_and"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"logical_not"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Literal {
                                        value: r#"and"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"logical_not"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 98,
            },
            GrammarRule {
                name: r#"logical_not"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::Sequence {
                            elements: vec![
                                GrammarElement::Literal {
                                    value: r#"not"#.to_string(),
                                },
                                GrammarElement::RuleReference {
                                    name: r#"logical_not"#.to_string(),
                                },
                            ],
                        },
                        GrammarElement::RuleReference {
                            name: r#"comparison"#.to_string(),
                        },
                    ],
                },
                line_number: 99,
            },
            GrammarRule {
                name: r#"comparison"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"additive"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::Group {
                                        element: Box::new(GrammarElement::Alternation {
                                            choices: vec![
                                                GrammarElement::TokenReference {
                                                    name: r#"EQ"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"HASH"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"LT"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"GT"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"LEQ"#.to_string(),
                                                },
                                                GrammarElement::TokenReference {
                                                    name: r#"GEQ"#.to_string(),
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
                line_number: 103,
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
                line_number: 105,
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
                line_number: 106,
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
                                                name: r#"MINUS"#.to_string(),
                                            },
                                            GrammarElement::TokenReference {
                                                name: r#"PLUS"#.to_string(),
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
                line_number: 110,
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
                                                    name: r#"STAREQ"#.to_string(),
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
                line_number: 114,
            },
            GrammarRule {
                name: r#"postfix"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"atom"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"LPAREN"#.to_string(),
                                    },
                                    GrammarElement::Optional {
                                        element: Box::new(GrammarElement::RuleReference {
                                            name: r#"arglist"#.to_string(),
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
                line_number: 118,
            },
            GrammarRule {
                name: r#"arglist"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"COMMA"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"expression"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 119,
            },
            GrammarRule {
                name: r#"atom"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference {
                            name: r#"NUMBER"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"STRING"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"NAME"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"true"#.to_string(),
                        },
                        GrammarElement::Literal {
                            value: r#"false"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"group"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"list"#.to_string(),
                        },
                    ],
                },
                line_number: 121,
            },
            GrammarRule {
                name: r#"group"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"LPAREN"#.to_string(),
                        },
                        GrammarElement::RuleReference {
                            name: r#"expression"#.to_string(),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RPAREN"#.to_string(),
                        },
                    ],
                },
                line_number: 129,
            },
            GrammarRule {
                name: r#"list"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::TokenReference {
                            name: r#"LBRACKET"#.to_string(),
                        },
                        GrammarElement::Optional {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"arglist"#.to_string(),
                            }),
                        },
                        GrammarElement::TokenReference {
                            name: r#"RBRACKET"#.to_string(),
                        },
                    ],
                },
                line_number: 130,
            },
        ],
        version: 2,
    }
}
