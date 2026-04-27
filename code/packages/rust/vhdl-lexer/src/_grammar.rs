// AUTO-GENERATED FILE - DO NOT EDIT
// Source family: vhdl
// Regenerate with: grammar-tools generate-rust-compiled-grammars vhdl
//
// This file embeds versioned TokenGrammar values as native Rust data structures.
// Call `token_grammar` instead of reading and parsing grammar files at runtime.

use grammar_tools::token_grammar::TokenGrammar;

pub const SUPPORTED_VERSIONS: &[&str] = &[
    "1987",
    "1993",
    "2002",
    "2008",
    "2019",
];

pub fn token_grammar(version: &str) -> Option<TokenGrammar> {
    match version {
        "1987" => Some(v_1987::token_grammar()),
        "1993" => Some(v_1993::token_grammar()),
        "2002" => Some(v_2002::token_grammar()),
        "2008" => Some(v_2008::token_grammar()),
        "2019" => Some(v_2019::token_grammar()),
        _ => None,
    }
}

mod v_1987 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: vhdl1987.tokens
    // Regenerate with: grammar-tools compile-tokens vhdl1987.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""([^"]|"")*""#.to_string(),
                    is_regex: true,
                    line_number: 66,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BIT_STRING"#.to_string(),
                    pattern: r#"[bBoOxXdD]"[0-9a-fA-F_]+""#.to_string(),
                    is_regex: true,
                    line_number: 85,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHAR_LITERAL"#.to_string(),
                    pattern: r#"'[^']'"#.to_string(),
                    is_regex: true,
                    line_number: 103,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BASED_LITERAL"#.to_string(),
                    pattern: r#"[0-9]+#[0-9a-fA-F_]+(\.[0-9a-fA-F_]+)?#([eE][+-]?[0-9_]+)?"#.to_string(),
                    is_regex: true,
                    line_number: 119,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"REAL_NUMBER"#.to_string(),
                    pattern: r#"[0-9][0-9_]*\.[0-9_]+([eE][+-]?[0-9_]+)?"#.to_string(),
                    is_regex: true,
                    line_number: 123,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NUMBER"#.to_string(),
                    pattern: r#"[0-9][0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 127,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EXTENDED_IDENT"#.to_string(),
                    pattern: r#"\\[^\\]+\\"#.to_string(),
                    is_regex: true,
                    line_number: 146,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[a-zA-Z][a-zA-Z0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 147,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"VAR_ASSIGN"#.to_string(),
                    pattern: r#":="#.to_string(),
                    is_regex: false,
                    line_number: 168,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESS_EQUALS"#.to_string(),
                    pattern: r#"<="#.to_string(),
                    is_regex: false,
                    line_number: 169,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATER_EQUALS"#.to_string(),
                    pattern: r#">="#.to_string(),
                    is_regex: false,
                    line_number: 170,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"ARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 171,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NOT_EQUALS"#.to_string(),
                    pattern: r#"/="#.to_string(),
                    is_regex: false,
                    line_number: 172,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"POWER"#.to_string(),
                    pattern: r#"**"#.to_string(),
                    is_regex: false,
                    line_number: 173,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BOX"#.to_string(),
                    pattern: r#"<>"#.to_string(),
                    is_regex: false,
                    line_number: 174,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 187,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 188,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 189,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 190,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 191,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESS_THAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 192,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATER_THAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 193,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 194,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TICK"#.to_string(),
                    pattern: r#"'"#.to_string(),
                    is_regex: false,
                    line_number: 195,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 196,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 202,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 203,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 204,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 205,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 206,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 207,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 208,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 209,
                    alias: None,
                },
            ],
            keywords: vec![r#"abs"#.to_string(), r#"access"#.to_string(), r#"after"#.to_string(), r#"alias"#.to_string(), r#"all"#.to_string(), r#"and"#.to_string(), r#"architecture"#.to_string(), r#"array"#.to_string(), r#"assert"#.to_string(), r#"attribute"#.to_string(), r#"begin"#.to_string(), r#"block"#.to_string(), r#"body"#.to_string(), r#"buffer"#.to_string(), r#"bus"#.to_string(), r#"case"#.to_string(), r#"component"#.to_string(), r#"configuration"#.to_string(), r#"constant"#.to_string(), r#"disconnect"#.to_string(), r#"downto"#.to_string(), r#"else"#.to_string(), r#"elsif"#.to_string(), r#"end"#.to_string(), r#"entity"#.to_string(), r#"exit"#.to_string(), r#"file"#.to_string(), r#"for"#.to_string(), r#"function"#.to_string(), r#"generate"#.to_string(), r#"generic"#.to_string(), r#"group"#.to_string(), r#"guarded"#.to_string(), r#"if"#.to_string(), r#"impure"#.to_string(), r#"in"#.to_string(), r#"inout"#.to_string(), r#"is"#.to_string(), r#"label"#.to_string(), r#"library"#.to_string(), r#"linkage"#.to_string(), r#"literal"#.to_string(), r#"loop"#.to_string(), r#"map"#.to_string(), r#"mod"#.to_string(), r#"nand"#.to_string(), r#"new"#.to_string(), r#"next"#.to_string(), r#"nor"#.to_string(), r#"not"#.to_string(), r#"null"#.to_string(), r#"of"#.to_string(), r#"on"#.to_string(), r#"open"#.to_string(), r#"or"#.to_string(), r#"others"#.to_string(), r#"out"#.to_string(), r#"package"#.to_string(), r#"port"#.to_string(), r#"postponed"#.to_string(), r#"procedure"#.to_string(), r#"process"#.to_string(), r#"pure"#.to_string(), r#"range"#.to_string(), r#"record"#.to_string(), r#"register"#.to_string(), r#"reject"#.to_string(), r#"rem"#.to_string(), r#"report"#.to_string(), r#"return"#.to_string(), r#"rol"#.to_string(), r#"ror"#.to_string(), r#"select"#.to_string(), r#"severity"#.to_string(), r#"signal"#.to_string(), r#"shared"#.to_string(), r#"sla"#.to_string(), r#"sll"#.to_string(), r#"sra"#.to_string(), r#"srl"#.to_string(), r#"subtype"#.to_string(), r#"then"#.to_string(), r#"to"#.to_string(), r#"transport"#.to_string(), r#"type"#.to_string(), r#"unaffected"#.to_string(), r#"units"#.to_string(), r#"until"#.to_string(), r#"use"#.to_string(), r#"variable"#.to_string(), r#"wait"#.to_string(), r#"when"#.to_string(), r#"while"#.to_string(), r#"with"#.to_string(), r#"xnor"#.to_string(), r#"xor"#.to_string()],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 54,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: Some(r#"none"#.to_string()),
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: false,
            version: 0,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
        }
    }
}

