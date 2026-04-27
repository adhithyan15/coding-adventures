// AUTO-GENERATED FILE - DO NOT EDIT
// Source family: haskell
// Regenerate with: grammar-tools generate-rust-compiled-grammars haskell
//
// This file embeds versioned TokenGrammar values as native Rust data structures.
// Call `token_grammar` instead of reading and parsing grammar files at runtime.

use grammar_tools::token_grammar::TokenGrammar;

pub const SUPPORTED_VERSIONS: &[&str] = &[
    "1.0",
    "1.1",
    "1.2",
    "1.3",
    "1.4",
    "98",
    "2010",
];

pub fn token_grammar(version: &str) -> Option<TokenGrammar> {
    match version {
        "1.0" => Some(v_1_0::token_grammar()),
        "1.1" => Some(v_1_1::token_grammar()),
        "1.2" => Some(v_1_2::token_grammar()),
        "1.3" => Some(v_1_3::token_grammar()),
        "1.4" => Some(v_1_4::token_grammar()),
        "98" => Some(v_98::token_grammar()),
        "2010" => Some(v_2010::token_grammar()),
        _ => None,
    }
}

mod v_1_0 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell1.0.tokens
    // Regenerate with: grammar-tools compile-tokens haskell1.0.tokens
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
                    name: r#"FLOAT"#.to_string(),
                    pattern: r#"[0-9]+\.[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 29,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INTEGER"#.to_string(),
                    pattern: r#"[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 30,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHARACTER"#.to_string(),
                    pattern: r#"'(?:[^'\\]|\\.)'"#.to_string(),
                    is_regex: true,
                    line_number: 31,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""(?:[^"\\]|\\.)*""#.to_string(),
                    is_regex: true,
                    line_number: 32,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LAMBDA"#.to_string(),
                    pattern: r#"\\"#.to_string(),
                    is_regex: false,
                    line_number: 37,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RARROW"#.to_string(),
                    pattern: r#"->"#.to_string(),
                    is_regex: false,
                    line_number: 38,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LARROW"#.to_string(),
                    pattern: r#"<-"#.to_string(),
                    is_regex: false,
                    line_number: 39,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 40,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_COLON"#.to_string(),
                    pattern: r#"::"#.to_string(),
                    is_regex: false,
                    line_number: 41,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_DOT"#.to_string(),
                    pattern: r#".."#.to_string(),
                    is_regex: false,
                    line_number: 42,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 43,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQ"#.to_string(),
                    pattern: r#"=="#.to_string(),
                    is_regex: false,
                    line_number: 44,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 45,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 46,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 47,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 48,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 49,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 50,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CARET"#.to_string(),
                    pattern: r#"^"#.to_string(),
                    is_regex: false,
                    line_number: 51,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TILDE"#.to_string(),
                    pattern: r#"~"#.to_string(),
                    is_regex: false,
                    line_number: 52,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BANG"#.to_string(),
                    pattern: r#"!"#.to_string(),
                    is_regex: false,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESSTHAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 54,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATERTHAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 55,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 56,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 57,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 58,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 59,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 60,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 61,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 62,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 63,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACE"#.to_string(),
                    pattern: r#"{"#.to_string(),
                    is_regex: false,
                    line_number: 64,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACE"#.to_string(),
                    pattern: r#"}"#.to_string(),
                    is_regex: false,
                    line_number: 65,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[A-Za-z_][A-Za-z0-9_']*"#.to_string(),
                    is_regex: true,
                    line_number: 70,
                    alias: None,
                },
            ],
            keywords: vec![r#"as"#.to_string(), r#"case"#.to_string(), r#"class"#.to_string(), r#"data"#.to_string(), r#"do"#.to_string(), r#"else"#.to_string(), r#"foreign"#.to_string(), r#"if"#.to_string(), r#"import"#.to_string(), r#"in"#.to_string(), r#"infix"#.to_string(), r#"infixl"#.to_string(), r#"infixr"#.to_string(), r#"let"#.to_string(), r#"module"#.to_string(), r#"of"#.to_string(), r#"then"#.to_string(), r#"type"#.to_string(), r#"where"#.to_string()],
            mode: Some(r#"layout"#.to_string()),
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"LINE_COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 22,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BLOCK_COMMENT"#.to_string(),
                    pattern: r#"\{-[\s\S]*?\-\}"#.to_string(),
                    is_regex: true,
                    line_number: 23,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r]+"#.to_string(),
                    is_regex: true,
                    line_number: 24,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![r#"let"#.to_string(), r#"where"#.to_string(), r#"do"#.to_string(), r#"of"#.to_string()],
        }
    }
}

