// AUTO-GENERATED FILE — DO NOT EDIT
// Source: lisp.tokens
// Regenerate with: grammar-tools compile-tokens lisp.tokens
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
                name: r#"NUMBER"#.to_string(),
                pattern: r#"-?[0-9]+"#.to_string(),
                is_regex: true,
                line_number: 11,
                alias: None,
            },
            TokenDefinition {
                name: r#"SYMBOL"#.to_string(),
                pattern: r#"[a-zA-Z_+\-*\/=<>!?&][a-zA-Z0-9_+\-*\/=<>!?&]*"#.to_string(),
                is_regex: true,
                line_number: 12,
                alias: None,
            },
            TokenDefinition {
                name: r#"STRING"#.to_string(),
                pattern: r#""([^"\\]|\\.)*""#.to_string(),
                is_regex: true,
                line_number: 13,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 14,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 15,
                alias: None,
            },
            TokenDefinition {
                name: r#"QUOTE"#.to_string(),
                pattern: r#"'"#.to_string(),
                is_regex: false,
                line_number: 16,
                alias: None,
            },
            TokenDefinition {
                name: r#"DOT"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: false,
                line_number: 17,
                alias: None,
            },
        ],
        keywords: vec![],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 8,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMENT"#.to_string(),
                pattern: r#";[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 9,
                alias: None,
            },
        ],
        reserved_keywords: vec![],
        escapes: Some(r#"none"#.to_string()),
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
