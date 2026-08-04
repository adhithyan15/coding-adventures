// AUTO-GENERATED FILE — DO NOT EDIT
// Source: j.tokens
// Regenerate with: grammar-tools compile-tokens j.tokens
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
                name: r#"FLOOR"#.to_string(),
                pattern: r#"<."#.to_string(),
                is_regex: false,
                line_number: 65,
                alias: None,
            },
            TokenDefinition {
                name: r#"CEILING"#.to_string(),
                pattern: r#">."#.to_string(),
                is_regex: false,
                line_number: 66,
                alias: None,
            },
            TokenDefinition {
                name: r#"LE"#.to_string(),
                pattern: r#"<:"#.to_string(),
                is_regex: false,
                line_number: 70,
                alias: None,
            },
            TokenDefinition {
                name: r#"GE"#.to_string(),
                pattern: r#">:"#.to_string(),
                is_regex: false,
                line_number: 71,
                alias: None,
            },
            TokenDefinition {
                name: r#"NE"#.to_string(),
                pattern: r#"~:"#.to_string(),
                is_regex: false,
                line_number: 72,
                alias: None,
            },
            TokenDefinition {
                name: r#"IDOT"#.to_string(),
                pattern: r#"i."#.to_string(),
                is_regex: false,
                line_number: 77,
                alias: None,
            },
            TokenDefinition {
                name: r#"ASSIGN_LOCAL"#.to_string(),
                pattern: r#"=."#.to_string(),
                is_regex: false,
                line_number: 82,
                alias: None,
            },
            TokenDefinition {
                name: r#"ASSIGN_GLOBAL"#.to_string(),
                pattern: r#"=:"#.to_string(),
                is_regex: false,
                line_number: 83,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 96,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 97,
                alias: None,
            },
            TokenDefinition {
                name: r#"STAR"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 98,
                alias: None,
            },
            TokenDefinition {
                name: r#"PERCENT"#.to_string(),
                pattern: r#"%"#.to_string(),
                is_regex: false,
                line_number: 99,
                alias: None,
            },
            TokenDefinition {
                name: r#"CARET"#.to_string(),
                pattern: r#"^"#.to_string(),
                is_regex: false,
                line_number: 100,
                alias: None,
            },
            TokenDefinition {
                name: r#"DOLLAR"#.to_string(),
                pattern: r#"$"#.to_string(),
                is_regex: false,
                line_number: 101,
                alias: None,
            },
            TokenDefinition {
                name: r#"RAVEL"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 102,
                alias: None,
            },
            TokenDefinition {
                name: r#"HASH"#.to_string(),
                pattern: r#"#"#.to_string(),
                is_regex: false,
                line_number: 103,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQ"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 105,
                alias: None,
            },
            TokenDefinition {
                name: r#"LT"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 106,
                alias: None,
            },
            TokenDefinition {
                name: r#"GT"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 107,
                alias: None,
            },
            TokenDefinition {
                name: r#"REDUCE"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 119,
                alias: None,
            },
            TokenDefinition {
                name: r#"SCAN"#.to_string(),
                pattern: r#"\"#.to_string(),
                is_regex: false,
                line_number: 120,
                alias: None,
            },
            TokenDefinition {
                name: r#"AT"#.to_string(),
                pattern: r#"@"#.to_string(),
                is_regex: false,
                line_number: 121,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 123,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 124,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"_?[0-9]+(\.[0-9]+)?([Ee]_?[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 139,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[A-Za-z][A-Za-z0-9]*"#.to_string(),
                is_regex: true,
                line_number: 140,
                alias: None,
            },
            TokenDefinition {
                name: r#"NEWLINE"#.to_string(),
                pattern: r#"\r?\n"#.to_string(),
                is_regex: true,
                line_number: 158,
                alias: None,
            },
        ],
        keywords: vec![],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t]+"#.to_string(),
                is_regex: true,
                line_number: 161,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMENT"#.to_string(),
                pattern: r#"NB\.[^\r\n]*"#.to_string(),
                is_regex: true,
                line_number: 162,
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
        start_mode: None,
        transitions: vec![],
    }
}
