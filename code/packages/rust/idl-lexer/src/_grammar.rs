// AUTO-GENERATED FILE — DO NOT EDIT
// Source: idl.tokens
// Regenerate with: grammar-tools compile-tokens idl.tokens
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
                name: r#"HASH_HASH"#.to_string(),
                pattern: r#"##"#.to_string(),
                is_regex: false,
                line_number: 130,
                alias: None,
            },
            TokenDefinition {
                name: r#"HASH"#.to_string(),
                pattern: r#"#"#.to_string(),
                is_regex: false,
                line_number: 131,
                alias: None,
            },
            TokenDefinition {
                name: r#"SQ_STRING"#.to_string(),
                pattern: r#"'[^']*'"#.to_string(),
                is_regex: true,
                line_number: 157,
                alias: Some(r#"STRING"#.to_string()),
            },
            TokenDefinition {
                name: r#"DQ_STRING"#.to_string(),
                pattern: r#""[^"]*""#.to_string(),
                is_regex: true,
                line_number: 158,
                alias: Some(r#"STRING"#.to_string()),
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 201,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 202,
                alias: None,
            },
            TokenDefinition {
                name: r#"STAR"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 203,
                alias: None,
            },
            TokenDefinition {
                name: r#"SLASH"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 204,
                alias: None,
            },
            TokenDefinition {
                name: r#"CARET"#.to_string(),
                pattern: r#"^"#.to_string(),
                is_regex: false,
                line_number: 205,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQUALS"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 207,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 209,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 210,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 211,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 212,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 213,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 214,
                alias: None,
            },
            TokenDefinition {
                name: r#"STMT_SEP"#.to_string(),
                pattern: r#"&"#.to_string(),
                is_regex: false,
                line_number: 242,
                alias: None,
            },
            TokenDefinition {
                name: r#"CONTINUATION"#.to_string(),
                pattern: r#"$"#.to_string(),
                is_regex: false,
                line_number: 243,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"([0-9]+(\.[0-9]+)?|\.[0-9]+)([eE][+-]?[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 261,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z][a-zA-Z0-9_]*"#.to_string(),
                is_regex: true,
                line_number: 325,
                alias: None,
            },
            TokenDefinition {
                name: r#"NEWLINE"#.to_string(),
                pattern: r#"\r?\n"#.to_string(),
                is_regex: true,
                line_number: 371,
                alias: None,
            },
        ],
        keywords: vec![r#"EQ"#.to_string(), r#"NE"#.to_string(), r#"LT"#.to_string(), r#"LE"#.to_string(), r#"GT"#.to_string(), r#"GE"#.to_string(), r#"AND"#.to_string(), r#"OR"#.to_string(), r#"NOT"#.to_string(), r#"XOR"#.to_string(), r#"IF"#.to_string(), r#"THEN"#.to_string(), r#"ELSE"#.to_string(), r#"ENDIF"#.to_string(), r#"ENDELSE"#.to_string(), r#"FOR"#.to_string(), r#"DO"#.to_string(), r#"ENDFOR"#.to_string(), r#"WHILE"#.to_string(), r#"ENDWHILE"#.to_string(), r#"REPEAT"#.to_string(), r#"UNTIL"#.to_string(), r#"ENDREP"#.to_string(), r#"BREAK"#.to_string(), r#"CONTINUE"#.to_string(), r#"BEGIN"#.to_string(), r#"END"#.to_string(), r#"PRO"#.to_string(), r#"FUNCTION"#.to_string(), r#"RETURN"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t]+"#.to_string(),
                is_regex: true,
                line_number: 390,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMENT"#.to_string(),
                pattern: r#";[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 391,
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
                line_number: 398,
                alias: None,
            },
        ],
        groups: HashMap::new(),
        case_sensitive: true,
        version: 1,
        case_insensitive: true,
        context_keywords: vec![],
        soft_keywords: vec![],
        layout_keywords: vec![],
        start_mode: None,
        transitions: vec![],
    }
}
