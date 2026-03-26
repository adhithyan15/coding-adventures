// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use std::collections::HashMap;
use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition, PatternGroup};

pub fn VhdlTokens() -> TokenGrammar {
    let groups = HashMap::new();
    TokenGrammar {
        version: 0,
        case_insensitive: false,
        case_sensitive: false,
        mode: None,
        escapes: Some("none".to_string()),
        keywords: vec!["abs".to_string(), "access".to_string(), "after".to_string(), "alias".to_string(), "all".to_string(), "and".to_string(), "architecture".to_string(), "array".to_string(), "assert".to_string(), "attribute".to_string(), "begin".to_string(), "block".to_string(), "body".to_string(), "buffer".to_string(), "bus".to_string(), "case".to_string(), "component".to_string(), "configuration".to_string(), "constant".to_string(), "disconnect".to_string(), "downto".to_string(), "else".to_string(), "elsif".to_string(), "end".to_string(), "entity".to_string(), "exit".to_string(), "file".to_string(), "for".to_string(), "function".to_string(), "generate".to_string(), "generic".to_string(), "group".to_string(), "guarded".to_string(), "if".to_string(), "impure".to_string(), "in".to_string(), "inout".to_string(), "is".to_string(), "label".to_string(), "library".to_string(), "linkage".to_string(), "literal".to_string(), "loop".to_string(), "map".to_string(), "mod".to_string(), "nand".to_string(), "new".to_string(), "next".to_string(), "nor".to_string(), "not".to_string(), "null".to_string(), "of".to_string(), "on".to_string(), "open".to_string(), "or".to_string(), "others".to_string(), "out".to_string(), "package".to_string(), "port".to_string(), "postponed".to_string(), "procedure".to_string(), "process".to_string(), "pure".to_string(), "range".to_string(), "record".to_string(), "register".to_string(), "reject".to_string(), "rem".to_string(), "report".to_string(), "return".to_string(), "rol".to_string(), "ror".to_string(), "select".to_string(), "severity".to_string(), "signal".to_string(), "shared".to_string(), "sla".to_string(), "sll".to_string(), "sra".to_string(), "srl".to_string(), "subtype".to_string(), "then".to_string(), "to".to_string(), "transport".to_string(), "type".to_string(), "unaffected".to_string(), "units".to_string(), "until".to_string(), "use".to_string(), "variable".to_string(), "wait".to_string(), "when".to_string(), "while".to_string(), "with".to_string(), "xnor".to_string(), "xor".to_string()],
        reserved_keywords: vec![],
        definitions: vec![
            TokenDefinition { name: "STRING".to_string(), pattern: "\"([^\"]|\"\")*\"".to_string(), is_regex: true, line_number: 63, alias: None },
            TokenDefinition { name: "BIT_STRING".to_string(), pattern: "[bBoOxXdD]\"[0-9a-fA-F_]+\"".to_string(), is_regex: true, line_number: 82, alias: None },
            TokenDefinition { name: "CHAR_LITERAL".to_string(), pattern: "'[^']'".to_string(), is_regex: true, line_number: 100, alias: None },
            TokenDefinition { name: "BASED_LITERAL".to_string(), pattern: "[0-9]+#[0-9a-fA-F_]+(\\.[0-9a-fA-F_]+)?#([eE][+-]?[0-9_]+)?".to_string(), is_regex: true, line_number: 116, alias: None },
            TokenDefinition { name: "REAL_NUMBER".to_string(), pattern: "[0-9][0-9_]*\\.[0-9_]+([eE][+-]?[0-9_]+)?".to_string(), is_regex: true, line_number: 120, alias: None },
            TokenDefinition { name: "NUMBER".to_string(), pattern: "[0-9][0-9_]*".to_string(), is_regex: true, line_number: 124, alias: None },
            TokenDefinition { name: "EXTENDED_IDENT".to_string(), pattern: "\\\\[^\\\\]+\\\\".to_string(), is_regex: true, line_number: 143, alias: None },
            TokenDefinition { name: "NAME".to_string(), pattern: "[a-zA-Z][a-zA-Z0-9_]*".to_string(), is_regex: true, line_number: 144, alias: None },
            TokenDefinition { name: "VAR_ASSIGN".to_string(), pattern: ":=".to_string(), is_regex: false, line_number: 165, alias: None },
            TokenDefinition { name: "LESS_EQUALS".to_string(), pattern: "<=".to_string(), is_regex: false, line_number: 166, alias: None },
            TokenDefinition { name: "GREATER_EQUALS".to_string(), pattern: ">=".to_string(), is_regex: false, line_number: 167, alias: None },
            TokenDefinition { name: "ARROW".to_string(), pattern: "=>".to_string(), is_regex: false, line_number: 168, alias: None },
            TokenDefinition { name: "NOT_EQUALS".to_string(), pattern: "/=".to_string(), is_regex: false, line_number: 169, alias: None },
            TokenDefinition { name: "POWER".to_string(), pattern: "**".to_string(), is_regex: false, line_number: 170, alias: None },
            TokenDefinition { name: "BOX".to_string(), pattern: "<>".to_string(), is_regex: false, line_number: 171, alias: None },
            TokenDefinition { name: "PLUS".to_string(), pattern: "+".to_string(), is_regex: false, line_number: 184, alias: None },
            TokenDefinition { name: "MINUS".to_string(), pattern: "-".to_string(), is_regex: false, line_number: 185, alias: None },
            TokenDefinition { name: "STAR".to_string(), pattern: "*".to_string(), is_regex: false, line_number: 186, alias: None },
            TokenDefinition { name: "SLASH".to_string(), pattern: "/".to_string(), is_regex: false, line_number: 187, alias: None },
            TokenDefinition { name: "AMPERSAND".to_string(), pattern: "&".to_string(), is_regex: false, line_number: 188, alias: None },
            TokenDefinition { name: "LESS_THAN".to_string(), pattern: "<".to_string(), is_regex: false, line_number: 189, alias: None },
            TokenDefinition { name: "GREATER_THAN".to_string(), pattern: ">".to_string(), is_regex: false, line_number: 190, alias: None },
            TokenDefinition { name: "EQUALS".to_string(), pattern: "=".to_string(), is_regex: false, line_number: 191, alias: None },
            TokenDefinition { name: "TICK".to_string(), pattern: "'".to_string(), is_regex: false, line_number: 192, alias: None },
            TokenDefinition { name: "PIPE".to_string(), pattern: "|".to_string(), is_regex: false, line_number: 193, alias: None },
            TokenDefinition { name: "LPAREN".to_string(), pattern: "(".to_string(), is_regex: false, line_number: 199, alias: None },
            TokenDefinition { name: "RPAREN".to_string(), pattern: ")".to_string(), is_regex: false, line_number: 200, alias: None },
            TokenDefinition { name: "LBRACKET".to_string(), pattern: "[".to_string(), is_regex: false, line_number: 201, alias: None },
            TokenDefinition { name: "RBRACKET".to_string(), pattern: "]".to_string(), is_regex: false, line_number: 202, alias: None },
            TokenDefinition { name: "SEMICOLON".to_string(), pattern: ";".to_string(), is_regex: false, line_number: 203, alias: None },
            TokenDefinition { name: "COMMA".to_string(), pattern: ",".to_string(), is_regex: false, line_number: 204, alias: None },
            TokenDefinition { name: "DOT".to_string(), pattern: ".".to_string(), is_regex: false, line_number: 205, alias: None },
            TokenDefinition { name: "COLON".to_string(), pattern: ":".to_string(), is_regex: false, line_number: 206, alias: None },
        ],
        skip_definitions: vec![
            TokenDefinition { name: "COMMENT".to_string(), pattern: "--[^\\n]*".to_string(), is_regex: true, line_number: 50, alias: None },
            TokenDefinition { name: "WHITESPACE".to_string(), pattern: "[ \\t\\r\\n]+".to_string(), is_regex: true, line_number: 51, alias: None },
        ],
        error_definitions: vec![
        ],
        groups,
    }
}