mod v_1_1 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell1.1.tokens
    // Regenerate with: grammar-tools compile-tokens haskell1.1.tokens
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
                    name: r#"FLOAT"#.to_string(),
                    pattern: r#"[0-9]+\.[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 29,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INTEGER"#.to_string(),
                    pattern: r#"[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 30,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHARACTER"#.to_string(),
                    pattern: r#"'(?:[^'\\]|\\.)'"#.to_string(),
                    is_regex: true,
                    line_number: 31,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""(?:[^"\\]|\\.)*""#.to_string(),
                    is_regex: true,
                    line_number: 32,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LAMBDA"#.to_string(),
                    pattern: r#"\\"#.to_string(),
                    is_regex: false,
                    line_number: 37,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RARROW"#.to_string(),
                    pattern: r#"->"#.to_string(),
                    is_regex: false,
                    line_number: 38,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LARROW"#.to_string(),
                    pattern: r#"<-"#.to_string(),
                    is_regex: false,
                    line_number: 39,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 40,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_COLON"#.to_string(),
                    pattern: r#"::"#.to_string(),
                    is_regex: false,
                    line_number: 41,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_DOT"#.to_string(),
                    pattern: r#".."#.to_string(),
                    is_regex: false,
                    line_number: 42,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 43,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQ"#.to_string(),
                    pattern: r#"=="#.to_string(),
                    is_regex: false,
                    line_number: 44,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 45,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 46,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 47,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 48,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 49,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 50,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CARET"#.to_string(),
                    pattern: r#"^"#.to_string(),
                    is_regex: false,
                    line_number: 51,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TILDE"#.to_string(),
                    pattern: r#"~"#.to_string(),
                    is_regex: false,
                    line_number: 52,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BANG"#.to_string(),
                    pattern: r#"!"#.to_string(),
                    is_regex: false,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESSTHAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 54,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATERTHAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 55,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 56,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 57,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 58,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 59,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 60,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 61,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 62,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 63,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACE"#.to_string(),
                    pattern: r#"{"#.to_string(),
                    is_regex: false,
                    line_number: 64,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACE"#.to_string(),
                    pattern: r#"}"#.to_string(),
                    is_regex: false,
                    line_number: 65,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[A-Za-z_][A-Za-z0-9_']*"#.to_string(),
                    is_regex: true,
                    line_number: 70,
                    alias: None,
                },
            ],
            keywords: vec![r#"as"#.to_string(), r#"case"#.to_string(), r#"class"#.to_string(), r#"data"#.to_string(), r#"do"#.to_string(), r#"else"#.to_string(), r#"foreign"#.to_string(), r#"if"#.to_string(), r#"import"#.to_string(), r#"in"#.to_string(), r#"infix"#.to_string(), r#"infixl"#.to_string(), r#"infixr"#.to_string(), r#"let"#.to_string(), r#"module"#.to_string(), r#"of"#.to_string(), r#"then"#.to_string(), r#"type"#.to_string(), r#"where"#.to_string()],
            mode: Some(r#"layout"#.to_string()),
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"LINE_COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 22,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BLOCK_COMMENT"#.to_string(),
                    pattern: r#"\{-[\s\S]*?\-\}"#.to_string(),
                    is_regex: true,
                    line_number: 23,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r]+"#.to_string(),
                    is_regex: true,
                    line_number: 24,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![r#"let"#.to_string(), r#"where"#.to_string(), r#"do"#.to_string(), r#"of"#.to_string()],
        }
    }
}

