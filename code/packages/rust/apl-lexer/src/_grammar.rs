// AUTO-GENERATED FILE — DO NOT EDIT
// Source: apl.tokens
// Regenerate with: grammar-tools compile-tokens apl.tokens
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
                name: r#"OUTER"#.to_string(),
                pattern: r#"∘."#.to_string(),
                is_regex: false,
                line_number: 40,
                alias: None,
            },
            TokenDefinition {
                name: r#"ARROW"#.to_string(),
                pattern: r#"←"#.to_string(),
                is_regex: false,
                line_number: 51,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 56,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 57,
                alias: None,
            },
            TokenDefinition {
                name: r#"TIMES"#.to_string(),
                pattern: r#"×"#.to_string(),
                is_regex: false,
                line_number: 58,
                alias: None,
            },
            TokenDefinition {
                name: r#"DIVIDE"#.to_string(),
                pattern: r#"÷"#.to_string(),
                is_regex: false,
                line_number: 59,
                alias: None,
            },
            TokenDefinition {
                name: r#"CEILING"#.to_string(),
                pattern: r#"⌈"#.to_string(),
                is_regex: false,
                line_number: 60,
                alias: None,
            },
            TokenDefinition {
                name: r#"FLOOR"#.to_string(),
                pattern: r#"⌊"#.to_string(),
                is_regex: false,
                line_number: 61,
                alias: None,
            },
            TokenDefinition {
                name: r#"RHO"#.to_string(),
                pattern: r#"⍴"#.to_string(),
                is_regex: false,
                line_number: 62,
                alias: None,
            },
            TokenDefinition {
                name: r#"IOTA"#.to_string(),
                pattern: r#"⍳"#.to_string(),
                is_regex: false,
                line_number: 63,
                alias: None,
            },
            TokenDefinition {
                name: r#"RAVEL"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 64,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQ"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 66,
                alias: None,
            },
            TokenDefinition {
                name: r#"NE"#.to_string(),
                pattern: r#"≠"#.to_string(),
                is_regex: false,
                line_number: 67,
                alias: None,
            },
            TokenDefinition {
                name: r#"LT"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 68,
                alias: None,
            },
            TokenDefinition {
                name: r#"LE"#.to_string(),
                pattern: r#"≤"#.to_string(),
                is_regex: false,
                line_number: 69,
                alias: None,
            },
            TokenDefinition {
                name: r#"GE"#.to_string(),
                pattern: r#"≥"#.to_string(),
                is_regex: false,
                line_number: 70,
                alias: None,
            },
            TokenDefinition {
                name: r#"GT"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 71,
                alias: None,
            },
            TokenDefinition {
                name: r#"REDUCE"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 81,
                alias: None,
            },
            TokenDefinition {
                name: r#"SCAN"#.to_string(),
                pattern: r#"\"#.to_string(),
                is_regex: false,
                line_number: 82,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 84,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 85,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"¯?[0-9]+(\.[0-9]+)?([Ee]¯?[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 101,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[A-Za-z][A-Za-z0-9]*"#.to_string(),
                is_regex: true,
                line_number: 102,
                alias: None,
            },
            TokenDefinition {
                name: r#"NEWLINE"#.to_string(),
                pattern: r#"\r?\n"#.to_string(),
                is_regex: true,
                line_number: 117,
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
                line_number: 120,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMENT"#.to_string(),
                pattern: r#"⍝[^\r\n]*"#.to_string(),
                is_regex: true,
                line_number: 121,
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
