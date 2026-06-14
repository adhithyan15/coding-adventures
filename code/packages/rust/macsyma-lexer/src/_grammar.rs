// AUTO-GENERATED FILE — DO NOT EDIT
// Source: macsyma.tokens
// Regenerate with: grammar-tools compile-tokens macsyma.tokens
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
                name: r#"COLONEQ"#.to_string(),
                pattern: r#":="#.to_string(),
                is_regex: false,
                line_number: 48,
                alias: None,
            },
            TokenDefinition {
                name: r#"STAREQ"#.to_string(),
                pattern: r#"**"#.to_string(),
                is_regex: false,
                line_number: 49,
                alias: None,
            },
            TokenDefinition {
                name: r#"LEQ"#.to_string(),
                pattern: r#"<="#.to_string(),
                is_regex: false,
                line_number: 50,
                alias: None,
            },
            TokenDefinition {
                name: r#"GEQ"#.to_string(),
                pattern: r#">="#.to_string(),
                is_regex: false,
                line_number: 51,
                alias: None,
            },
            TokenDefinition {
                name: r#"ARROW"#.to_string(),
                pattern: r#"->"#.to_string(),
                is_regex: false,
                line_number: 52,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 58,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 59,
                alias: None,
            },
            TokenDefinition {
                name: r#"STAR"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 60,
                alias: None,
            },
            TokenDefinition {
                name: r#"SLASH"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 61,
                alias: None,
            },
            TokenDefinition {
                name: r#"CARET"#.to_string(),
                pattern: r#"^"#.to_string(),
                is_regex: false,
                line_number: 62,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 63,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQ"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 64,
                alias: None,
            },
            TokenDefinition {
                name: r#"HASH"#.to_string(),
                pattern: r#"#"#.to_string(),
                is_regex: false,
                line_number: 65,
                alias: None,
            },
            TokenDefinition {
                name: r#"LT"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 66,
                alias: None,
            },
            TokenDefinition {
                name: r#"GT"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 67,
                alias: None,
            },
            TokenDefinition {
                name: r#"BANG"#.to_string(),
                pattern: r#"!"#.to_string(),
                is_regex: false,
                line_number: 68,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 70,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 71,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 72,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 73,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 74,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 75,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 77,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMI"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 78,
                alias: None,
            },
            TokenDefinition {
                name: r#"DOLLAR"#.to_string(),
                pattern: r#"$"#.to_string(),
                is_regex: false,
                line_number: 79,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 96,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"%[a-zA-Z_][a-zA-Z0-9_]*|%|[a-zA-Z_][a-zA-Z0-9_]*"#.to_string(),
                is_regex: true,
                line_number: 97,
                alias: None,
            },
            TokenDefinition {
                name: r#"STRING"#.to_string(),
                pattern: r#""([^"\\]|\\.)*""#.to_string(),
                is_regex: true,
                line_number: 98,
                alias: None,
            },
        ],
        keywords: vec![r#"and"#.to_string(), r#"or"#.to_string(), r#"not"#.to_string(), r#"if"#.to_string(), r#"then"#.to_string(), r#"else"#.to_string(), r#"elseif"#.to_string(), r#"true"#.to_string(), r#"false"#.to_string(), r#"do"#.to_string(), r#"for"#.to_string(), r#"while"#.to_string(), r#"unless"#.to_string(), r#"in"#.to_string(), r#"step"#.to_string(), r#"thru"#.to_string(), r#"block"#.to_string(), r#"return"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 137,
                alias: None,
            },
            TokenDefinition {
                name: r#"LINECOMMENT"#.to_string(),
                pattern: r#"\/\*([^*]|\*[^\/])*\*\/"#.to_string(),
                is_regex: true,
                line_number: 138,
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
