// AUTO-GENERATED FILE — DO NOT EDIT
// Source: dartmouth_basic.tokens
// Regenerate with: grammar-tools compile-tokens dartmouth_basic.tokens
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
                name: r#"LE"#.to_string(),
                pattern: r#"<="#.to_string(),
                is_regex: false,
                line_number: 50,
                alias: None,
            },
            TokenDefinition {
                name: r#"GE"#.to_string(),
                pattern: r#">="#.to_string(),
                is_regex: false,
                line_number: 51,
                alias: None,
            },
            TokenDefinition {
                name: r#"NE"#.to_string(),
                pattern: r#"<>"#.to_string(),
                is_regex: false,
                line_number: 52,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]*\.?[0-9]+([Ee][+-]?[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 85,
                alias: None,
            },
            TokenDefinition {
                name: r#"LINE_NUM"#.to_string(),
                pattern: r#"[0-9]+"#.to_string(),
                is_regex: true,
                line_number: 86,
                alias: None,
            },
            TokenDefinition {
                name: r#"STRING_BODY"#.to_string(),
                pattern: r#""[^"]*""#.to_string(),
                is_regex: true,
                line_number: 112,
                alias: Some(r#"STRING"#.to_string()),
            },
            TokenDefinition {
                name: r#"BUILTIN_FN"#.to_string(),
                pattern: r#"(?:sin|cos|tan|atn|exp|log|abs|sqr|int|rnd|sgn)"#.to_string(),
                is_regex: true,
                line_number: 168,
                alias: None,
            },
            TokenDefinition {
                name: r#"USER_FN"#.to_string(),
                pattern: r#"fn[a-z]"#.to_string(),
                is_regex: true,
                line_number: 169,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-z][a-z0-9]*"#.to_string(),
                is_regex: true,
                line_number: 202,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 242,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 243,
                alias: None,
            },
            TokenDefinition {
                name: r#"STAR"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 244,
                alias: None,
            },
            TokenDefinition {
                name: r#"SLASH"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 245,
                alias: None,
            },
            TokenDefinition {
                name: r#"CARET"#.to_string(),
                pattern: r#"^"#.to_string(),
                is_regex: false,
                line_number: 246,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQ"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 247,
                alias: None,
            },
            TokenDefinition {
                name: r#"LT"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 248,
                alias: None,
            },
            TokenDefinition {
                name: r#"GT"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 249,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 250,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 251,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 252,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMICOLON"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 253,
                alias: None,
            },
            TokenDefinition {
                name: r#"NEWLINE"#.to_string(),
                pattern: r#"\r?\n"#.to_string(),
                is_regex: true,
                line_number: 274,
                alias: None,
            },
        ],
        keywords: vec![r#"LET"#.to_string(), r#"PRINT"#.to_string(), r#"INPUT"#.to_string(), r#"IF"#.to_string(), r#"THEN"#.to_string(), r#"GOTO"#.to_string(), r#"GOSUB"#.to_string(), r#"RETURN"#.to_string(), r#"FOR"#.to_string(), r#"TO"#.to_string(), r#"STEP"#.to_string(), r#"NEXT"#.to_string(), r#"END"#.to_string(), r#"STOP"#.to_string(), r#"REM"#.to_string(), r#"READ"#.to_string(), r#"DATA"#.to_string(), r#"RESTORE"#.to_string(), r#"DIM"#.to_string(), r#"DEF"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t]+"#.to_string(),
                is_regex: true,
                line_number: 286,
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
                line_number: 302,
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
    }
}
