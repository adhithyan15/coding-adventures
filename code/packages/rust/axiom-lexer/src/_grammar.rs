// AUTO-GENERATED FILE — DO NOT EDIT
// Source: axiom.tokens
// Regenerate with: grammar-tools compile-tokens axiom.tokens
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
                line_number: 79,
                alias: None,
            },
            TokenDefinition {
                name: r#"COERCE"#.to_string(),
                pattern: r#"::"#.to_string(),
                is_regex: false,
                line_number: 88,
                alias: None,
            },
            TokenDefinition {
                name: r#"DEFINE"#.to_string(),
                pattern: r#"=="#.to_string(),
                is_regex: false,
                line_number: 97,
                alias: None,
            },
            TokenDefinition {
                name: r#"NE"#.to_string(),
                pattern: r#"~="#.to_string(),
                is_regex: false,
                line_number: 105,
                alias: None,
            },
            TokenDefinition {
                name: r#"LE"#.to_string(),
                pattern: r#"<="#.to_string(),
                is_regex: false,
                line_number: 107,
                alias: None,
            },
            TokenDefinition {
                name: r#"GE"#.to_string(),
                pattern: r#">="#.to_string(),
                is_regex: false,
                line_number: 108,
                alias: None,
            },
            TokenDefinition {
                name: r#"POW"#.to_string(),
                pattern: r#"**"#.to_string(),
                is_regex: false,
                line_number: 115,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 121,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 122,
                alias: None,
            },
            TokenDefinition {
                name: r#"TIMES"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 123,
                alias: None,
            },
            TokenDefinition {
                name: r#"SLASH"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 124,
                alias: None,
            },
            TokenDefinition {
                name: r#"CARET"#.to_string(),
                pattern: r#"^"#.to_string(),
                is_regex: false,
                line_number: 131,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQ"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 142,
                alias: None,
            },
            TokenDefinition {
                name: r#"LESS"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 143,
                alias: None,
            },
            TokenDefinition {
                name: r#"GREATER"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 144,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 146,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 147,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 153,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 154,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 156,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMI"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 162,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 177,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 214,
                alias: None,
            },
            TokenDefinition {
                name: r#"STRING"#.to_string(),
                pattern: r#""[^"]*""#.to_string(),
                is_regex: true,
                line_number: 215,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z][a-zA-Z0-9]*"#.to_string(),
                is_regex: true,
                line_number: 216,
                alias: None,
            },
        ],
        keywords: vec![r#"if"#.to_string(), r#"then"#.to_string(), r#"else"#.to_string(), r#"has"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 248,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMENT"#.to_string(),
                pattern: r#"--[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 249,
                alias: None,
            },
        ],
        reserved_keywords: vec![],
        escapes: Some(r#"none"#.to_string()),
        error_definitions: vec![
            TokenDefinition {
                name: r#"UNKNOWN"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: true,
                line_number: 293,
                alias: None,
            },
        ],
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