mod v_1_2 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell1.2.tokens
    // Regenerate with: grammar-tools compile-tokens haskell1.2.tokens
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
                    name: r#"FLOAT"#.to_string(),
                    pattern: r#"[0-9]+\.[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 29,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INTEGER"#.to_string(),
                    pattern: r#"[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 30,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHARACTER"#.to_string(),
                    pattern: r#"'(?:[^'\\]|\\.)'"#.to_string(),
                    is_regex: true,
                    line_number: 31,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""(?:[^"\\]|\\.)*""#.to_string(),
                    is_regex: true,
                    line_number: 32,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LAMBDA"#.to_string(),
                    pattern: r#"\\"#.to_string(),
                    is_regex: false,
                    line_number: 37,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RARROW"#.to_string(),
                    pattern: r#"->"#.to_string(),
                    is_regex: false,
                    line_number: 38,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LARROW"#.to_string(),
                    pattern: r#"<-"#.to_string(),
                    is_regex: false,
                    line_number: 39,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 40,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_COLON"#.to_string(),
                    pattern: r#"::"#.to_string(),
                    is_regex: false,
                    line_number: 41,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_DOT"#.to_string(),
                    pattern: r#".."#.to_string(),
                    is_regex: false,
                    line_number: 42,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 43,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQ"#.to_string(),
                    pattern: r#"=="#.to_string(),
                    is_regex: false,
                    line_number: 44,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 45,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 46,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 47,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 48,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 49,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 50,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CARET"#.to_string(),
                    pattern: r#"^"#.to_string(),
                    is_regex: false,
                    line_number: 51,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TILDE"#.to_string(),
                    pattern: r#"~"#.to_string(),
                    is_regex: false,
                    line_number: 52,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BANG"#.to_string(),
                    pattern: r#"!"#.to_string(),
                    is_regex: false,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESSTHAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 54,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATERTHAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 55,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 56,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 57,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 58,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 59,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 60,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 61,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 62,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 63,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACE"#.to_string(),
                    pattern: r#"{"#.to_string(),
                    is_regex: false,
                    line_number: 64,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACE"#.to_string(),
                    pattern: r#"}"#.to_string(),
                    is_regex: false,
                    line_number: 65,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[A-Za-z_][A-Za-z0-9_']*"#.to_string(),
                    is_regex: true,
                    line_number: 70,
                    alias: None,
                },
            ],
            keywords: vec![r#"as"#.to_string(), r#"case"#.to_string(), r#"class"#.to_string(), r#"data"#.to_string(), r#"do"#.to_string(), r#"else"#.to_string(), r#"foreign"#.to_string(), r#"if"#.to_string(), r#"import"#.to_string(), r#"in"#.to_string(), r#"infix"#.to_string(), r#"infixl"#.to_string(), r#"infixr"#.to_string(), r#"let"#.to_string(), r#"module"#.to_string(), r#"of"#.to_string(), r#"then"#.to_string(), r#"type"#.to_string(), r#"where"#.to_string()],
            mode: Some(r#"layout"#.to_string()),
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"LINE_COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 22,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BLOCK_COMMENT"#.to_string(),
                    pattern: r#"\{-[\s\S]*?\-\}"#.to_string(),
                    is_regex: true,
                    line_number: 23,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r]+"#.to_string(),
                    is_regex: true,
                    line_number: 24,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![r#"let"#.to_string(), r#"where"#.to_string(), r#"do"#.to_string(), r#"of"#.to_string()],
        }
    }
}

