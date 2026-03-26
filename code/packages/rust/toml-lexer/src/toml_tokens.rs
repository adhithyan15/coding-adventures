// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use std::collections::HashMap;
use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition, PatternGroup};

pub fn TomlTokens() -> TokenGrammar {
    let groups = HashMap::new();
    TokenGrammar {
        version: 1,
        case_insensitive: false,
        case_sensitive: true,
        mode: None,
        escapes: Some("none".to_string()),
        keywords: vec![],
        reserved_keywords: vec![],
        definitions: vec![
            TokenDefinition { name: "ML_BASIC_STRING".to_string(), pattern: "\"\"\"([^\\\\]|\\\\(.|\\n)|\\n)*?\"\"\"".to_string(), is_regex: true, line_number: 60, alias: None },
            TokenDefinition { name: "ML_LITERAL_STRING".to_string(), pattern: "'''[\\s\\S]*?'''".to_string(), is_regex: true, line_number: 61, alias: None },
            TokenDefinition { name: "BASIC_STRING".to_string(), pattern: "\"([^\"\\\\\\n]|\\\\.)*\"".to_string(), is_regex: true, line_number: 70, alias: None },
            TokenDefinition { name: "LITERAL_STRING".to_string(), pattern: "'[^'\\n]*'".to_string(), is_regex: true, line_number: 71, alias: None },
            TokenDefinition { name: "OFFSET_DATETIME".to_string(), pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?(Z|[+-]\\d{2}:\\d{2})".to_string(), is_regex: true, line_number: 91, alias: None },
            TokenDefinition { name: "LOCAL_DATETIME".to_string(), pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?".to_string(), is_regex: true, line_number: 92, alias: None },
            TokenDefinition { name: "LOCAL_DATE".to_string(), pattern: "\\d{4}-\\d{2}-\\d{2}".to_string(), is_regex: true, line_number: 93, alias: None },
            TokenDefinition { name: "LOCAL_TIME".to_string(), pattern: "\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?".to_string(), is_regex: true, line_number: 94, alias: None },
            TokenDefinition { name: "FLOAT_SPECIAL".to_string(), pattern: "[+-]?(inf|nan)".to_string(), is_regex: true, line_number: 109, alias: Some("FLOAT".to_string()) },
            TokenDefinition { name: "FLOAT_EXP".to_string(), pattern: "[+-]?([0-9](_?[0-9])*)(\\.[0-9](_?[0-9])*)?[eE][+-]?[0-9](_?[0-9])*".to_string(), is_regex: true, line_number: 110, alias: Some("FLOAT".to_string()) },
            TokenDefinition { name: "FLOAT_DEC".to_string(), pattern: "[+-]?([0-9](_?[0-9])*)\\.([0-9](_?[0-9])*)".to_string(), is_regex: true, line_number: 111, alias: Some("FLOAT".to_string()) },
            TokenDefinition { name: "HEX_INTEGER".to_string(), pattern: "0x[0-9a-fA-F](_?[0-9a-fA-F])*".to_string(), is_regex: true, line_number: 123, alias: Some("INTEGER".to_string()) },
            TokenDefinition { name: "OCT_INTEGER".to_string(), pattern: "0o[0-7](_?[0-7])*".to_string(), is_regex: true, line_number: 124, alias: Some("INTEGER".to_string()) },
            TokenDefinition { name: "BIN_INTEGER".to_string(), pattern: "0b[01](_?[01])*".to_string(), is_regex: true, line_number: 125, alias: Some("INTEGER".to_string()) },
            TokenDefinition { name: "INTEGER".to_string(), pattern: "[+-]?[0-9](_?[0-9])*".to_string(), is_regex: true, line_number: 126, alias: None },
            TokenDefinition { name: "TRUE".to_string(), pattern: "true".to_string(), is_regex: false, line_number: 137, alias: None },
            TokenDefinition { name: "FALSE".to_string(), pattern: "false".to_string(), is_regex: false, line_number: 138, alias: None },
            TokenDefinition { name: "BARE_KEY".to_string(), pattern: "[A-Za-z0-9_-]+".to_string(), is_regex: true, line_number: 152, alias: None },
            TokenDefinition { name: "EQUALS".to_string(), pattern: "=".to_string(), is_regex: false, line_number: 162, alias: None },
            TokenDefinition { name: "DOT".to_string(), pattern: ".".to_string(), is_regex: false, line_number: 163, alias: None },
            TokenDefinition { name: "COMMA".to_string(), pattern: ",".to_string(), is_regex: false, line_number: 164, alias: None },
            TokenDefinition { name: "LBRACKET".to_string(), pattern: "[".to_string(), is_regex: false, line_number: 165, alias: None },
            TokenDefinition { name: "RBRACKET".to_string(), pattern: "]".to_string(), is_regex: false, line_number: 166, alias: None },
            TokenDefinition { name: "LBRACE".to_string(), pattern: "{".to_string(), is_regex: false, line_number: 167, alias: None },
            TokenDefinition { name: "RBRACE".to_string(), pattern: "}".to_string(), is_regex: false, line_number: 168, alias: None },
        ],
        skip_definitions: vec![
            TokenDefinition { name: "COMMENT".to_string(), pattern: "#[^\\n]*".to_string(), is_regex: true, line_number: 28, alias: None },
            TokenDefinition { name: "WHITESPACE".to_string(), pattern: "[ \\t]+".to_string(), is_regex: true, line_number: 29, alias: None },
        ],
        error_definitions: vec![
        ],
        groups,
    }
}
