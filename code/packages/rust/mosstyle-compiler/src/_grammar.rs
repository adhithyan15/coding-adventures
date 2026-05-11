// AUTO-GENERATED FILE — DO NOT EDIT
// Source: mosstyle.tokens + mosstyle.grammar
// Regenerate with: grammar-tools -f compile-tokens mosstyle.tokens
//                  grammar-tools    compile-grammar mosstyle.grammar
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
// Token grammar (from mosstyle.tokens)
// ===========================================================================

pub fn token_grammar() -> TokenGrammar {
    TokenGrammar {
        definitions: vec![
            TokenDefinition {
                name: r#"STRING"#.to_string(),
                pattern: r#""([^"\
]|\.)*""#.to_string(),
                is_regex: true,
                line_number: 29,
                alias: None,
            },
            TokenDefinition {
                name: r#"DIMENSION"#.to_string(),
                pattern: r#"[0-9]+(\.[0-9]+)?(px|rem|em|pt|ms|s|deg|%)"#.to_string(),
                is_regex: true,
                line_number: 30,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+(\.[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 31,
                alias: None,
            },
            TokenDefinition {
                name: r#"HASH_COLOR"#.to_string(),
                pattern: r#"#[0-9a-fA-F]{3,8}"#.to_string(),
                is_regex: true,
                line_number: 32,
                alias: None,
            },
            TokenDefinition {
                name: r#"TOKEN_REF"#.to_string(),
                pattern: r#"\$[a-z][a-z0-9]*(-[a-z][a-z0-9]*)*"#.to_string(),
                is_regex: true,
                line_number: 38,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z][a-zA-Z0-9]*(-[a-zA-Z][a-zA-Z0-9]*)*"#.to_string(),
                is_regex: true,
                line_number: 55,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 61,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 62,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 63,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 64,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMICOLON"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 65,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 66,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 67,
                alias: None,
            },
        ],
        keywords: vec![r#"style"#.to_string(), r#"part"#.to_string(), r#"state"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"LINE_COMMENT"#.to_string(),
                pattern: r#"\/\/[^
]*"#.to_string(),
                is_regex: true,
                line_number: 19,
                alias: None,
            },
            TokenDefinition {
                name: r#"BLOCK_COMMENT"#.to_string(),
                pattern: r#"\/\*[\s\S]*?\*\/"#.to_string(),
                is_regex: true,
                line_number: 20,
                alias: None,
            },
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ 	
]+"#.to_string(),
                is_regex: true,
                line_number: 21,
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
    }
}

// ===========================================================================
// Parser grammar (from mosstyle.grammar)
// ===========================================================================

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"file"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"style_def"#.to_string() },
            line_number: 36,
        },
        GrammarRule {
            name: r#"style_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"part_def"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 49,
        },
        GrammarRule {
            name: r#"part_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"part_item"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 56,
        },
        GrammarRule {
            name: r#"part_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"state_block"#.to_string() },
                GrammarElement::RuleReference { name: r#"property_decl"#.to_string() },
            ] },
            line_number: 66,
        },
        GrammarRule {
            name: r#"state_block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"property_decl"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 76,
        },
        GrammarRule {
            name: r#"property_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"style_value"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 83,
        },
        GrammarRule {
            name: r#"style_value"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"TOKEN_REF"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH_COLOR"#.to_string() },
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 96,
        },
    ],
        version: 1,
    }
}
