// AUTO-GENERATED FILE — DO NOT EDIT
// Source: iso.tokens
// Regenerate with: grammar-tools compile-tokens iso.tokens
//
// This file embeds a TokenGrammar as native Rust data structures.
// Call `token_grammar()` instead of reading and parsing the .tokens file.

#[allow(unused_imports)]
use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
#[allow(unused_imports)]
use std::collections::HashMap;

pub fn token_grammar() -> TokenGrammar {
    TokenGrammar {
        definitions: vec![
            TokenDefinition {
                name: r#"DCG"#.to_string(),
                pattern: r#"-->"#.to_string(),
                is_regex: false,
                line_number: 32,
                alias: None,
            },
            TokenDefinition {
                name: r#"QUERY"#.to_string(),
                pattern: r#"?-"#.to_string(),
                is_regex: false,
                line_number: 33,
                alias: None,
            },
            TokenDefinition {
                name: r#"RULE"#.to_string(),
                pattern: r#":-"#.to_string(),
                is_regex: false,
                line_number: 34,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAF"#.to_string(),
                pattern: r#"\+"#.to_string(),
                is_regex: false,
                line_number: 35,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 38,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 39,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 40,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 41,
                alias: None,
            },
            TokenDefinition {
                name: r#"LCURLY"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 42,
                alias: None,
            },
            TokenDefinition {
                name: r#"RCURLY"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 43,
                alias: None,
            },
            TokenDefinition {
                name: r#"BAR"#.to_string(),
                pattern: r#"|"#.to_string(),
                is_regex: false,
                line_number: 44,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 45,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMICOLON"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 46,
                alias: None,
            },
            TokenDefinition {
                name: r#"CUT"#.to_string(),
                pattern: r#"!"#.to_string(),
                is_regex: false,
                line_number: 47,
                alias: None,
            },
            TokenDefinition {
                name: r#"DOT"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: false,
                line_number: 48,
                alias: None,
            },
            TokenDefinition {
                name: r#"FLOAT"#.to_string(),
                pattern: r#"[0-9]+\.[0-9]+([eE][-+]?[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 51,
                alias: None,
            },
            TokenDefinition {
                name: r#"INTEGER"#.to_string(),
                pattern: r#"[0-9]+"#.to_string(),
                is_regex: true,
                line_number: 52,
                alias: None,
            },
            TokenDefinition {
                name: r#"STRING"#.to_string(),
                pattern: r#""([^"\\]|\\.)*""#.to_string(),
                is_regex: true,
                line_number: 55,
                alias: None,
            },
            TokenDefinition {
                name: r#"QUOTED_ATOM"#.to_string(),
                pattern: r#"'([^'\\]|\\.)*'"#.to_string(),
                is_regex: true,
                line_number: 56,
                alias: Some(r#"ATOM"#.to_string()),
            },
            TokenDefinition {
                name: r#"VARIABLE"#.to_string(),
                pattern: r#"(?:[A-Z][A-Za-z0-9_]*|_[A-Za-z0-9_]+)"#.to_string(),
                is_regex: true,
                line_number: 64,
                alias: None,
            },
            TokenDefinition {
                name: r#"ANON_VAR"#.to_string(),
                pattern: r#"_"#.to_string(),
                is_regex: true,
                line_number: 63,
                alias: None,
            },
            TokenDefinition {
                name: r#"ATOM"#.to_string(),
                pattern: r#"[a-z][A-Za-z0-9_]*"#.to_string(),
                is_regex: true,
                line_number: 65,
                alias: None,
            },
            TokenDefinition {
                name: r#"ATOM_SYMBOLIC"#.to_string(),
                pattern: r#"[#$&*+\-\/:<=>?@\\^~]+"#.to_string(),
                is_regex: true,
                line_number: 70,
                alias: Some(r#"ATOM"#.to_string()),
            },
        ],
        keywords: vec![],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 22,
                alias: None,
            },
            TokenDefinition {
                name: r#"LINE_COMMENT"#.to_string(),
                pattern: r#"%[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 23,
                alias: None,
            },
            TokenDefinition {
                name: r#"BLOCK_COMMENT"#.to_string(),
                pattern: r#"\/\*[\s\S]*?\*\/"#.to_string(),
                is_regex: true,
                line_number: 26,
                alias: None,
            },
        ],
        reserved_keywords: vec![],
        escapes: None,
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
