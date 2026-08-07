// AUTO-GENERATED FILE — DO NOT EDIT
// Source: maple.tokens
// Regenerate with: grammar-tools compile-tokens maple.tokens
//
// This file embeds a TokenGrammar as native Rust data structures.
// Call `token_grammar()` instead of reading and parsing the .tokens file.

#[allow(unused_imports)]
use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
#[allow(unused_imports)]
use std::collections::HashMap;

pub fn token_grammar() -> TokenGrammar {
    TokenGrammar {
        definitions: vec![
            TokenDefinition {
                name: r#"ASSIGN"#.to_string(),
                pattern: r#":="#.to_string(),
                is_regex: false,
                line_number: 69,
                alias: None,
            },
            TokenDefinition {
                name: r#"ARROW"#.to_string(),
                pattern: r#"->"#.to_string(),
                is_regex: false,
                line_number: 78,
                alias: None,
            },
            TokenDefinition {
                name: r#"NEQ"#.to_string(),
                pattern: r#"<>"#.to_string(),
                is_regex: false,
                line_number: 85,
                alias: None,
            },
            TokenDefinition {
                name: r#"LE"#.to_string(),
                pattern: r#"<="#.to_string(),
                is_regex: false,
                line_number: 87,
                alias: None,
            },
            TokenDefinition {
                name: r#"GE"#.to_string(),
                pattern: r#">="#.to_string(),
                is_regex: false,
                line_number: 88,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 94,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 95,
                alias: None,
            },
            TokenDefinition {
                name: r#"TIMES"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 96,
                alias: None,
            },
            TokenDefinition {
                name: r#"SLASH"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 97,
                alias: None,
            },
            TokenDefinition {
                name: r#"CARET"#.to_string(),
                pattern: r#"^"#.to_string(),
                is_regex: false,
                line_number: 104,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQ"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 110,
                alias: None,
            },
            TokenDefinition {
                name: r#"LESS"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 111,
                alias: None,
            },
            TokenDefinition {
                name: r#"GREATER"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 112,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 114,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 115,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 125,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 126,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 137,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 138,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 144,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMI"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 154,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 155,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 170,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z][a-zA-Z0-9]*"#.to_string(),
                is_regex: true,
                line_number: 171,
                alias: None,
            },
        ],
        keywords: vec![r#"and"#.to_string(), r#"or"#.to_string(), r#"not"#.to_string(), r#"if"#.to_string(), r#"then"#.to_string(), r#"elif"#.to_string(), r#"else"#.to_string(), r#"end"#.to_string(), r#"fi"#.to_string(), r#"true"#.to_string(), r#"false"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 185,
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
        start_mode: None,
        transitions: vec![],
    }
}
