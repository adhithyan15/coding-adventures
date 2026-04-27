// AUTO-GENERATED FILE — DO NOT EDIT
// Source: algol.tokens
// Regenerate with: grammar-tools compile-tokens algol.tokens
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
                name: r#"REAL_LIT"#.to_string(),
                pattern: r#"[0-9]+\.[0-9]*([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+"#.to_string(),
                is_regex: true,
                line_number: 37,
                alias: None,
            },
            TokenDefinition {
                name: r#"INTEGER_LIT"#.to_string(),
                pattern: r#"[0-9]+"#.to_string(),
                is_regex: true,
                line_number: 40,
                alias: None,
            },
            TokenDefinition {
                name: r#"STRING_LIT"#.to_string(),
                pattern: r#"'[^']*'"#.to_string(),
                is_regex: true,
                line_number: 44,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z][a-zA-Z0-9]*"#.to_string(),
                is_regex: true,
                line_number: 51,
                alias: None,
            },
            TokenDefinition {
                name: r#"ASSIGN"#.to_string(),
                pattern: r#":="#.to_string(),
                is_regex: false,
                line_number: 59,
                alias: None,
            },
            TokenDefinition {
                name: r#"POWER"#.to_string(),
                pattern: r#"**"#.to_string(),
                is_regex: false,
                line_number: 64,
                alias: None,
            },
            TokenDefinition {
                name: r#"LEQ"#.to_string(),
                pattern: r#"<="#.to_string(),
                is_regex: false,
                line_number: 67,
                alias: None,
            },
            TokenDefinition {
                name: r#"GEQ"#.to_string(),
                pattern: r#">="#.to_string(),
                is_regex: false,
                line_number: 68,
                alias: None,
            },
            TokenDefinition {
                name: r#"NEQ"#.to_string(),
                pattern: r#"!="#.to_string(),
                is_regex: false,
                line_number: 69,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 75,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 76,
                alias: None,
            },
            TokenDefinition {
                name: r#"STAR"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 77,
                alias: None,
            },
            TokenDefinition {
                name: r#"SLASH"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 78,
                alias: None,
            },
            TokenDefinition {
                name: r#"CARET"#.to_string(),
                pattern: r#"^"#.to_string(),
                is_regex: false,
                line_number: 83,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQ"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 86,
                alias: None,
            },
            TokenDefinition {
                name: r#"LT"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 88,
                alias: None,
            },
            TokenDefinition {
                name: r#"GT"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 89,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 95,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 96,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 97,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 98,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMICOLON"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 99,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 100,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 104,
                alias: None,
            },
        ],
        keywords: vec![r#"begin"#.to_string(), r#"end"#.to_string(), r#"if"#.to_string(), r#"then"#.to_string(), r#"else"#.to_string(), r#"for"#.to_string(), r#"do"#.to_string(), r#"step"#.to_string(), r#"until"#.to_string(), r#"while"#.to_string(), r#"goto"#.to_string(), r#"switch"#.to_string(), r#"procedure"#.to_string(), r#"own"#.to_string(), r#"array"#.to_string(), r#"label"#.to_string(), r#"value"#.to_string(), r#"integer"#.to_string(), r#"real"#.to_string(), r#"boolean"#.to_string(), r#"string"#.to_string(), r#"true"#.to_string(), r#"false"#.to_string(), r#"not"#.to_string(), r#"and"#.to_string(), r#"or"#.to_string(), r#"impl"#.to_string(), r#"eqv"#.to_string(), r#"div"#.to_string(), r#"mod"#.to_string(), r#"comment"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 171,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMENT"#.to_string(),
                pattern: r#"comment[^;]*;"#.to_string(),
                is_regex: true,
                line_number: 177,
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
    }
}