mod v_1_3 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell1.3.tokens
    // Regenerate with: grammar-tools compile-tokens haskell1.3.tokens
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
                    name: r#"FLOAT"#.to_string(),
                    pattern: r#"[0-9]+\.[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 29,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INTEGER"#.to_string(),
                    pattern: r#"[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 30,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHARACTER"#.to_string(),
                    pattern: r#"'(?:[^'\\]|\\.)'"#.to_string(),
                    is_regex: true,
                    line_number: 31,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""(?:[^"\\]|\\.)*""#.to_string(),
                    is_regex: true,
                    line_number: 32,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LAMBDA"#.to_string(),
                    pattern: r#"\\"#.to_string(),
                    is_regex: false,
                    line_number: 37,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RARROW"#.to_string(),
                    pattern: r#"->"#.to_string(),
                    is_regex: false,
                    line_number: 38,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LARROW"#.to_string(),
                    pattern: r#"<-"#.to_string(),
                    is_regex: false,
                    line_number: 39,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 40,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_COLON"#.to_string(),
                    pattern: r#"::"#.to_string(),
                    is_regex: false,
                    line_number: 41,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_DOT"#.to_string(),
                    pattern: r#".."#.to_string(),
                    is_regex: false,
                    line_number: 42,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 43,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQ"#.to_string(),
                    pattern: r#"=="#.to_string(),
                    is_regex: false,
                    line_number: 44,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 45,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 46,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 47,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 48,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 49,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 50,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CARET"#.to_string(),
                    pattern: r#"^"#.to_string(),
                    is_regex: false,
                    line_number: 51,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TILDE"#.to_string(),
                    pattern: r#"~"#.to_string(),
                    is_regex: false,
                    line_number: 52,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BANG"#.to_string(),
                    pattern: r#"!"#.to_string(),
                    is_regex: false,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESSTHAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 54,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATERTHAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 55,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 56,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 57,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 58,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 59,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 60,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 61,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 62,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 63,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACE"#.to_string(),
                    pattern: r#"{"#.to_string(),
                    is_regex: false,
                    line_number: 64,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACE"#.to_string(),
                    pattern: r#"}"#.to_string(),
                    is_regex: false,
                    line_number: 65,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[A-Za-z_][A-Za-z0-9_']*"#.to_string(),
                    is_regex: true,
                    line_number: 70,
                    alias: None,
                },
            ],
            keywords: vec![r#"as"#.to_string(), r#"case"#.to_string(), r#"class"#.to_string(), r#"data"#.to_string(), r#"do"#.to_string(), r#"else"#.to_string(), r#"foreign"#.to_string(), r#"if"#.to_string(), r#"import"#.to_string(), r#"in"#.to_string(), r#"infix"#.to_string(), r#"infixl"#.to_string(), r#"infixr"#.to_string(), r#"let"#.to_string(), r#"module"#.to_string(), r#"of"#.to_string(), r#"then"#.to_string(), r#"type"#.to_string(), r#"where"#.to_string()],
            mode: Some(r#"layout"#.to_string()),
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"LINE_COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 22,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BLOCK_COMMENT"#.to_string(),
                    pattern: r#"\{-[\s\S]*?\-\}"#.to_string(),
                    is_regex: true,
                    line_number: 23,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r]+"#.to_string(),
                    is_regex: true,
                    line_number: 24,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![r#"let"#.to_string(), r#"where"#.to_string(), r#"do"#.to_string(), r#"of"#.to_string()],
        }
    }
}

