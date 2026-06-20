// AUTO-GENERATED FILE — DO NOT EDIT
// Source: adj_lang.tokens
// Regenerate with: grammar-tools compile-tokens adj_lang.tokens
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
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 36,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 37,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 38,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 39,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACK"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 40,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACK"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 41,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 42,
                alias: None,
            },
            TokenDefinition {
                name: r#"QUESTION"#.to_string(),
                pattern: r#"?"#.to_string(),
                is_regex: false,
                line_number: 43,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 44,
                alias: None,
            },
            TokenDefinition {
                name: r#"GE"#.to_string(),
                pattern: r#">="#.to_string(),
                is_regex: false,
                line_number: 59,
                alias: None,
            },
            TokenDefinition {
                name: r#"LE"#.to_string(),
                pattern: r#"<="#.to_string(),
                is_regex: false,
                line_number: 60,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQEQ"#.to_string(),
                pattern: r#"=="#.to_string(),
                is_regex: false,
                line_number: 61,
                alias: None,
            },
            TokenDefinition {
                name: r#"NE"#.to_string(),
                pattern: r#"!="#.to_string(),
                is_regex: false,
                line_number: 62,
                alias: None,
            },
            TokenDefinition {
                name: r#"GT"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 63,
                alias: None,
            },
            TokenDefinition {
                name: r#"LT"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 64,
                alias: None,
            },
            TokenDefinition {
                name: r#"STRING"#.to_string(),
                pattern: r#""([^"\\]|\\.)*""#.to_string(),
                is_regex: true,
                line_number: 80,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"-?(?:\.[0-9]+|[0-9]+(?:\.[0-9]*)?)(?:[eE][+-]?[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 81,
                alias: None,
            },
            TokenDefinition {
                name: r#"VAR"#.to_string(),
                pattern: r#"\$[A-Za-z_][A-Za-z0-9_]*"#.to_string(),
                is_regex: true,
                line_number: 96,
                alias: None,
            },
            TokenDefinition {
                name: r#"IDENT"#.to_string(),
                pattern: r#"[a-z_][a-z0-9_]*"#.to_string(),
                is_regex: true,
                line_number: 106,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 126,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 127,
                alias: None,
            },
            TokenDefinition {
                name: r#"STAR"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 128,
                alias: None,
            },
            TokenDefinition {
                name: r#"SLASH"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 129,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQUALS"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 130,
                alias: None,
            },
        ],
        keywords: vec![r#"prior"#.to_string(), r#"for"#.to_string(), r#"contributes"#.to_string(), r#"from"#.to_string(), r#"to"#.to_string(), r#"interacts"#.to_string(), r#"when"#.to_string(), r#"and"#.to_string(), r#"observe"#.to_string(), r#"uncertain"#.to_string(), r#"source"#.to_string(), r#"trust"#.to_string(), r#"locator"#.to_string(), r#"consensus"#.to_string(), r#"authoritative"#.to_string(), r#"empirical"#.to_string(), r#"inferred"#.to_string(), r#"unattributed"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 28,
                alias: None,
            },
            TokenDefinition {
                name: r#"LINE_COMMENT"#.to_string(),
                pattern: r#"%[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 29,
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