mod v_1993 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: vhdl1993.tokens
    // Regenerate with: grammar-tools compile-tokens vhdl1993.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""([^"]|"")*""#.to_string(),
                    is_regex: true,
                    line_number: 66,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BIT_STRING"#.to_string(),
                    pattern: r#"[bBoOxXdD]"[0-9a-fA-F_]+""#.to_string(),
                    is_regex: true,
                    line_number: 85,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHAR_LITERAL"#.to_string(),
                    pattern: r#"'[^']'"#.to_string(),
                    is_regex: true,
                    line_number: 103,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BASED_LITERAL"#.to_string(),
                    pattern: r#"[0-9]+#[0-9a-fA-F_]+(\.[0-9a-fA-F_]+)?#([eE][+-]?[0-9_]+)?"#.to_string(),
                    is_regex: true,
                    line_number: 119,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"REAL_NUMBER"#.to_string(),
                    pattern: r#"[0-9][0-9_]*\.[0-9_]+([eE][+-]?[0-9_]+)?"#.to_string(),
                    is_regex: true,
                    line_number: 123,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NUMBER"#.to_string(),
                    pattern: r#"[0-9][0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 127,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EXTENDED_IDENT"#.to_string(),
                    pattern: r#"\\[^\\]+\\"#.to_string(),
                    is_regex: true,
                    line_number: 146,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[a-zA-Z][a-zA-Z0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 147,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"VAR_ASSIGN"#.to_string(),
                    pattern: r#":="#.to_string(),
                    is_regex: false,
                    line_number: 168,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESS_EQUALS"#.to_string(),
                    pattern: r#"<="#.to_string(),
                    is_regex: false,
                    line_number: 169,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATER_EQUALS"#.to_string(),
                    pattern: r#">="#.to_string(),
                    is_regex: false,
                    line_number: 170,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"ARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 171,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NOT_EQUALS"#.to_string(),
                    pattern: r#"/="#.to_string(),
                    is_regex: false,
                    line_number: 172,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"POWER"#.to_string(),
                    pattern: r#"**"#.to_string(),
                    is_regex: false,
                    line_number: 173,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BOX"#.to_string(),
                    pattern: r#"<>"#.to_string(),
                    is_regex: false,
                    line_number: 174,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 187,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 188,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 189,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 190,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 191,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESS_THAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 192,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATER_THAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 193,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 194,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TICK"#.to_string(),
                    pattern: r#"'"#.to_string(),
                    is_regex: false,
                    line_number: 195,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 196,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 202,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 203,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 204,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 205,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 206,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 207,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 208,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 209,
                    alias: None,
                },
            ],
            keywords: vec![r#"abs"#.to_string(), r#"access"#.to_string(), r#"after"#.to_string(), r#"alias"#.to_string(), r#"all"#.to_string(), r#"and"#.to_string(), r#"architecture"#.to_string(), r#"array"#.to_string(), r#"assert"#.to_string(), r#"attribute"#.to_string(), r#"begin"#.to_string(), r#"block"#.to_string(), r#"body"#.to_string(), r#"buffer"#.to_string(), r#"bus"#.to_string(), r#"case"#.to_string(), r#"component"#.to_string(), r#"configuration"#.to_string(), r#"constant"#.to_string(), r#"disconnect"#.to_string(), r#"downto"#.to_string(), r#"else"#.to_string(), r#"elsif"#.to_string(), r#"end"#.to_string(), r#"entity"#.to_string(), r#"exit"#.to_string(), r#"file"#.to_string(), r#"for"#.to_string(), r#"function"#.to_string(), r#"generate"#.to_string(), r#"generic"#.to_string(), r#"group"#.to_string(), r#"guarded"#.to_string(), r#"if"#.to_string(), r#"impure"#.to_string(), r#"in"#.to_string(), r#"inout"#.to_string(), r#"is"#.to_string(), r#"label"#.to_string(), r#"library"#.to_string(), r#"linkage"#.to_string(), r#"literal"#.to_string(), r#"loop"#.to_string(), r#"map"#.to_string(), r#"mod"#.to_string(), r#"nand"#.to_string(), r#"new"#.to_string(), r#"next"#.to_string(), r#"nor"#.to_string(), r#"not"#.to_string(), r#"null"#.to_string(), r#"of"#.to_string(), r#"on"#.to_string(), r#"open"#.to_string(), r#"or"#.to_string(), r#"others"#.to_string(), r#"out"#.to_string(), r#"package"#.to_string(), r#"port"#.to_string(), r#"postponed"#.to_string(), r#"procedure"#.to_string(), r#"process"#.to_string(), r#"pure"#.to_string(), r#"range"#.to_string(), r#"record"#.to_string(), r#"register"#.to_string(), r#"reject"#.to_string(), r#"rem"#.to_string(), r#"report"#.to_string(), r#"return"#.to_string(), r#"rol"#.to_string(), r#"ror"#.to_string(), r#"select"#.to_string(), r#"severity"#.to_string(), r#"signal"#.to_string(), r#"shared"#.to_string(), r#"sla"#.to_string(), r#"sll"#.to_string(), r#"sra"#.to_string(), r#"srl"#.to_string(), r#"subtype"#.to_string(), r#"then"#.to_string(), r#"to"#.to_string(), r#"transport"#.to_string(), r#"type"#.to_string(), r#"unaffected"#.to_string(), r#"units"#.to_string(), r#"until"#.to_string(), r#"use"#.to_string(), r#"variable"#.to_string(), r#"wait"#.to_string(), r#"when"#.to_string(), r#"while"#.to_string(), r#"with"#.to_string(), r#"xnor"#.to_string(), r#"xor"#.to_string()],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 54,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: Some(r#"none"#.to_string()),
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: false,
            version: 0,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
        }
    }
}

