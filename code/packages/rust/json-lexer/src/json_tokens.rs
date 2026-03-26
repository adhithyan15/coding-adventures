// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use std::collections::HashMap;
use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition, PatternGroup};

pub fn JsonTokens() -> TokenGrammar {
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
            TokenDefinition { name: "STRING".to_string(), pattern: "\"([^\"\\\\]|\\\\[\"\\\\\\x2fbfnrt]|\\\\u[0-9a-fA-F]{4})*\"".to_string(), is_regex: true, line_number: 25, alias: None },
            TokenDefinition { name: "NUMBER".to_string(), pattern: "-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?".to_string(), is_regex: true, line_number: 31, alias: None },
            TokenDefinition { name: "TRUE".to_string(), pattern: "true".to_string(), is_regex: false, line_number: 35, alias: None },
            TokenDefinition { name: "FALSE".to_string(), pattern: "false".to_string(), is_regex: false, line_number: 36, alias: None },
            TokenDefinition { name: "NULL".to_string(), pattern: "null".to_string(), is_regex: false, line_number: 37, alias: None },
            TokenDefinition { name: "LBRACE".to_string(), pattern: "{".to_string(), is_regex: false, line_number: 43, alias: None },
            TokenDefinition { name: "RBRACE".to_string(), pattern: "}".to_string(), is_regex: false, line_number: 44, alias: None },
            TokenDefinition { name: "LBRACKET".to_string(), pattern: "[".to_string(), is_regex: false, line_number: 45, alias: None },
            TokenDefinition { name: "RBRACKET".to_string(), pattern: "]".to_string(), is_regex: false, line_number: 46, alias: None },
            TokenDefinition { name: "COLON".to_string(), pattern: ":".to_string(), is_regex: false, line_number: 47, alias: None },
            TokenDefinition { name: "COMMA".to_string(), pattern: ",".to_string(), is_regex: false, line_number: 48, alias: None },
        ],
        skip_definitions: vec![
            TokenDefinition { name: "WHITESPACE".to_string(), pattern: "[ \\t\\r\\n]+".to_string(), is_regex: true, line_number: 59, alias: None },
        ],
        error_definitions: vec![
        ],
        groups,
    }
}
