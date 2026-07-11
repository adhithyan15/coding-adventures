// AUTO-GENERATED FILE — DO NOT EDIT
// Source: cobol.tokens
// Regenerate with: grammar-tools compile-tokens cobol.tokens
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
                name: r#"NUMBER"#.to_string(),
                pattern: r#"-?[0-9]+(\.[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 68,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[A-Za-z][A-Za-z0-9]*(-[A-Za-z0-9]+)*"#.to_string(),
                is_regex: true,
                line_number: 75,
                alias: None,
            },
            TokenDefinition {
                name: r#"STRING_DQ"#.to_string(),
                pattern: r#""[^"]*""#.to_string(),
                is_regex: true,
                line_number: 80,
                alias: Some(r#"STRING"#.to_string()),
            },
            TokenDefinition {
                name: r#"STRING_SQ"#.to_string(),
                pattern: r#"'[^']*'"#.to_string(),
                is_regex: true,
                line_number: 81,
                alias: Some(r#"STRING"#.to_string()),
            },
            TokenDefinition {
                name: r#"DOT"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: false,
                line_number: 92,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 93,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 94,
                alias: None,
            },
        ],
        keywords: vec![r#"IDENTIFICATION"#.to_string(), r#"ENVIRONMENT"#.to_string(), r#"DATA"#.to_string(), r#"PROCEDURE"#.to_string(), r#"DIVISION"#.to_string(), r#"SECTION"#.to_string(), r#"PROGRAM-ID"#.to_string(), r#"AUTHOR"#.to_string(), r#"INSTALLATION"#.to_string(), r#"DATE-WRITTEN"#.to_string(), r#"DATE-COMPILED"#.to_string(), r#"SECURITY"#.to_string(), r#"REMARKS"#.to_string(), r#"CONFIGURATION"#.to_string(), r#"SOURCE-COMPUTER"#.to_string(), r#"OBJECT-COMPUTER"#.to_string(), r#"INPUT-OUTPUT"#.to_string(), r#"FILE-CONTROL"#.to_string(), r#"SELECT"#.to_string(), r#"ASSIGN"#.to_string(), r#"FILE"#.to_string(), r#"WORKING-STORAGE"#.to_string(), r#"FD"#.to_string(), r#"SD"#.to_string(), r#"PICTURE"#.to_string(), r#"PIC"#.to_string(), r#"VALUE"#.to_string(), r#"FILLER"#.to_string(), r#"OCCURS"#.to_string(), r#"REDEFINES"#.to_string(), r#"USAGE"#.to_string(), r#"COMPUTATIONAL"#.to_string(), r#"COMP"#.to_string(), r#"MOVE"#.to_string(), r#"ADD"#.to_string(), r#"SUBTRACT"#.to_string(), r#"MULTIPLY"#.to_string(), r#"DIVIDE"#.to_string(), r#"COMPUTE"#.to_string(), r#"PERFORM"#.to_string(), r#"DISPLAY"#.to_string(), r#"ACCEPT"#.to_string(), r#"OPEN"#.to_string(), r#"CLOSE"#.to_string(), r#"READ"#.to_string(), r#"WRITE"#.to_string(), r#"GO"#.to_string(), r#"STOP"#.to_string(), r#"RUN"#.to_string(), r#"ALTER"#.to_string(), r#"EXAMINE"#.to_string(), r#"IF"#.to_string(), r#"ELSE"#.to_string(), r#"TO"#.to_string(), r#"FROM"#.to_string(), r#"BY"#.to_string(), r#"INTO"#.to_string(), r#"GIVING"#.to_string(), r#"IS"#.to_string(), r#"ARE"#.to_string(), r#"OF"#.to_string(), r#"IN"#.to_string(), r#"AND"#.to_string(), r#"OR"#.to_string(), r#"NOT"#.to_string(), r#"GREATER"#.to_string(), r#"LESS"#.to_string(), r#"EQUAL"#.to_string(), r#"THAN"#.to_string(), r#"THROUGH"#.to_string(), r#"THRU"#.to_string(), r#"UNTIL"#.to_string(), r#"VARYING"#.to_string(), r#"TIMES"#.to_string(), r#"DEPENDING"#.to_string(), r#"ON"#.to_string(), r#"WHEN"#.to_string(), r#"NEXT"#.to_string(), r#"SENTENCE"#.to_string(), r#"ZERO"#.to_string(), r#"ZEROS"#.to_string(), r#"ZEROES"#.to_string(), r#"SPACE"#.to_string(), r#"SPACES"#.to_string(), r#"HIGH-VALUE"#.to_string(), r#"HIGH-VALUES"#.to_string(), r#"LOW-VALUE"#.to_string(), r#"LOW-VALUES"#.to_string(), r#"QUOTE"#.to_string(), r#"QUOTES"#.to_string(), r#"ALL"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 245,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEPARATOR"#.to_string(),
                pattern: r#"[,;]"#.to_string(),
                is_regex: true,
                line_number: 246,
                alias: None,
            },
        ],
        reserved_keywords: vec![],
        escapes: None,
        error_definitions: vec![
            TokenDefinition {
                name: r#"UNKNOWN"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: true,
                line_number: 252,
                alias: None,
            },
        ],
        groups: {
            let mut __map: HashMap<String, PatternGroup> = HashMap::new();
            let mut __g_picture = PatternGroup { name: r#"picture"#.to_string(), definitions: vec![
                    TokenDefinition {
                        name: r#"PIC_STRING"#.to_string(),
                        pattern: r#"[9XAVSPxavsp()0-9]+"#.to_string(),
                        is_regex: true,
                        line_number: 111,
                        alias: None,
                    },
                ] };
            __map.insert(r#"picture"#.to_string(), __g_picture);
            __map
        },
        case_sensitive: false,
        version: 1,
        case_insensitive: false,
        context_keywords: vec![],
        soft_keywords: vec![],
        layout_keywords: vec![],
        start_mode: Some(r#"default"#.to_string()),
        transitions: vec![
            ModeTransition {
                on_tokens: vec![r#"KEYWORD"#.to_string()],
                on_value: Some(r#"PICTURE"#.to_string()),
                in_mode: None,
                actions: vec![TransitionAction::SetMode(r#"picture"#.to_string())],
                line_number: 232,
            },
            ModeTransition {
                on_tokens: vec![r#"KEYWORD"#.to_string()],
                on_value: Some(r#"PIC"#.to_string()),
                in_mode: None,
                actions: vec![TransitionAction::SetMode(r#"picture"#.to_string())],
                line_number: 233,
            },
            ModeTransition {
                on_tokens: vec![r#"PIC_STRING"#.to_string()],
                on_value: None,
                in_mode: None,
                actions: vec![TransitionAction::SetMode(r#"default"#.to_string())],
                line_number: 234,
            },
        ],
    }
}
