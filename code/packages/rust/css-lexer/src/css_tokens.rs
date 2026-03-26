// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use std::collections::HashMap;
use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition, PatternGroup};

pub fn CssTokens() -> TokenGrammar {
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
            TokenDefinition { name: "STRING_DQ".to_string(), pattern: "\"([^\"\\\\\\n]|\\\\.)*\"".to_string(), is_regex: true, line_number: 67, alias: Some("STRING".to_string()) },
            TokenDefinition { name: "STRING_SQ".to_string(), pattern: "'([^'\\\\\\n]|\\\\.)*'".to_string(), is_regex: true, line_number: 68, alias: Some("STRING".to_string()) },
            TokenDefinition { name: "DIMENSION".to_string(), pattern: "-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?[a-zA-Z]+".to_string(), is_regex: true, line_number: 94, alias: None },
            TokenDefinition { name: "PERCENTAGE".to_string(), pattern: "-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?%".to_string(), is_regex: true, line_number: 95, alias: None },
            TokenDefinition { name: "NUMBER".to_string(), pattern: "-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?".to_string(), is_regex: true, line_number: 96, alias: None },
            TokenDefinition { name: "HASH".to_string(), pattern: "#[a-zA-Z0-9_-]+".to_string(), is_regex: true, line_number: 109, alias: None },
            TokenDefinition { name: "AT_KEYWORD".to_string(), pattern: "@-?[a-zA-Z][a-zA-Z0-9-]*".to_string(), is_regex: true, line_number: 123, alias: None },
            TokenDefinition { name: "URL_TOKEN".to_string(), pattern: "url\\([^)'\"]*\\)".to_string(), is_regex: true, line_number: 136, alias: None },
            TokenDefinition { name: "FUNCTION".to_string(), pattern: "-?[a-zA-Z_][a-zA-Z0-9_-]*\\(".to_string(), is_regex: true, line_number: 149, alias: None },
            TokenDefinition { name: "CDO".to_string(), pattern: "<!--".to_string(), is_regex: false, line_number: 162, alias: None },
            TokenDefinition { name: "CDC".to_string(), pattern: "-->".to_string(), is_regex: false, line_number: 163, alias: None },
            TokenDefinition { name: "UNICODE_RANGE".to_string(), pattern: "[Uu]\\+[0-9a-fA-F?]{1,6}(-[0-9a-fA-F]{1,6})?".to_string(), is_regex: true, line_number: 190, alias: None },
            TokenDefinition { name: "CUSTOM_PROPERTY".to_string(), pattern: "--[a-zA-Z_][a-zA-Z0-9_-]*".to_string(), is_regex: true, line_number: 192, alias: None },
            TokenDefinition { name: "IDENT".to_string(), pattern: "-?[a-zA-Z_][a-zA-Z0-9_-]*".to_string(), is_regex: true, line_number: 193, alias: None },
            TokenDefinition { name: "COLON_COLON".to_string(), pattern: "::".to_string(), is_regex: false, line_number: 202, alias: None },
            TokenDefinition { name: "TILDE_EQUALS".to_string(), pattern: "~=".to_string(), is_regex: false, line_number: 203, alias: None },
            TokenDefinition { name: "PIPE_EQUALS".to_string(), pattern: "|=".to_string(), is_regex: false, line_number: 204, alias: None },
            TokenDefinition { name: "CARET_EQUALS".to_string(), pattern: "^=".to_string(), is_regex: false, line_number: 205, alias: None },
            TokenDefinition { name: "DOLLAR_EQUALS".to_string(), pattern: "$=".to_string(), is_regex: false, line_number: 206, alias: None },
            TokenDefinition { name: "STAR_EQUALS".to_string(), pattern: "*=".to_string(), is_regex: false, line_number: 207, alias: None },
            TokenDefinition { name: "LBRACE".to_string(), pattern: "{".to_string(), is_regex: false, line_number: 216, alias: None },
            TokenDefinition { name: "RBRACE".to_string(), pattern: "}".to_string(), is_regex: false, line_number: 217, alias: None },
            TokenDefinition { name: "LPAREN".to_string(), pattern: "(".to_string(), is_regex: false, line_number: 218, alias: None },
            TokenDefinition { name: "RPAREN".to_string(), pattern: ")".to_string(), is_regex: false, line_number: 219, alias: None },
            TokenDefinition { name: "LBRACKET".to_string(), pattern: "[".to_string(), is_regex: false, line_number: 220, alias: None },
            TokenDefinition { name: "RBRACKET".to_string(), pattern: "]".to_string(), is_regex: false, line_number: 221, alias: None },
            TokenDefinition { name: "SEMICOLON".to_string(), pattern: ";".to_string(), is_regex: false, line_number: 222, alias: None },
            TokenDefinition { name: "COLON".to_string(), pattern: ":".to_string(), is_regex: false, line_number: 223, alias: None },
            TokenDefinition { name: "COMMA".to_string(), pattern: ",".to_string(), is_regex: false, line_number: 224, alias: None },
            TokenDefinition { name: "DOT".to_string(), pattern: ".".to_string(), is_regex: false, line_number: 225, alias: None },
            TokenDefinition { name: "PLUS".to_string(), pattern: "+".to_string(), is_regex: false, line_number: 226, alias: None },
            TokenDefinition { name: "GREATER".to_string(), pattern: ">".to_string(), is_regex: false, line_number: 227, alias: None },
            TokenDefinition { name: "TILDE".to_string(), pattern: "~".to_string(), is_regex: false, line_number: 228, alias: None },
            TokenDefinition { name: "STAR".to_string(), pattern: "*".to_string(), is_regex: false, line_number: 229, alias: None },
            TokenDefinition { name: "PIPE".to_string(), pattern: "|".to_string(), is_regex: false, line_number: 230, alias: None },
            TokenDefinition { name: "BANG".to_string(), pattern: "!".to_string(), is_regex: false, line_number: 231, alias: None },
            TokenDefinition { name: "SLASH".to_string(), pattern: "/".to_string(), is_regex: false, line_number: 232, alias: None },
            TokenDefinition { name: "EQUALS".to_string(), pattern: "=".to_string(), is_regex: false, line_number: 233, alias: None },
            TokenDefinition { name: "AMPERSAND".to_string(), pattern: "&".to_string(), is_regex: false, line_number: 234, alias: None },
            TokenDefinition { name: "MINUS".to_string(), pattern: "-".to_string(), is_regex: false, line_number: 235, alias: None },
        ],
        skip_definitions: vec![
            TokenDefinition { name: "COMMENT".to_string(), pattern: "\\/\\*[\\s\\S]*?\\*\\/".to_string(), is_regex: true, line_number: 51, alias: None },
            TokenDefinition { name: "WHITESPACE".to_string(), pattern: "[ \\t\\r\\n]+".to_string(), is_regex: true, line_number: 52, alias: None },
        ],
        error_definitions: vec![
            TokenDefinition { name: "BAD_STRING".to_string(), pattern: "\"[^\"]*$".to_string(), is_regex: true, line_number: 251, alias: None },
            TokenDefinition { name: "BAD_URL".to_string(), pattern: "url\\([^)]*$".to_string(), is_regex: true, line_number: 252, alias: None },
        ],
        groups,
    }
}
