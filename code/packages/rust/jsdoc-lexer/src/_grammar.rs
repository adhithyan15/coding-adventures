// AUTO-GENERATED FILE — DO NOT EDIT
// Source: jsdoc.tokens
// Regenerate with: grammar-tools compile-tokens jsdoc.tokens
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
                name: r#"AT_TAG"#.to_string(),
                pattern: r#"@[a-zA-Z_$][a-zA-Z0-9_$-]*"#.to_string(),
                is_regex: true,
                line_number: 45,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 48,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 49,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 50,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 51,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 52,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 53,
                alias: None,
            },
            TokenDefinition {
                name: r#"ANGLE_OPEN"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 54,
                alias: None,
            },
            TokenDefinition {
                name: r#"ANGLE_CLOSE"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 55,
                alias: None,
            },
            TokenDefinition {
                name: r#"PIPE"#.to_string(),
                pattern: r#"|"#.to_string(),
                is_regex: false,
                line_number: 58,
                alias: None,
            },
            TokenDefinition {
                name: r#"AMP"#.to_string(),
                pattern: r#"&"#.to_string(),
                is_regex: false,
                line_number: 59,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 60,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 61,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQUALS"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 62,
                alias: None,
            },
            TokenDefinition {
                name: r#"ELLIPSIS"#.to_string(),
                pattern: r#"..."#.to_string(),
                is_regex: false,
                line_number: 63,
                alias: None,
            },
            TokenDefinition {
                name: r#"QUESTION"#.to_string(),
                pattern: r#"?"#.to_string(),
                is_regex: false,
                line_number: 64,
                alias: None,
            },
            TokenDefinition {
                name: r#"BANG"#.to_string(),
                pattern: r#"!"#.to_string(),
                is_regex: false,
                line_number: 65,
                alias: None,
            },
            TokenDefinition {
                name: r#"STAR"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 66,
                alias: None,
            },
            TokenDefinition {
                name: r#"ARROW"#.to_string(),
                pattern: r#"=>"#.to_string(),
                is_regex: false,
                line_number: 67,
                alias: None,
            },
            TokenDefinition {
                name: r#"DOT"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: false,
                line_number: 68,
                alias: None,
            },
            TokenDefinition {
                name: r#"NEWLINE"#.to_string(),
                pattern: r#"\n"#.to_string(),
                is_regex: true,
                line_number: 72,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z_$][a-zA-Z0-9_$]*"#.to_string(),
                is_regex: true,
                line_number: 79,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"-?[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 82,
                alias: None,
            },
            TokenDefinition {
                name: r#"STRING_DQ"#.to_string(),
                pattern: r#""([^"\\]|\\.)*""#.to_string(),
                is_regex: true,
                line_number: 85,
                alias: Some(r#"STRING"#.to_string()),
            },
            TokenDefinition {
                name: r#"STRING_SQ"#.to_string(),
                pattern: r#"'([^'\\]|\\.)*'"#.to_string(),
                is_regex: true,
                line_number: 86,
                alias: Some(r#"STRING"#.to_string()),
            },
            TokenDefinition {
                name: r#"DESCRIPTION_TEXT"#.to_string(),
                pattern: r#"[a-zA-Z_$0-9 \t][^\n@{}\[\]()<>|&,:=?!*]*"#.to_string(),
                is_regex: true,
                line_number: 100,
                alias: None,
            },
        ],
        keywords: vec![],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"HSPACE"#.to_string(),
                pattern: r#"[ \t]+"#.to_string(),
                is_regex: true,
                line_number: 109,
                alias: None,
            },
            TokenDefinition {
                name: r#"LEADING_STAR"#.to_string(),
                pattern: r#"\n[ \t]*\*[ \t]?"#.to_string(),
                is_regex: true,
                line_number: 117,
                alias: None,
            },
        ],
        reserved_keywords: vec![],
        escapes: None,
        error_definitions: vec![
            TokenDefinition {
                name: r#"BAD_STRING_DQ"#.to_string(),
                pattern: r#""[^"\n]*$"#.to_string(),
                is_regex: true,
                line_number: 124,
                alias: None,
            },
            TokenDefinition {
                name: r#"BAD_STRING_SQ"#.to_string(),
                pattern: r#"'[^'\n]*$"#.to_string(),
                is_regex: true,
                line_number: 125,
                alias: None,
            },
            TokenDefinition {
                name: r#"UNTERMINATED_TYPE"#.to_string(),
                pattern: r#"\{[^}]*$"#.to_string(),
                is_regex: true,
                line_number: 126,
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
