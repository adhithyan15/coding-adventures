// AUTO-GENERATED FILE — DO NOT EDIT
// Source: ruby.tokens
// Regenerate with: grammar-tools compile-tokens ruby.tokens
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
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z_][a-zA-Z0-9_]*"#.to_string(),
                is_regex: true,
                line_number: 23,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+"#.to_string(),
                is_regex: true,
                line_number: 24,
                alias: None,
            },
            TokenDefinition {
                name: r#"STRING"#.to_string(),
                pattern: r#""([^"\\]|\\.)*""#.to_string(),
                is_regex: true,
                line_number: 25,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQUALS_EQUALS"#.to_string(),
                pattern: r#"=="#.to_string(),
                is_regex: false,
                line_number: 28,
                alias: None,
            },
            TokenDefinition {
                name: r#"DOT_DOT"#.to_string(),
                pattern: r#".."#.to_string(),
                is_regex: false,
                line_number: 29,
                alias: None,
            },
            TokenDefinition {
                name: r#"HASH_ROCKET"#.to_string(),
                pattern: r#"=>"#.to_string(),
                is_regex: false,
                line_number: 30,
                alias: None,
            },
            TokenDefinition {
                name: r#"NOT_EQUALS"#.to_string(),
                pattern: r#"!="#.to_string(),
                is_regex: false,
                line_number: 31,
                alias: None,
            },
            TokenDefinition {
                name: r#"LESS_EQUALS"#.to_string(),
                pattern: r#"<="#.to_string(),
                is_regex: false,
                line_number: 32,
                alias: None,
            },
            TokenDefinition {
                name: r#"GREATER_EQUALS"#.to_string(),
                pattern: r#">="#.to_string(),
                is_regex: false,
                line_number: 33,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQUALS"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 36,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 37,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 38,
                alias: None,
            },
            TokenDefinition {
                name: r#"STAR"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 39,
                alias: None,
            },
            TokenDefinition {
                name: r#"SLASH"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 40,
                alias: None,
            },
            TokenDefinition {
                name: r#"LESS_THAN"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 43,
                alias: None,
            },
            TokenDefinition {
                name: r#"GREATER_THAN"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 44,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 47,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 48,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 49,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 50,
                alias: None,
            },
        ],
        keywords: vec![r#"if"#.to_string(), r#"else"#.to_string(), r#"elsif"#.to_string(), r#"end"#.to_string(), r#"while"#.to_string(), r#"for"#.to_string(), r#"do"#.to_string(), r#"def"#.to_string(), r#"return"#.to_string(), r#"class"#.to_string(), r#"module"#.to_string(), r#"require"#.to_string(), r#"puts"#.to_string(), r#"true"#.to_string(), r#"false"#.to_string(), r#"nil"#.to_string(), r#"and"#.to_string(), r#"or"#.to_string(), r#"not"#.to_string(), r#"then"#.to_string(), r#"unless"#.to_string(), r#"until"#.to_string(), r#"yield"#.to_string(), r#"begin"#.to_string(), r#"rescue"#.to_string(), r#"ensure"#.to_string()],
        mode: None,
        skip_definitions: vec![],
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
    }
}