mod v_2002 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: vhdl2002.tokens
    // Regenerate with: grammar-tools compile-tokens vhdl2002.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""([^"]|"")*""#.to_string(),
                    is_regex: true,
                    line_number: 66,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BIT_STRING"#.to_string(),
                    pattern: r#"[bBoOxXdD]"[0-9a-fA-F_]+""#.to_string(),
                    is_regex: true,
                    line_number: 85,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHAR_LITERAL"#.to_string(),
                    pattern: r#"'[^']'"#.to_string(),
                    is_regex: true,
                    line_number: 103,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BASED_LITERAL"#.to_string(),
                    pattern: r#"[0-9]+#[0-9a-fA-F_]+(\.[0-9a-fA-F_]+)?#([eE][+-]?[0-9_]+)?"#.to_string(),
                    is_regex: true,
                    line_number: 119,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"REAL_NUMBER"#.to_string(),
                    pattern: r#"[0-9][0-9_]*\.[0-9_]+([eE][+-]?[0-9_]+)?"#.to_string(),
                    is_regex: true,
                    line_number: 123,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NUMBER"#.to_string(),
                    pattern: r#"[0-9][0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 127,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EXTENDED_IDENT"#.to_string(),
                    pattern: r#"\\[^\\]+\\"#.to_string(),
                    is_regex: true,
                    line_number: 146,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[a-zA-Z][a-zA-Z0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 147,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"VAR_ASSIGN"#.to_string(),
                    pattern: r#":="#.to_string(),
                    is_regex: false,
                    line_number: 168,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESS_EQUALS"#.to_string(),
                    pattern: r#"<="#.to_string(),
                    is_regex: false,
                    line_number: 169,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATER_EQUALS"#.to_string(),
                    pattern: r#">="#.to_string(),
                    is_regex: false,
                    line_number: 170,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"ARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 171,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NOT_EQUALS"#.to_string(),
                    pattern: r#"/="#.to_string(),
                    is_regex: false,
                    line_number: 172,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"POWER"#.to_string(),
                    pattern: r#"**"#.to_string(),
                    is_regex: false,
                    line_number: 173,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BOX"#.to_string(),
                    pattern: r#"<>"#.to_string(),
                    is_regex: false,
                    line_number: 174,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 187,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 188,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 189,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 190,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 191,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESS_THAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 192,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATER_THAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 193,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 194,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TICK"#.to_string(),
                    pattern: r#"'"#.to_string(),
                    is_regex: false,
                    line_number: 195,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 196,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 202,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 203,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 204,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 205,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 206,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 207,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 208,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 209,
                    alias: None,
                },
            ],
            keywords: vec![r#"abs"#.to_string(), r#"access"#.to_string(), r#"after"#.to_string(), r#"alias"#.to_string(), r#"all"#.to_string(), r#"and"#.to_string(), r#"architecture"#.to_string(), r#"array"#.to_string(), r#"assert"#.to_string(), r#"attribute"#.to_string(), r#"begin"#.to_string(), r#"block"#.to_string(), r#"body"#.to_string(), r#"buffer"#.to_string(), r#"bus"#.to_string(), r#"case"#.to_string(), r#"component"#.to_string(), r#"configuration"#.to_string(), r#"constant"#.to_string(), r#"disconnect"#.to_string(), r#"downto"#.to_string(), r#"else"#.to_string(), r#"elsif"#.to_string(), r#"end"#.to_string(), r#"entity"#.to_string(), r#"exit"#.to_string(), r#"file"#.to_string(), r#"for"#.to_string(), r#"function"#.to_string(), r#"generate"#.to_string(), r#"generic"#.to_string(), r#"group"#.to_string(), r#"guarded"#.to_string(), r#"if"#.to_string(), r#"impure"#.to_string(), r#"in"#.to_string(), r#"inout"#.to_string(), r#"is"#.to_string(), r#"label"#.to_string(), r#"library"#.to_string(), r#"linkage"#.to_string(), r#"literal"#.to_string(), r#"loop"#.to_string(), r#"map"#.to_string(), r#"mod"#.to_string(), r#"nand"#.to_string(), r#"new"#.to_string(), r#"next"#.to_string(), r#"nor"#.to_string(), r#"not"#.to_string(), r#"null"#.to_string(), r#"of"#.to_string(), r#"on"#.to_string(), r#"open"#.to_string(), r#"or"#.to_string(), r#"others"#.to_string(), r#"out"#.to_string(), r#"package"#.to_string(), r#"port"#.to_string(), r#"postponed"#.to_string(), r#"procedure"#.to_string(), r#"process"#.to_string(), r#"pure"#.to_string(), r#"range"#.to_string(), r#"record"#.to_string(), r#"register"#.to_string(), r#"reject"#.to_string(), r#"rem"#.to_string(), r#"report"#.to_string(), r#"return"#.to_string(), r#"rol"#.to_string(), r#"ror"#.to_string(), r#"select"#.to_string(), r#"severity"#.to_string(), r#"signal"#.to_string(), r#"shared"#.to_string(), r#"sla"#.to_string(), r#"sll"#.to_string(), r#"sra"#.to_string(), r#"srl"#.to_string(), r#"subtype"#.to_string(), r#"then"#.to_string(), r#"to"#.to_string(), r#"transport"#.to_string(), r#"type"#.to_string(), r#"unaffected"#.to_string(), r#"units"#.to_string(), r#"until"#.to_string(), r#"use"#.to_string(), r#"variable"#.to_string(), r#"wait"#.to_string(), r#"when"#.to_string(), r#"while"#.to_string(), r#"with"#.to_string(), r#"xnor"#.to_string(), r#"xor"#.to_string()],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 54,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: Some(r#"none"#.to_string()),
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: false,
            version: 0,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
        }
    }
}

