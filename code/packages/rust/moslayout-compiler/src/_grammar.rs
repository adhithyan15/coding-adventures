// AUTO-GENERATED FILE — DO NOT EDIT
// Source: moslayout.tokens + moslayout.grammar
// Regenerate with: grammar-tools -f compile-tokens moslayout.tokens
//                  grammar-tools    compile-grammar moslayout.grammar
//
// This file embeds both the token grammar and the parser grammar as native
// Rust data structures.  Call `token_grammar()` or `parser_grammar()` instead
// of reading and parsing the .tokens / .grammar files at runtime.
//
// The -f flag is needed for compile-tokens because 'escapes: standard' is a
// semantic directive consumed by the lexer runtime, not a grammar-tools concept.

#[allow(unused_imports)]
use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};
#[allow(unused_imports)]
use std::collections::HashMap;

// ===========================================================================
// Token grammar (from moslayout.tokens)
// ===========================================================================

pub fn token_grammar() -> TokenGrammar {
    TokenGrammar {
        definitions: vec![
            TokenDefinition {
                name: r#"STRING"#.to_string(),
                pattern: r#""([^"\\\n]|\\.)*""#.to_string(),
                is_regex: true,
                line_number: 36,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+(\.[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 38,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z][a-zA-Z0-9]*(-[a-zA-Z][a-zA-Z0-9]*)*"#.to_string(),
                is_regex: true,
                line_number: 71,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 77,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 78,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 79,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 80,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 81,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 82,
                alias: None,
            },
            TokenDefinition {
                name: r#"DOUBLE_COLON"#.to_string(),
                pattern: r#"::"#.to_string(),
                is_regex: false,
                line_number: 86,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 87,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 88,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQ"#.to_string(),
                pattern: r#"=="#.to_string(),
                is_regex: false,
                line_number: 110,
                alias: None,
            },
            TokenDefinition {
                name: r#"NEQ"#.to_string(),
                pattern: r#"!="#.to_string(),
                is_regex: false,
                line_number: 111,
                alias: None,
            },
            TokenDefinition {
                name: r#"LE"#.to_string(),
                pattern: r#"<="#.to_string(),
                is_regex: false,
                line_number: 112,
                alias: None,
            },
            TokenDefinition {
                name: r#"GE"#.to_string(),
                pattern: r#">="#.to_string(),
                is_regex: false,
                line_number: 113,
                alias: None,
            },
            TokenDefinition {
                name: r#"LT"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 114,
                alias: None,
            },
            TokenDefinition {
                name: r#"GT"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 115,
                alias: None,
            },
            TokenDefinition {
                name: r#"AND"#.to_string(),
                pattern: r#"&&"#.to_string(),
                is_regex: false,
                line_number: 116,
                alias: None,
            },
            TokenDefinition {
                name: r#"OR"#.to_string(),
                pattern: r#"||"#.to_string(),
                is_regex: false,
                line_number: 117,
                alias: None,
            },
            TokenDefinition {
                name: r#"NOT"#.to_string(),
                pattern: r#"!"#.to_string(),
                is_regex: false,
                line_number: 118,
                alias: None,
            },
            TokenDefinition {
                name: r#"DOT"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: false,
                line_number: 119,
                alias: None,
            },
        ],
        keywords: vec![r#"layout"#.to_string(), r#"slot"#.to_string(), r#"emit"#.to_string(), r#"pkg"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"LINE_COMMENT"#.to_string(),
                pattern: r#"\/\/[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 20,
                alias: None,
            },
            TokenDefinition {
                name: r#"BLOCK_COMMENT"#.to_string(),
                pattern: r#"\/\*[\s\S]*?\*\/"#.to_string(),
                is_regex: true,
                line_number: 21,
                alias: None,
            },
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 22,
                alias: None,
            },
        ],
        reserved_keywords: vec![],
        escapes: Some(r#"standard"#.to_string()),
        error_definitions: vec![],
        groups: HashMap::new(),
        case_sensitive: true,
        version: 1,
        case_insensitive: false,
        context_keywords: vec![],
        soft_keywords: vec![],
        layout_keywords: vec![],
        start_mode: None,
        transitions: vec![],
    }
}

// ===========================================================================
// Parser grammar (from moslayout.grammar)
// ===========================================================================


pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"file"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"layout_def"#.to_string() },
            line_number: 36,
        },
        GrammarRule {
            name: r#"layout_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"node"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 53,
        },
        GrammarRule {
            name: r#"node"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"part_name"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"prop_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"node"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                    ] }) },
            ] },
            line_number: 76,
        },
        GrammarRule {
            name: r#"qualified_name"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
            ] },
            line_number: 85,
        },
        GrammarRule {
            name: r#"part_name"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 97,
        },
        GrammarRule {
            name: r#"prop_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"prop"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"prop"#.to_string() },
                    ] }) },
            ] },
            line_number: 126,
        },
        GrammarRule {
            name: r#"prop"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"prop_value"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
            ] },
            line_number: 140,
        },
        GrammarRule {
            name: r#"prop_value"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            line_number: 171,
        },
        GrammarRule {
            name: r#"expr"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
            line_number: 173,
        },
        GrammarRule {
            name: r#"or_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"OR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 174,
        },
        GrammarRule {
            name: r#"and_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"eq_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"AND"#.to_string() },
                        GrammarElement::RuleReference { name: r#"eq_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 175,
        },
        GrammarRule {
            name: r#"eq_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"rel_expr"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                                GrammarElement::TokenReference { name: r#"NEQ"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"rel_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 176,
        },
        GrammarRule {
            name: r#"rel_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                    ] }) },
            ] },
            line_number: 177,
        },
        GrammarRule {
            name: r#"unary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NOT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"postfix"#.to_string() },
            ] },
            line_number: 178,
        },
        GrammarRule {
            name: r#"postfix"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                            GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                        ] },
                    ] }) },
            ] },
            line_number: 179,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 180,
        },
    ],
        version: 1,
    }
}
