// AUTO-GENERATED FILE — DO NOT EDIT
// Source: flow_matic.tokens
// Regenerate with: grammar-tools compile-tokens flow_matic.tokens
//
// This file embeds a TokenGrammar as native Rust data structures.
// Call `token_grammar()` instead of reading and parsing the .tokens file.

#[allow(unused_imports)]
use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
#[allow(unused_imports)]
use std::collections::HashMap;

pub fn token_grammar() -> TokenGrammar {
    TokenGrammar {
        definitions: vec![
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+"#.to_string(),
                is_regex: true,
                line_number: 83,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-z][a-z0-9]*(-[a-z0-9]+)*"#.to_string(),
                is_regex: true,
                line_number: 101,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 176,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 177,
                alias: None,
            },
            TokenDefinition {
                name: r#"PERIOD"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: false,
                line_number: 178,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMICOLON"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 179,
                alias: None,
            },
        ],
        keywords: vec![r#"INPUT"#.to_string(), r#"OUTPUT"#.to_string(), r#"HSP"#.to_string(), r#"COMPARE"#.to_string(), r#"WITH"#.to_string(), r#"IF"#.to_string(), r#"GREATER"#.to_string(), r#"EQUAL"#.to_string(), r#"LESS"#.to_string(), r#"GO"#.to_string(), r#"TO"#.to_string(), r#"OPERATION"#.to_string(), r#"OTHERWISE"#.to_string(), r#"JUMP"#.to_string(), r#"TRANSFER"#.to_string(), r#"MOVE"#.to_string(), r#"WRITE-ITEM"#.to_string(), r#"READ-ITEM"#.to_string(), r#"END"#.to_string(), r#"OF"#.to_string(), r#"DATA"#.to_string(), r#"TEST"#.to_string(), r#"AGAINST"#.to_string(), r#"REWIND"#.to_string(), r#"CLOSE-OUT"#.to_string(), r#"FILES"#.to_string(), r#"STOP"#.to_string(), r#"SET"#.to_string(), r#"ADD"#.to_string(), r#"SUBTRACT"#.to_string(), r#"MULTIPLY"#.to_string(), r#"DIVIDE"#.to_string(), r#"BY"#.to_string(), r#"FROM"#.to_string(), r#"INTO"#.to_string(), r#"EXECUTE"#.to_string(), r#"DEFINE"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 189,
                alias: None,
            },
        ],
        reserved_keywords: vec![],
        escapes: None,
        error_definitions: vec![
            TokenDefinition {
                name: r#"UNKNOWN"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: true,
                line_number: 200,
                alias: None,
            },
        ],
        groups: HashMap::new(),
        case_sensitive: false,
        version: 1,
        case_insensitive: true,
        context_keywords: vec![],
        soft_keywords: vec![],
        layout_keywords: vec![],
        start_mode: None,
        transitions: vec![],
    }
}