mod v_2008 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: vhdl2008.tokens
    // Regenerate with: grammar-tools compile-tokens vhdl2008.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""([^"]|"")*""#.to_string(),
                    is_regex: true,
                    line_number: 66,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BIT_STRING"#.to_string(),
                    pattern: r#"[bBoOxXdD]"[0-9a-fA-F_]+""#.to_string(),
                    is_regex: true,
                    line_number: 85,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHAR_LITERAL"#.to_string(),
                    pattern: r#"'[^']'"#.to_string(),
                    is_regex: true,
                    line_number: 103,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BASED_LITERAL"#.to_string(),
                    pattern: r#"[0-9]+#[0-9a-fA-F_]+(\.[0-9a-fA-F_]+)?#([eE][+-]?[0-9_]+)?"#.to_string(),
                    is_regex: true,
                    line_number: 119,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"REAL_NUMBER"#.to_string(),
                    pattern: r#"[0-9][0-9_]*\.[0-9_]+([eE][+-]?[0-9_]+)?"#.to_string(),
                    is_regex: true,
                    line_number: 123,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NUMBER"#.to_string(),
                    pattern: r#"[0-9][0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 127,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EXTENDED_IDENT"#.to_string(),
                    pattern: r#"\\[^\\]+\\"#.to_string(),
                    is_regex: true,
                    line_number: 146,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[a-zA-Z][a-zA-Z0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 147,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"VAR_ASSIGN"#.to_string(),
                    pattern: r#":="#.to_string(),
                    is_regex: false,
                    line_number: 168,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESS_EQUALS"#.to_string(),
                    pattern: r#"<="#.to_string(),
                    is_regex: false,
                    line_number: 169,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATER_EQUALS"#.to_string(),
                    pattern: r#">="#.to_string(),
                    is_regex: false,
                    line_number: 170,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"ARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 171,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NOT_EQUALS"#.to_string(),
                    pattern: r#"/="#.to_string(),
                    is_regex: false,
                    line_number: 172,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"POWER"#.to_string(),
                    pattern: r#"**"#.to_string(),
                    is_regex: false,
                    line_number: 173,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BOX"#.to_string(),
                    pattern: r#"<>"#.to_string(),
                    is_regex: false,
                    line_number: 174,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 187,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 188,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 189,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 190,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 191,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESS_THAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 192,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATER_THAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 193,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 194,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TICK"#.to_string(),
                    pattern: r#"'"#.to_string(),
                    is_regex: false,
                    line_number: 195,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 196,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 202,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 203,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 204,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 205,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 206,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 207,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 208,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 209,
                    alias: None,
                },
            ],
            keywords: vec![r#"abs"#.to_string(), r#"access"#.to_string(), r#"after"#.to_string(), r#"alias"#.to_string(), r#"all"#.to_string(), r#"and"#.to_string(), r#"architecture"#.to_string(), r#"array"#.to_string(), r#"assert"#.to_string(), r#"attribute"#.to_string(), r#"begin"#.to_string(), r#"block"#.to_string(), r#"body"#.to_string(), r#"buffer"#.to_string(), r#"bus"#.to_string(), r#"case"#.to_string(), r#"component"#.to_string(), r#"configuration"#.to_string(), r#"constant"#.to_string(), r#"disconnect"#.to_string(), r#"downto"#.to_string(), r#"else"#.to_string(), r#"elsif"#.to_string(), r#"end"#.to_string(), r#"entity"#.to_string(), r#"exit"#.to_string(), r#"file"#.to_string(), r#"for"#.to_string(), r#"function"#.to_string(), r#"generate"#.to_string(), r#"generic"#.to_string(), r#"group"#.to_string(), r#"guarded"#.to_string(), r#"if"#.to_string(), r#"impure"#.to_string(), r#"in"#.to_string(), r#"inout"#.to_string(), r#"is"#.to_string(), r#"label"#.to_string(), r#"library"#.to_string(), r#"linkage"#.to_string(), r#"literal"#.to_string(), r#"loop"#.to_string(), r#"map"#.to_string(), r#"mod"#.to_string(), r#"nand"#.to_string(), r#"new"#.to_string(), r#"next"#.to_string(), r#"nor"#.to_string(), r#"not"#.to_string(), r#"null"#.to_string(), r#"of"#.to_string(), r#"on"#.to_string(), r#"open"#.to_string(), r#"or"#.to_string(), r#"others"#.to_string(), r#"out"#.to_string(), r#"package"#.to_string(), r#"port"#.to_string(), r#"postponed"#.to_string(), r#"procedure"#.to_string(), r#"process"#.to_string(), r#"pure"#.to_string(), r#"range"#.to_string(), r#"record"#.to_string(), r#"register"#.to_string(), r#"reject"#.to_string(), r#"rem"#.to_string(), r#"report"#.to_string(), r#"return"#.to_string(), r#"rol"#.to_string(), r#"ror"#.to_string(), r#"select"#.to_string(), r#"severity"#.to_string(), r#"signal"#.to_string(), r#"shared"#.to_string(), r#"sla"#.to_string(), r#"sll"#.to_string(), r#"sra"#.to_string(), r#"srl"#.to_string(), r#"subtype"#.to_string(), r#"then"#.to_string(), r#"to"#.to_string(), r#"transport"#.to_string(), r#"type"#.to_string(), r#"unaffected"#.to_string(), r#"units"#.to_string(), r#"until"#.to_string(), r#"use"#.to_string(), r#"variable"#.to_string(), r#"wait"#.to_string(), r#"when"#.to_string(), r#"while"#.to_string(), r#"with"#.to_string(), r#"xnor"#.to_string(), r#"xor"#.to_string()],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 54,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: Some(r#"none"#.to_string()),
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: false,
            version: 0,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
        }
    }
}

