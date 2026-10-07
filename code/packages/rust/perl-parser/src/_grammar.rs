// AUTO-GENERATED FILE — DO NOT EDIT
// Source: perl.grammar
// Regenerate with: grammar-tools compile-grammar perl.grammar
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
                        GrammarElement::RuleReference {
                            name: r#"print_statement"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::RuleReference {
                                name: r#"print_statement"#.to_string(),
                            }),
                        },
                    ],
                },
                line_number: 5,
            },
            GrammarRule {
                name: r#"print_statement"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::Literal {
                            value: r#"print"#.to_string(),
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
                        GrammarElement::TokenReference {
                            name: r#"SEMICOLON"#.to_string(),
                        },
                    ],
                },
                line_number: 6,
            },
            GrammarRule {
                name: r#"expr"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"term"#.to_string(),
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
                                        name: r#"term"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 7,
            },
            GrammarRule {
                name: r#"term"#.to_string(),
                body: GrammarElement::Sequence {
                    elements: vec![
                        GrammarElement::RuleReference {
                            name: r#"unary"#.to_string(),
                        },
                        GrammarElement::Repetition {
                            element: Box::new(GrammarElement::Sequence {
                                elements: vec![
                                    GrammarElement::TokenReference {
                                        name: r#"STAR"#.to_string(),
                                    },
                                    GrammarElement::RuleReference {
                                        name: r#"unary"#.to_string(),
                                    },
                                ],
                            }),
                        },
                    ],
                },
                line_number: 8,
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
                            name: r#"atom"#.to_string(),
                        },
                    ],
                },
                line_number: 9,
            },
            GrammarRule {
                name: r#"atom"#.to_string(),
                body: GrammarElement::Alternation {
                    choices: vec![
                        GrammarElement::TokenReference {
                            name: r#"INT"#.to_string(),
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
                line_number: 10,
            },
        ],
        version: 1,
    }
}
