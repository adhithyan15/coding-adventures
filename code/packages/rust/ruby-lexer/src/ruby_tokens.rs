// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use std::collections::HashMap;
use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition, PatternGroup};

pub fn RubyTokens() -> TokenGrammar {
    let groups = HashMap::new();
    TokenGrammar {
        version: 1,
        case_insensitive: false,
        case_sensitive: true,
        mode: None,
        escapes: None,
        keywords: vec!["if".to_string(), "else".to_string(), "elsif".to_string(), "end".to_string(), "while".to_string(), "for".to_string(), "do".to_string(), "def".to_string(), "return".to_string(), "class".to_string(), "module".to_string(), "require".to_string(), "puts".to_string(), "true".to_string(), "false".to_string(), "nil".to_string(), "and".to_string(), "or".to_string(), "not".to_string(), "then".to_string(), "unless".to_string(), "until".to_string(), "yield".to_string(), "begin".to_string(), "rescue".to_string(), "ensure".to_string()],
        reserved_keywords: vec![],
        definitions: vec![
            TokenDefinition { name: "NAME".to_string(), pattern: "[a-zA-Z_][a-zA-Z0-9_]*".to_string(), is_regex: true, line_number: 23, alias: None },
            TokenDefinition { name: "NUMBER".to_string(), pattern: "[0-9]+".to_string(), is_regex: true, line_number: 24, alias: None },
            TokenDefinition { name: "STRING".to_string(), pattern: "\"([^\"\\\\]|\\\\.)*\"".to_string(), is_regex: true, line_number: 25, alias: None },
            TokenDefinition { name: "EQUALS_EQUALS".to_string(), pattern: "==".to_string(), is_regex: false, line_number: 28, alias: None },
            TokenDefinition { name: "DOT_DOT".to_string(), pattern: "..".to_string(), is_regex: false, line_number: 29, alias: None },
            TokenDefinition { name: "HASH_ROCKET".to_string(), pattern: "=>".to_string(), is_regex: false, line_number: 30, alias: None },
            TokenDefinition { name: "NOT_EQUALS".to_string(), pattern: "!=".to_string(), is_regex: false, line_number: 31, alias: None },
            TokenDefinition { name: "LESS_EQUALS".to_string(), pattern: "<=".to_string(), is_regex: false, line_number: 32, alias: None },
            TokenDefinition { name: "GREATER_EQUALS".to_string(), pattern: ">=".to_string(), is_regex: false, line_number: 33, alias: None },
            TokenDefinition { name: "EQUALS".to_string(), pattern: "=".to_string(), is_regex: false, line_number: 36, alias: None },
            TokenDefinition { name: "PLUS".to_string(), pattern: "+".to_string(), is_regex: false, line_number: 37, alias: None },
            TokenDefinition { name: "MINUS".to_string(), pattern: "-".to_string(), is_regex: false, line_number: 38, alias: None },
            TokenDefinition { name: "STAR".to_string(), pattern: "*".to_string(), is_regex: false, line_number: 39, alias: None },
            TokenDefinition { name: "SLASH".to_string(), pattern: "/".to_string(), is_regex: false, line_number: 40, alias: None },
            TokenDefinition { name: "LESS_THAN".to_string(), pattern: "<".to_string(), is_regex: false, line_number: 43, alias: None },
            TokenDefinition { name: "GREATER_THAN".to_string(), pattern: ">".to_string(), is_regex: false, line_number: 44, alias: None },
            TokenDefinition { name: "LPAREN".to_string(), pattern: "(".to_string(), is_regex: false, line_number: 47, alias: None },
            TokenDefinition { name: "RPAREN".to_string(), pattern: ")".to_string(), is_regex: false, line_number: 48, alias: None },
            TokenDefinition { name: "COMMA".to_string(), pattern: ",".to_string(), is_regex: false, line_number: 49, alias: None },
            TokenDefinition { name: "COLON".to_string(), pattern: ":".to_string(), is_regex: false, line_number: 50, alias: None },
        ],
        skip_definitions: vec![
        ],
        error_definitions: vec![
        ],
        groups,
    }
}