mod v_2019 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: vhdl2019.tokens
    // Regenerate with: grammar-tools compile-tokens vhdl2019.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""([^"]|"")*""#.to_string(),
                    is_regex: true,
                    line_number: 66,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BIT_STRING"#.to_string(),
                    pattern: r#"[bBoOxXdD]"[0-9a-fA-F_]+""#.to_string(),
                    is_regex: true,
                    line_number: 85,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHAR_LITERAL"#.to_string(),
                    pattern: r#"'[^']'"#.to_string(),
                    is_regex: true,
                    line_number: 103,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BASED_LITERAL"#.to_string(),
                    pattern: r#"[0-9]+#[0-9a-fA-F_]+(\.[0-9a-fA-F_]+)?#([eE][+-]?[0-9_]+)?"#.to_string(),
                    is_regex: true,
                    line_number: 119,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"REAL_NUMBER"#.to_string(),
                    pattern: r#"[0-9][0-9_]*\.[0-9_]+([eE][+-]?[0-9_]+)?"#.to_string(),
                    is_regex: true,
                    line_number: 123,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NUMBER"#.to_string(),
                    pattern: r#"[0-9][0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 127,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EXTENDED_IDENT"#.to_string(),
                    pattern: r#"\\[^\\]+\\"#.to_string(),
                    is_regex: true,
                    line_number: 146,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[a-zA-Z][a-zA-Z0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 147,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"VAR_ASSIGN"#.to_string(),
                    pattern: r#":="#.to_string(),
                    is_regex: false,
                    line_number: 168,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESS_EQUALS"#.to_string(),
                    pattern: r#"<="#.to_string(),
                    is_regex: false,
                    line_number: 169,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATER_EQUALS"#.to_string(),
                    pattern: r#">="#.to_string(),
                    is_regex: false,
                    line_number: 170,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"ARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 171,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NOT_EQUALS"#.to_string(),
                    pattern: r#"/="#.to_string(),
                    is_regex: false,
                    line_number: 172,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"POWER"#.to_string(),
                    pattern: r#"**"#.to_string(),
                    is_regex: false,
                    line_number: 173,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BOX"#.to_string(),
                    pattern: r#"<>"#.to_string(),
                    is_regex: false,
                    line_number: 174,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 187,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 188,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 189,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 190,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 191,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESS_THAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 192,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATER_THAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 193,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 194,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TICK"#.to_string(),
                    pattern: r#"'"#.to_string(),
                    is_regex: false,
                    line_number: 195,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 196,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 202,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 203,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 204,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 205,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 206,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 207,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 208,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 209,
                    alias: None,
                },
            ],
            keywords: vec![r#"abs"#.to_string(), r#"access"#.to_string(), r#"after"#.to_string(), r#"alias"#.to_string(), r#"all"#.to_string(), r#"and"#.to_string(), r#"architecture"#.to_string(), r#"array"#.to_string(), r#"assert"#.to_string(), r#"attribute"#.to_string(), r#"begin"#.to_string(), r#"block"#.to_string(), r#"body"#.to_string(), r#"buffer"#.to_string(), r#"bus"#.to_string(), r#"case"#.to_string(), r#"component"#.to_string(), r#"configuration"#.to_string(), r#"constant"#.to_string(), r#"disconnect"#.to_string(), r#"downto"#.to_string(), r#"else"#.to_string(), r#"elsif"#.to_string(), r#"end"#.to_string(), r#"entity"#.to_string(), r#"exit"#.to_string(), r#"file"#.to_string(), r#"for"#.to_string(), r#"function"#.to_string(), r#"generate"#.to_string(), r#"generic"#.to_string(), r#"group"#.to_string(), r#"guarded"#.to_string(), r#"if"#.to_string(), r#"impure"#.to_string(), r#"in"#.to_string(), r#"inout"#.to_string(), r#"is"#.to_string(), r#"label"#.to_string(), r#"library"#.to_string(), r#"linkage"#.to_string(), r#"literal"#.to_string(), r#"loop"#.to_string(), r#"map"#.to_string(), r#"mod"#.to_string(), r#"nand"#.to_string(), r#"new"#.to_string(), r#"next"#.to_string(), r#"nor"#.to_string(), r#"not"#.to_string(), r#"null"#.to_string(), r#"of"#.to_string(), r#"on"#.to_string(), r#"open"#.to_string(), r#"or"#.to_string(), r#"others"#.to_string(), r#"out"#.to_string(), r#"package"#.to_string(), r#"port"#.to_string(), r#"postponed"#.to_string(), r#"procedure"#.to_string(), r#"process"#.to_string(), r#"pure"#.to_string(), r#"range"#.to_string(), r#"record"#.to_string(), r#"register"#.to_string(), r#"reject"#.to_string(), r#"rem"#.to_string(), r#"report"#.to_string(), r#"return"#.to_string(), r#"rol"#.to_string(), r#"ror"#.to_string(), r#"select"#.to_string(), r#"severity"#.to_string(), r#"signal"#.to_string(), r#"shared"#.to_string(), r#"sla"#.to_string(), r#"sll"#.to_string(), r#"sra"#.to_string(), r#"srl"#.to_string(), r#"subtype"#.to_string(), r#"then"#.to_string(), r#"to"#.to_string(), r#"transport"#.to_string(), r#"type"#.to_string(), r#"unaffected"#.to_string(), r#"units"#.to_string(), r#"until"#.to_string(), r#"use"#.to_string(), r#"variable"#.to_string(), r#"wait"#.to_string(), r#"when"#.to_string(), r#"while"#.to_string(), r#"with"#.to_string(), r#"xnor"#.to_string(), r#"xor"#.to_string()],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 54,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: Some(r#"none"#.to_string()),
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: false,
            version: 0,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
        }
    }
}