mod v_1_4 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell1.4.tokens
    // Regenerate with: grammar-tools compile-tokens haskell1.4.tokens
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
                    name: r#"FLOAT"#.to_string(),
                    pattern: r#"[0-9]+\.[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 29,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INTEGER"#.to_string(),
                    pattern: r#"[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 30,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHARACTER"#.to_string(),
                    pattern: r#"'(?:[^'\\]|\\.)'"#.to_string(),
                    is_regex: true,
                    line_number: 31,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""(?:[^"\\]|\\.)*""#.to_string(),
                    is_regex: true,
                    line_number: 32,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LAMBDA"#.to_string(),
                    pattern: r#"\\"#.to_string(),
                    is_regex: false,
                    line_number: 37,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RARROW"#.to_string(),
                    pattern: r#"->"#.to_string(),
                    is_regex: false,
                    line_number: 38,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LARROW"#.to_string(),
                    pattern: r#"<-"#.to_string(),
                    is_regex: false,
                    line_number: 39,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 40,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_COLON"#.to_string(),
                    pattern: r#"::"#.to_string(),
                    is_regex: false,
                    line_number: 41,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_DOT"#.to_string(),
                    pattern: r#".."#.to_string(),
                    is_regex: false,
                    line_number: 42,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 43,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQ"#.to_string(),
                    pattern: r#"=="#.to_string(),
                    is_regex: false,
                    line_number: 44,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 45,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 46,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 47,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 48,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 49,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 50,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CARET"#.to_string(),
                    pattern: r#"^"#.to_string(),
                    is_regex: false,
                    line_number: 51,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TILDE"#.to_string(),
                    pattern: r#"~"#.to_string(),
                    is_regex: false,
                    line_number: 52,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BANG"#.to_string(),
                    pattern: r#"!"#.to_string(),
                    is_regex: false,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESSTHAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 54,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATERTHAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 55,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 56,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 57,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 58,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 59,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 60,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 61,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 62,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 63,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACE"#.to_string(),
                    pattern: r#"{"#.to_string(),
                    is_regex: false,
                    line_number: 64,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACE"#.to_string(),
                    pattern: r#"}"#.to_string(),
                    is_regex: false,
                    line_number: 65,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[A-Za-z_][A-Za-z0-9_']*"#.to_string(),
                    is_regex: true,
                    line_number: 70,
                    alias: None,
                },
            ],
            keywords: vec![r#"as"#.to_string(), r#"case"#.to_string(), r#"class"#.to_string(), r#"data"#.to_string(), r#"do"#.to_string(), r#"else"#.to_string(), r#"foreign"#.to_string(), r#"if"#.to_string(), r#"import"#.to_string(), r#"in"#.to_string(), r#"infix"#.to_string(), r#"infixl"#.to_string(), r#"infixr"#.to_string(), r#"let"#.to_string(), r#"module"#.to_string(), r#"of"#.to_string(), r#"then"#.to_string(), r#"type"#.to_string(), r#"where"#.to_string()],
            mode: Some(r#"layout"#.to_string()),
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"LINE_COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 22,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BLOCK_COMMENT"#.to_string(),
                    pattern: r#"\{-[\s\S]*?\-\}"#.to_string(),
                    is_regex: true,
                    line_number: 23,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r]+"#.to_string(),
                    is_regex: true,
                    line_number: 24,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![r#"let"#.to_string(), r#"where"#.to_string(), r#"do"#.to_string(), r#"of"#.to_string()],
        }
    }
}

