// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use std::collections::HashMap;
use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition, PatternGroup};

pub fn LispTokens() -> TokenGrammar {
    let groups = HashMap::new();
    TokenGrammar {
        version: 1,
        case_insensitive: false,
        case_sensitive: true,
        mode: None,
        escapes: None,
        keywords: vec![],
        reserved_keywords: vec![],
        definitions: vec![
            TokenDefinition { name: "NUMBER".to_string(), pattern: "-?[0-9]+".to_string(), is_regex: true, line_number: 6, alias: None },
            TokenDefinition { name: "SYMBOL".to_string(), pattern: "[a-zA-Z_+\\-*\\/=<>!?&][a-zA-Z0-9_+\\-*\\/=<>!?&]*".to_string(), is_regex: true, line_number: 7, alias: None },
            TokenDefinition { name: "STRING".to_string(), pattern: "\"([^\"\\\\]|\\\\.)*\"".to_string(), is_regex: true, line_number: 8, alias: None },
            TokenDefinition { name: "LPAREN".to_string(), pattern: "(".to_string(), is_regex: false, line_number: 9, alias: None },
            TokenDefinition { name: "RPAREN".to_string(), pattern: ")".to_string(), is_regex: false, line_number: 10, alias: None },
            TokenDefinition { name: "QUOTE".to_string(), pattern: "'".to_string(), is_regex: false, line_number: 11, alias: None },
            TokenDefinition { name: "DOT".to_string(), pattern: ".".to_string(), is_regex: false, line_number: 12, alias: None },
        ],
        skip_definitions: vec![
            TokenDefinition { name: "WHITESPACE".to_string(), pattern: "[ \\t\\r\\n]+".to_string(), is_regex: true, line_number: 3, alias: None },
            TokenDefinition { name: "COMMENT".to_string(), pattern: ";[^\\n]*".to_string(), is_regex: true, line_number: 4, alias: None },
        ],
        error_definitions: vec![
        ],
        groups,
    }
}
