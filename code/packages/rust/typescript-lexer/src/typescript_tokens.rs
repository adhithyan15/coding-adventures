// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use std::collections::HashMap;
use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition, PatternGroup};

pub fn TypescriptTokens() -> TokenGrammar {
    let groups = HashMap::new();
    TokenGrammar {
        version: 1,
        case_insensitive: false,
        case_sensitive: true,
        mode: None,
        escapes: None,
        keywords: vec!["let".to_string(), "const".to_string(), "var".to_string(), "if".to_string(), "else".to_string(), "while".to_string(), "for".to_string(), "do".to_string(), "function".to_string(), "return".to_string(), "class".to_string(), "import".to_string(), "export".to_string(), "from".to_string(), "as".to_string(), "new".to_string(), "this".to_string(), "typeof".to_string(), "instanceof".to_string(), "true".to_string(), "false".to_string(), "null".to_string(), "undefined".to_string(), "interface".to_string(), "type".to_string(), "enum".to_string(), "namespace".to_string(), "declare".to_string(), "readonly".to_string(), "public".to_string(), "private".to_string(), "protected".to_string(), "abstract".to_string(), "implements".to_string(), "extends".to_string(), "keyof".to_string(), "infer".to_string(), "never".to_string(), "unknown".to_string(), "any".to_string(), "void".to_string(), "number".to_string(), "string".to_string(), "boolean".to_string(), "object".to_string(), "symbol".to_string(), "bigint".to_string()],
        reserved_keywords: vec![],
        definitions: vec![
            TokenDefinition { name: "NAME".to_string(), pattern: "[a-zA-Z_$][a-zA-Z0-9_$]*".to_string(), is_regex: true, line_number: 25, alias: None },
            TokenDefinition { name: "NUMBER".to_string(), pattern: "[0-9]+".to_string(), is_regex: true, line_number: 26, alias: None },
            TokenDefinition { name: "STRING".to_string(), pattern: "\"([^\"\\\\]|\\\\.)*\"".to_string(), is_regex: true, line_number: 27, alias: None },
            TokenDefinition { name: "STRICT_EQUALS".to_string(), pattern: "===".to_string(), is_regex: false, line_number: 30, alias: None },
            TokenDefinition { name: "STRICT_NOT_EQUALS".to_string(), pattern: "!==".to_string(), is_regex: false, line_number: 31, alias: None },
            TokenDefinition { name: "EQUALS_EQUALS".to_string(), pattern: "==".to_string(), is_regex: false, line_number: 32, alias: None },
            TokenDefinition { name: "NOT_EQUALS".to_string(), pattern: "!=".to_string(), is_regex: false, line_number: 33, alias: None },
            TokenDefinition { name: "LESS_EQUALS".to_string(), pattern: "<=".to_string(), is_regex: false, line_number: 34, alias: None },
            TokenDefinition { name: "GREATER_EQUALS".to_string(), pattern: ">=".to_string(), is_regex: false, line_number: 35, alias: None },
            TokenDefinition { name: "ARROW".to_string(), pattern: "=>".to_string(), is_regex: false, line_number: 36, alias: None },
            TokenDefinition { name: "EQUALS".to_string(), pattern: "=".to_string(), is_regex: false, line_number: 39, alias: None },
            TokenDefinition { name: "PLUS".to_string(), pattern: "+".to_string(), is_regex: false, line_number: 40, alias: None },
            TokenDefinition { name: "MINUS".to_string(), pattern: "-".to_string(), is_regex: false, line_number: 41, alias: None },
            TokenDefinition { name: "STAR".to_string(), pattern: "*".to_string(), is_regex: false, line_number: 42, alias: None },
            TokenDefinition { name: "SLASH".to_string(), pattern: "/".to_string(), is_regex: false, line_number: 43, alias: None },
            TokenDefinition { name: "LESS_THAN".to_string(), pattern: "<".to_string(), is_regex: false, line_number: 44, alias: None },
            TokenDefinition { name: "GREATER_THAN".to_string(), pattern: ">".to_string(), is_regex: false, line_number: 45, alias: None },
            TokenDefinition { name: "BANG".to_string(), pattern: "!".to_string(), is_regex: false, line_number: 46, alias: None },
            TokenDefinition { name: "LPAREN".to_string(), pattern: "(".to_string(), is_regex: false, line_number: 49, alias: None },
            TokenDefinition { name: "RPAREN".to_string(), pattern: ")".to_string(), is_regex: false, line_number: 50, alias: None },
            TokenDefinition { name: "LBRACE".to_string(), pattern: "{".to_string(), is_regex: false, line_number: 51, alias: None },
            TokenDefinition { name: "RBRACE".to_string(), pattern: "}".to_string(), is_regex: false, line_number: 52, alias: None },
            TokenDefinition { name: "LBRACKET".to_string(), pattern: "[".to_string(), is_regex: false, line_number: 53, alias: None },
            TokenDefinition { name: "RBRACKET".to_string(), pattern: "]".to_string(), is_regex: false, line_number: 54, alias: None },
            TokenDefinition { name: "COMMA".to_string(), pattern: ",".to_string(), is_regex: false, line_number: 55, alias: None },
            TokenDefinition { name: "COLON".to_string(), pattern: ":".to_string(), is_regex: false, line_number: 56, alias: None },
            TokenDefinition { name: "SEMICOLON".to_string(), pattern: ";".to_string(), is_regex: false, line_number: 57, alias: None },
            TokenDefinition { name: "DOT".to_string(), pattern: ".".to_string(), is_regex: false, line_number: 58, alias: None },
        ],
        skip_definitions: vec![
        ],
        error_definitions: vec![
        ],
        groups,
    }
}
