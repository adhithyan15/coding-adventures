// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use std::collections::HashMap;
use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition, PatternGroup};

pub fn SqlTokens() -> TokenGrammar {
    let groups = HashMap::new();
    TokenGrammar {
        version: 1,
        case_insensitive: true,
        case_sensitive: true,
        mode: None,
        escapes: None,
        keywords: vec!["SELECT".to_string(), "FROM".to_string(), "WHERE".to_string(), "GROUP".to_string(), "BY".to_string(), "HAVING".to_string(), "ORDER".to_string(), "LIMIT".to_string(), "OFFSET".to_string(), "INSERT".to_string(), "INTO".to_string(), "VALUES".to_string(), "UPDATE".to_string(), "SET".to_string(), "DELETE".to_string(), "CREATE".to_string(), "DROP".to_string(), "TABLE".to_string(), "IF".to_string(), "EXISTS".to_string(), "NOT".to_string(), "AND".to_string(), "OR".to_string(), "NULL".to_string(), "IS".to_string(), "IN".to_string(), "BETWEEN".to_string(), "LIKE".to_string(), "AS".to_string(), "DISTINCT".to_string(), "ALL".to_string(), "UNION".to_string(), "INTERSECT".to_string(), "EXCEPT".to_string(), "JOIN".to_string(), "INNER".to_string(), "LEFT".to_string(), "RIGHT".to_string(), "OUTER".to_string(), "CROSS".to_string(), "FULL".to_string(), "ON".to_string(), "ASC".to_string(), "DESC".to_string(), "TRUE".to_string(), "FALSE".to_string(), "CASE".to_string(), "WHEN".to_string(), "THEN".to_string(), "ELSE".to_string(), "END".to_string(), "PRIMARY".to_string(), "KEY".to_string(), "UNIQUE".to_string(), "DEFAULT".to_string()],
        reserved_keywords: vec![],
        definitions: vec![
            TokenDefinition { name: "NAME".to_string(), pattern: "[a-zA-Z_][a-zA-Z0-9_]*".to_string(), is_regex: true, line_number: 12, alias: None },
            TokenDefinition { name: "NUMBER".to_string(), pattern: "[0-9]+(\\.[0-9]+)?".to_string(), is_regex: true, line_number: 13, alias: None },
            TokenDefinition { name: "STRING_SQ".to_string(), pattern: "'([^'\\\\]|\\\\.)*'".to_string(), is_regex: true, line_number: 14, alias: Some("STRING".to_string()) },
            TokenDefinition { name: "QUOTED_ID".to_string(), pattern: "`[^`]+`".to_string(), is_regex: true, line_number: 15, alias: Some("NAME".to_string()) },
            TokenDefinition { name: "LESS_EQUALS".to_string(), pattern: "<=".to_string(), is_regex: false, line_number: 17, alias: None },
            TokenDefinition { name: "GREATER_EQUALS".to_string(), pattern: ">=".to_string(), is_regex: false, line_number: 18, alias: None },
            TokenDefinition { name: "NOT_EQUALS".to_string(), pattern: "!=".to_string(), is_regex: false, line_number: 19, alias: None },
            TokenDefinition { name: "NEQ_ANSI".to_string(), pattern: "<>".to_string(), is_regex: false, line_number: 20, alias: Some("NOT_EQUALS".to_string()) },
            TokenDefinition { name: "EQUALS".to_string(), pattern: "=".to_string(), is_regex: false, line_number: 22, alias: None },
            TokenDefinition { name: "LESS_THAN".to_string(), pattern: "<".to_string(), is_regex: false, line_number: 23, alias: None },
            TokenDefinition { name: "GREATER_THAN".to_string(), pattern: ">".to_string(), is_regex: false, line_number: 24, alias: None },
            TokenDefinition { name: "PLUS".to_string(), pattern: "+".to_string(), is_regex: false, line_number: 25, alias: None },
            TokenDefinition { name: "MINUS".to_string(), pattern: "-".to_string(), is_regex: false, line_number: 26, alias: None },
            TokenDefinition { name: "STAR".to_string(), pattern: "*".to_string(), is_regex: false, line_number: 27, alias: None },
            TokenDefinition { name: "SLASH".to_string(), pattern: "/".to_string(), is_regex: false, line_number: 28, alias: None },
            TokenDefinition { name: "PERCENT".to_string(), pattern: "%".to_string(), is_regex: false, line_number: 29, alias: None },
            TokenDefinition { name: "LPAREN".to_string(), pattern: "(".to_string(), is_regex: false, line_number: 31, alias: None },
            TokenDefinition { name: "RPAREN".to_string(), pattern: ")".to_string(), is_regex: false, line_number: 32, alias: None },
            TokenDefinition { name: "COMMA".to_string(), pattern: ",".to_string(), is_regex: false, line_number: 33, alias: None },
            TokenDefinition { name: "SEMICOLON".to_string(), pattern: ";".to_string(), is_regex: false, line_number: 34, alias: None },
            TokenDefinition { name: "DOT".to_string(), pattern: ".".to_string(), is_regex: false, line_number: 35, alias: None },
        ],
        skip_definitions: vec![
            TokenDefinition { name: "WHITESPACE".to_string(), pattern: "[ \\t\\r\\n]+".to_string(), is_regex: true, line_number: 95, alias: None },
            TokenDefinition { name: "LINE_COMMENT".to_string(), pattern: "--[^\\n]*".to_string(), is_regex: true, line_number: 96, alias: None },
            TokenDefinition { name: "BLOCK_COMMENT".to_string(), pattern: "\\x2f\\*([^*]|\\*[^\\x2f])*\\*\\x2f".to_string(), is_regex: true, line_number: 97, alias: None },
        ],
        error_definitions: vec![
        ],
        groups,
    }
}