mod v_98 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell98.tokens
    // Regenerate with: grammar-tools compile-tokens haskell98.tokens
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
                    name: r#"FLOAT"#.to_string(),
                    pattern: r#"[0-9]+\.[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 29,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INTEGER"#.to_string(),
                    pattern: r#"[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 30,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHARACTER"#.to_string(),
                    pattern: r#"'(?:[^'\\]|\\.)'"#.to_string(),
                    is_regex: true,
                    line_number: 31,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""(?:[^"\\]|\\.)*""#.to_string(),
                    is_regex: true,
                    line_number: 32,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LAMBDA"#.to_string(),
                    pattern: r#"\\"#.to_string(),
                    is_regex: false,
                    line_number: 37,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RARROW"#.to_string(),
                    pattern: r#"->"#.to_string(),
                    is_regex: false,
                    line_number: 38,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LARROW"#.to_string(),
                    pattern: r#"<-"#.to_string(),
                    is_regex: false,
                    line_number: 39,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 40,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_COLON"#.to_string(),
                    pattern: r#"::"#.to_string(),
                    is_regex: false,
                    line_number: 41,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_DOT"#.to_string(),
                    pattern: r#".."#.to_string(),
                    is_regex: false,
                    line_number: 42,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 43,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQ"#.to_string(),
                    pattern: r#"=="#.to_string(),
                    is_regex: false,
                    line_number: 44,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 45,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 46,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 47,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 48,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 49,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 50,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CARET"#.to_string(),
                    pattern: r#"^"#.to_string(),
                    is_regex: false,
                    line_number: 51,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TILDE"#.to_string(),
                    pattern: r#"~"#.to_string(),
                    is_regex: false,
                    line_number: 52,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BANG"#.to_string(),
                    pattern: r#"!"#.to_string(),
                    is_regex: false,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESSTHAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 54,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATERTHAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 55,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 56,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 57,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 58,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 59,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 60,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 61,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 62,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 63,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACE"#.to_string(),
                    pattern: r#"{"#.to_string(),
                    is_regex: false,
                    line_number: 64,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACE"#.to_string(),
                    pattern: r#"}"#.to_string(),
                    is_regex: false,
                    line_number: 65,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[A-Za-z_][A-Za-z0-9_']*"#.to_string(),
                    is_regex: true,
                    line_number: 70,
                    alias: None,
                },
            ],
            keywords: vec![r#"as"#.to_string(), r#"case"#.to_string(), r#"class"#.to_string(), r#"data"#.to_string(), r#"do"#.to_string(), r#"else"#.to_string(), r#"foreign"#.to_string(), r#"if"#.to_string(), r#"import"#.to_string(), r#"in"#.to_string(), r#"infix"#.to_string(), r#"infixl"#.to_string(), r#"infixr"#.to_string(), r#"let"#.to_string(), r#"module"#.to_string(), r#"of"#.to_string(), r#"then"#.to_string(), r#"type"#.to_string(), r#"where"#.to_string()],
            mode: Some(r#"layout"#.to_string()),
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"LINE_COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 22,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BLOCK_COMMENT"#.to_string(),
                    pattern: r#"\{\-[\s\S]*?\-\}"#.to_string(),
                    is_regex: true,
                    line_number: 23,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r]+"#.to_string(),
                    is_regex: true,
                    line_number: 24,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![r#"let"#.to_string(), r#"where"#.to_string(), r#"do"#.to_string(), r#"of"#.to_string()],
        }
    }
}

