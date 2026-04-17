// AUTO-GENERATED FILE — DO NOT EDIT
// Source: mosaic.tokens
// Regenerate with: grammar-tools compile-tokens mosaic.tokens
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
                pattern: r#""([^"\\\n]|\\.)*""#.to_string(),
                is_regex: true,
                line_number: 23,
                alias: None,
            },
            TokenDefinition {
                name: r#"DIMENSION"#.to_string(),
                pattern: r#"-?[0-9]*\.?[0-9]+[a-zA-Z%]+"#.to_string(),
                is_regex: true,
                line_number: 31,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"-?[0-9]*\.?[0-9]+"#.to_string(),
                is_regex: true,
                line_number: 32,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLOR_HEX"#.to_string(),
                pattern: r#"#[0-9a-fA-F]{3,8}"#.to_string(),
                is_regex: true,
                line_number: 39,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z_][a-zA-Z0-9_-]*"#.to_string(),
                is_regex: true,
                line_number: 70,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 76,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 77,
                alias: None,
            },
            TokenDefinition {
                name: r#"LANGLE"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 78,
                alias: None,
            },
            TokenDefinition {
                name: r#"RANGLE"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 79,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 80,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMICOLON"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 81,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 82,
                alias: None,
            },
            TokenDefinition {
                name: r#"DOT"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: false,
                line_number: 83,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQUALS"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 84,
                alias: None,
            },
            TokenDefinition {
                name: r#"AT"#.to_string(),
                pattern: r#"@"#.to_string(),
                is_regex: false,
                line_number: 85,
                alias: None,
            },
        ],
        keywords: vec![r#"component"#.to_string(), r#"slot"#.to_string(), r#"import"#.to_string(), r#"from"#.to_string(), r#"as"#.to_string(), r#"text"#.to_string(), r#"number"#.to_string(), r#"bool"#.to_string(), r#"image"#.to_string(), r#"color"#.to_string(), r#"node"#.to_string(), r#"list"#.to_string(), r#"true"#.to_string(), r#"false"#.to_string(), r#"when"#.to_string(), r#"each"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"LINE_COMMENT"#.to_string(),
                pattern: r#"\/\/[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 15,
                alias: None,
            },
            TokenDefinition {
                name: r#"BLOCK_COMMENT"#.to_string(),
                pattern: r#"\/\*[\s\S]*?\*\/"#.to_string(),
                is_regex: true,
                line_number: 16,
                alias: None,
            },
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 17,
                alias: None,
            },
        ],
        reserved_keywords: vec![],
        escapes: Some(r#"standard"#.to_string()),
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
