// AUTO-GENERATED FILE — DO NOT EDIT
// Source: derive.tokens
// Regenerate with: grammar-tools compile-tokens derive.tokens
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
                line_number: 43,
                alias: None,
            },
            TokenDefinition {
                name: r#"LE"#.to_string(),
                pattern: r#"<="#.to_string(),
                is_regex: false,
                line_number: 45,
                alias: None,
            },
            TokenDefinition {
                name: r#"GE"#.to_string(),
                pattern: r#">="#.to_string(),
                is_regex: false,
                line_number: 46,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 52,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 53,
                alias: None,
            },
            TokenDefinition {
                name: r#"TIMES"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 54,
                alias: None,
            },
            TokenDefinition {
                name: r#"SLASH"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 55,
                alias: None,
            },
            TokenDefinition {
                name: r#"POWER"#.to_string(),
                pattern: r#"^"#.to_string(),
                is_regex: false,
                line_number: 56,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQ"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 63,
                alias: None,
            },
            TokenDefinition {
                name: r#"LESS"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 64,
                alias: None,
            },
            TokenDefinition {
                name: r#"GREATER"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 65,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 67,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 68,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 69,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 70,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 72,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMI"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 77,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 90,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z][a-zA-Z0-9]*"#.to_string(),
                is_regex: true,
                line_number: 91,
                alias: None,
            },
            TokenDefinition {
                name: r#"NEWLINE"#.to_string(),
                pattern: r#"\r?\n"#.to_string(),
                is_regex: true,
                line_number: 104,
                alias: None,
            },
        ],
        keywords: vec![r#"AND"#.to_string(), r#"OR"#.to_string(), r#"NOT"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t]+"#.to_string(),
                is_regex: true,
                line_number: 107,
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