mod v_2010 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell2010.tokens
    // Regenerate with: grammar-tools compile-tokens haskell2010.tokens
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
                    name: r#"FLOAT"#.to_string(),
                    pattern: r#"[0-9]+\.[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 29,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INTEGER"#.to_string(),
                    pattern: r#"[0-9]+"#.to_string(),
                    is_regex: true,
                    line_number: 30,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CHARACTER"#.to_string(),
                    pattern: r#"'(?:[^'\\]|\\.)'"#.to_string(),
                    is_regex: true,
                    line_number: 31,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STRING"#.to_string(),
                    pattern: r#""(?:[^"\\]|\\.)*""#.to_string(),
                    is_regex: true,
                    line_number: 32,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LAMBDA"#.to_string(),
                    pattern: r#"\\"#.to_string(),
                    is_regex: false,
                    line_number: 37,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RARROW"#.to_string(),
                    pattern: r#"->"#.to_string(),
                    is_regex: false,
                    line_number: 38,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LARROW"#.to_string(),
                    pattern: r#"<-"#.to_string(),
                    is_regex: false,
                    line_number: 39,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DARROW"#.to_string(),
                    pattern: r#"=>"#.to_string(),
                    is_regex: false,
                    line_number: 40,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_COLON"#.to_string(),
                    pattern: r#"::"#.to_string(),
                    is_regex: false,
                    line_number: 41,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOUBLE_DOT"#.to_string(),
                    pattern: r#".."#.to_string(),
                    is_regex: false,
                    line_number: 42,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQUALS"#.to_string(),
                    pattern: r#"="#.to_string(),
                    is_regex: false,
                    line_number: 43,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"EQ"#.to_string(),
                    pattern: r#"=="#.to_string(),
                    is_regex: false,
                    line_number: 44,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 45,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 46,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 47,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SLASH"#.to_string(),
                    pattern: r#"/"#.to_string(),
                    is_regex: false,
                    line_number: 48,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PIPE"#.to_string(),
                    pattern: r#"|"#.to_string(),
                    is_regex: false,
                    line_number: 49,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"AMPERSAND"#.to_string(),
                    pattern: r#"&"#.to_string(),
                    is_regex: false,
                    line_number: 50,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"CARET"#.to_string(),
                    pattern: r#"^"#.to_string(),
                    is_regex: false,
                    line_number: 51,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"TILDE"#.to_string(),
                    pattern: r#"~"#.to_string(),
                    is_regex: false,
                    line_number: 52,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BANG"#.to_string(),
                    pattern: r#"!"#.to_string(),
                    is_regex: false,
                    line_number: 53,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LESSTHAN"#.to_string(),
                    pattern: r#"<"#.to_string(),
                    is_regex: false,
                    line_number: 54,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"GREATERTHAN"#.to_string(),
                    pattern: r#">"#.to_string(),
                    is_regex: false,
                    line_number: 55,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COLON"#.to_string(),
                    pattern: r#":"#.to_string(),
                    is_regex: false,
                    line_number: 56,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"COMMA"#.to_string(),
                    pattern: r#","#.to_string(),
                    is_regex: false,
                    line_number: 57,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 58,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DOT"#.to_string(),
                    pattern: r#"."#.to_string(),
                    is_regex: false,
                    line_number: 59,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 60,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 61,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACKET"#.to_string(),
                    pattern: r#"["#.to_string(),
                    is_regex: false,
                    line_number: 62,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACKET"#.to_string(),
                    pattern: r#"]"#.to_string(),
                    is_regex: false,
                    line_number: 63,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LBRACE"#.to_string(),
                    pattern: r#"{"#.to_string(),
                    is_regex: false,
                    line_number: 64,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RBRACE"#.to_string(),
                    pattern: r#"}"#.to_string(),
                    is_regex: false,
                    line_number: 65,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[A-Za-z_][A-Za-z0-9_']*"#.to_string(),
                    is_regex: true,
                    line_number: 70,
                    alias: None,
                },
            ],
            keywords: vec![r#"as"#.to_string(), r#"case"#.to_string(), r#"class"#.to_string(), r#"data"#.to_string(), r#"do"#.to_string(), r#"else"#.to_string(), r#"foreign"#.to_string(), r#"if"#.to_string(), r#"import"#.to_string(), r#"in"#.to_string(), r#"infix"#.to_string(), r#"infixl"#.to_string(), r#"infixr"#.to_string(), r#"let"#.to_string(), r#"module"#.to_string(), r#"of"#.to_string(), r#"then"#.to_string(), r#"type"#.to_string(), r#"where"#.to_string()],
            mode: Some(r#"layout"#.to_string()),
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"LINE_COMMENT"#.to_string(),
                    pattern: r#"--[^\n]*"#.to_string(),
                    is_regex: true,
                    line_number: 22,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"BLOCK_COMMENT"#.to_string(),
                    pattern: r#"\{\-[\s\S]*?\-\}"#.to_string(),
                    is_regex: true,
                    line_number: 23,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r]+"#.to_string(),
                    is_regex: true,
                    line_number: 24,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![r#"let"#.to_string(), r#"where"#.to_string(), r#"do"#.to_string(), r#"of"#.to_string()],
        }
    }
}

