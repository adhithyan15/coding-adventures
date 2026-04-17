// AUTO-GENERATED FILE — DO NOT EDIT
// Source: json.tokens
// Regenerate with: grammar-tools compile-tokens json.tokens
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
                name: r#"STRING"#.to_string(),
                pattern: r#""([^"\\]|\\["\\\x2fbfnrt]|\\u[0-9a-fA-F]{4})*""#.to_string(),
                is_regex: true,
                line_number: 30,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"-?[0-9]+\.?[0-9]*[eE]?[-+]?[0-9]*"#.to_string(),
                is_regex: true,
                line_number: 37,
                alias: None,
            },
            TokenDefinition {
                name: r#"TRUE"#.to_string(),
                pattern: r#"true"#.to_string(),
                is_regex: false,
                line_number: 41,
                alias: None,
            },
            TokenDefinition {
                name: r#"FALSE"#.to_string(),
                pattern: r#"false"#.to_string(),
                is_regex: false,
                line_number: 42,
                alias: None,
            },
            TokenDefinition {
                name: r#"NULL"#.to_string(),
                pattern: r#"null"#.to_string(),
                is_regex: false,
                line_number: 43,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 49,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 50,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 51,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 52,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 53,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 54,
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
                line_number: 65,
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
