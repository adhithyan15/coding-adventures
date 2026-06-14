// AUTO-GENERATED FILE — DO NOT EDIT
// Source: oct.tokens
// Regenerate with: grammar-tools compile-tokens oct.tokens
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
                name: r#"EQ_EQ"#.to_string(),
                pattern: r#"=="#.to_string(),
                is_regex: false,
                line_number: 56,
                alias: None,
            },
            TokenDefinition {
                name: r#"NEQ"#.to_string(),
                pattern: r#"!="#.to_string(),
                is_regex: false,
                line_number: 61,
                alias: None,
            },
            TokenDefinition {
                name: r#"LEQ"#.to_string(),
                pattern: r#"<="#.to_string(),
                is_regex: false,
                line_number: 65,
                alias: None,
            },
            TokenDefinition {
                name: r#"GEQ"#.to_string(),
                pattern: r#">="#.to_string(),
                is_regex: false,
                line_number: 69,
                alias: None,
            },
            TokenDefinition {
                name: r#"LAND"#.to_string(),
                pattern: r#"&&"#.to_string(),
                is_regex: false,
                line_number: 76,
                alias: None,
            },
            TokenDefinition {
                name: r#"LOR"#.to_string(),
                pattern: r#"||"#.to_string(),
                is_regex: false,
                line_number: 82,
                alias: None,
            },
            TokenDefinition {
                name: r#"ARROW"#.to_string(),
                pattern: r#"->"#.to_string(),
                is_regex: false,
                line_number: 88,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 97,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 102,
                alias: None,
            },
            TokenDefinition {
                name: r#"AMP"#.to_string(),
                pattern: r#"&"#.to_string(),
                is_regex: false,
                line_number: 111,
                alias: None,
            },
            TokenDefinition {
                name: r#"PIPE"#.to_string(),
                pattern: r#"|"#.to_string(),
                is_regex: false,
                line_number: 116,
                alias: None,
            },
            TokenDefinition {
                name: r#"CARET"#.to_string(),
                pattern: r#"^"#.to_string(),
                is_regex: false,
                line_number: 121,
                alias: None,
            },
            TokenDefinition {
                name: r#"TILDE"#.to_string(),
                pattern: r#"~"#.to_string(),
                is_regex: false,
                line_number: 126,
                alias: None,
            },
            TokenDefinition {
                name: r#"BANG"#.to_string(),
                pattern: r#"!"#.to_string(),
                is_regex: false,
                line_number: 135,
                alias: None,
            },
            TokenDefinition {
                name: r#"LT"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 138,
                alias: None,
            },
            TokenDefinition {
                name: r#"GT"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 141,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQ"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 149,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 157,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 158,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 161,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 162,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 166,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMICOLON"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 171,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 174,
                alias: None,
            },
            TokenDefinition {
                name: r#"BIN_LIT"#.to_string(),
                pattern: r#"0b[01]+"#.to_string(),
                is_regex: true,
                line_number: 187,
                alias: None,
            },
            TokenDefinition {
                name: r#"HEX_LIT"#.to_string(),
                pattern: r#"0x[0-9A-Fa-f]+"#.to_string(),
                is_regex: true,
                line_number: 193,
                alias: None,
            },
            TokenDefinition {
                name: r#"INT_LIT"#.to_string(),
                pattern: r#"[0-9]+"#.to_string(),
                is_regex: true,
                line_number: 198,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z_][a-zA-Z0-9_]*"#.to_string(),
                is_regex: true,
                line_number: 206,
                alias: None,
            },
        ],
        keywords: vec![r#"fn"#.to_string(), r#"let"#.to_string(), r#"static"#.to_string(), r#"if"#.to_string(), r#"else"#.to_string(), r#"while"#.to_string(), r#"loop"#.to_string(), r#"break"#.to_string(), r#"return"#.to_string(), r#"true"#.to_string(), r#"false"#.to_string(), r#"in"#.to_string(), r#"out"#.to_string(), r#"adc"#.to_string(), r#"sbb"#.to_string(), r#"rlc"#.to_string(), r#"rrc"#.to_string(), r#"ral"#.to_string(), r#"rar"#.to_string(), r#"carry"#.to_string(), r#"parity"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 343,
                alias: None,
            },
            TokenDefinition {
                name: r#"LINE_COMMENT"#.to_string(),
                pattern: r#"\/\/[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 348,
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
