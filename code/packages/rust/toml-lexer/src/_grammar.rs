// AUTO-GENERATED FILE — DO NOT EDIT
// Source: toml.tokens
// Regenerate with: grammar-tools compile-tokens toml.tokens
//
// This file embeds a TokenGrammar as native Rust data structures.
// Call `token_grammar()` instead of reading and parsing the .tokens file.

use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
use std::collections::HashMap;

pub fn token_grammar() -> TokenGrammar {
    TokenGrammar {
        definitions: vec![
            TokenDefinition {
                name: r#"ML_BASIC_STRING"#.to_string(),
                pattern: r#""""([^\\]|\\(.|\n)|\n)*?""""#.to_string(),
                is_regex: true,
                line_number: 60,
                alias: None,
            },
            TokenDefinition {
                name: r#"ML_LITERAL_STRING"#.to_string(),
                pattern: r#"'''[\s\S]*?'''"#.to_string(),
                is_regex: true,
                line_number: 61,
                alias: None,
            },
            TokenDefinition {
                name: r#"BASIC_STRING"#.to_string(),
                pattern: r#""([^"\\\n]|\\.)*""#.to_string(),
                is_regex: true,
                line_number: 70,
                alias: None,
            },
            TokenDefinition {
                name: r#"LITERAL_STRING"#.to_string(),
                pattern: r#"'[^'\n]*'"#.to_string(),
                is_regex: true,
                line_number: 71,
                alias: None,
            },
            TokenDefinition {
                name: r#"OFFSET_DATETIME"#.to_string(),
                pattern: r#"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})"#.to_string(),
                is_regex: true,
                line_number: 91,
                alias: None,
            },
            TokenDefinition {
                name: r#"LOCAL_DATETIME"#.to_string(),
                pattern: r#"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?"#.to_string(),
                is_regex: true,
                line_number: 92,
                alias: None,
            },
            TokenDefinition {
                name: r#"LOCAL_DATE"#.to_string(),
                pattern: r#"\d{4}-\d{2}-\d{2}"#.to_string(),
                is_regex: true,
                line_number: 93,
                alias: None,
            },
            TokenDefinition {
                name: r#"LOCAL_TIME"#.to_string(),
                pattern: r#"\d{2}:\d{2}:\d{2}(\.\d+)?"#.to_string(),
                is_regex: true,
                line_number: 94,
                alias: None,
            },
            TokenDefinition {
                name: r#"FLOAT_SPECIAL"#.to_string(),
                pattern: r#"[+-]?(inf|nan)"#.to_string(),
                is_regex: true,
                line_number: 109,
                alias: Some(r#"FLOAT"#.to_string()),
            },
            TokenDefinition {
                name: r#"FLOAT_EXP"#.to_string(),
                pattern: r#"[+-]?([0-9](_?[0-9])*)(\.[0-9](_?[0-9])*)?[eE][+-]?[0-9](_?[0-9])*"#.to_string(),
                is_regex: true,
                line_number: 110,
                alias: Some(r#"FLOAT"#.to_string()),
            },
            TokenDefinition {
                name: r#"FLOAT_DEC"#.to_string(),
                pattern: r#"[+-]?([0-9](_?[0-9])*)\.([0-9](_?[0-9])*)"#.to_string(),
                is_regex: true,
                line_number: 111,
                alias: Some(r#"FLOAT"#.to_string()),
            },
            TokenDefinition {
                name: r#"HEX_INTEGER"#.to_string(),
                pattern: r#"0x[0-9a-fA-F](_?[0-9a-fA-F])*"#.to_string(),
                is_regex: true,
                line_number: 123,
                alias: Some(r#"INTEGER"#.to_string()),
            },
            TokenDefinition {
                name: r#"OCT_INTEGER"#.to_string(),
                pattern: r#"0o[0-7](_?[0-7])*"#.to_string(),
                is_regex: true,
                line_number: 124,
                alias: Some(r#"INTEGER"#.to_string()),
            },
            TokenDefinition {
                name: r#"BIN_INTEGER"#.to_string(),
                pattern: r#"0b[01](_?[01])*"#.to_string(),
                is_regex: true,
                line_number: 125,
                alias: Some(r#"INTEGER"#.to_string()),
            },
            TokenDefinition {
                name: r#"INTEGER"#.to_string(),
                pattern: r#"[+-]?[0-9](_?[0-9])*"#.to_string(),
                is_regex: true,
                line_number: 126,
                alias: None,
            },
            TokenDefinition {
                name: r#"TRUE"#.to_string(),
                pattern: r#"true"#.to_string(),
                is_regex: false,
                line_number: 137,
                alias: None,
            },
            TokenDefinition {
                name: r#"FALSE"#.to_string(),
                pattern: r#"false"#.to_string(),
                is_regex: false,
                line_number: 138,
                alias: None,
            },
            TokenDefinition {
                name: r#"BARE_KEY"#.to_string(),
                pattern: r#"[A-Za-z0-9_-]+"#.to_string(),
                is_regex: true,
                line_number: 152,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQUALS"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 162,
                alias: None,
            },
            TokenDefinition {
                name: r#"DOT"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: false,
                line_number: 163,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 164,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 165,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 166,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 167,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 168,
                alias: None,
            },
        ],
        keywords: vec![],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"COMMENT"#.to_string(),
                pattern: r#"#[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 28,
                alias: None,
            },
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t]+"#.to_string(),
                is_regex: true,
                line_number: 29,
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
            context_keywords: Vec::new(),
    }
}
