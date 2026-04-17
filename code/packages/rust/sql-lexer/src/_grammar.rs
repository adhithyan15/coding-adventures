// AUTO-GENERATED FILE — DO NOT EDIT
// Source: sql.tokens
// Regenerate with: grammar-tools compile-tokens sql.tokens
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
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z_][a-zA-Z0-9_]*"#.to_string(),
                is_regex: true,
                line_number: 17,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+\.?[0-9]*"#.to_string(),
                is_regex: true,
                line_number: 18,
                alias: None,
            },
            TokenDefinition {
                name: r#"STRING_SQ"#.to_string(),
                pattern: r#"'([^'\\]|\\.)*'"#.to_string(),
                is_regex: true,
                line_number: 19,
                alias: Some(r#"STRING"#.to_string()),
            },
            TokenDefinition {
                name: r#"QUOTED_ID"#.to_string(),
                pattern: r#"`[^`]+`"#.to_string(),
                is_regex: true,
                line_number: 20,
                alias: Some(r#"NAME"#.to_string()),
            },
            TokenDefinition {
                name: r#"LESS_EQUALS"#.to_string(),
                pattern: r#"<="#.to_string(),
                is_regex: false,
                line_number: 22,
                alias: None,
            },
            TokenDefinition {
                name: r#"GREATER_EQUALS"#.to_string(),
                pattern: r#">="#.to_string(),
                is_regex: false,
                line_number: 23,
                alias: None,
            },
            TokenDefinition {
                name: r#"NOT_EQUALS"#.to_string(),
                pattern: r#"!="#.to_string(),
                is_regex: false,
                line_number: 24,
                alias: None,
            },
            TokenDefinition {
                name: r#"NEQ_ANSI"#.to_string(),
                pattern: r#"<>"#.to_string(),
                is_regex: false,
                line_number: 25,
                alias: Some(r#"NOT_EQUALS"#.to_string()),
            },
            TokenDefinition {
                name: r#"EQUALS"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 27,
                alias: None,
            },
            TokenDefinition {
                name: r#"LESS_THAN"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 28,
                alias: None,
            },
            TokenDefinition {
                name: r#"GREATER_THAN"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 29,
                alias: None,
            },
            TokenDefinition {
                name: r#"PLUS"#.to_string(),
                pattern: r#"+"#.to_string(),
                is_regex: false,
                line_number: 30,
                alias: None,
            },
            TokenDefinition {
                name: r#"MINUS"#.to_string(),
                pattern: r#"-"#.to_string(),
                is_regex: false,
                line_number: 31,
                alias: None,
            },
            TokenDefinition {
                name: r#"STAR"#.to_string(),
                pattern: r#"*"#.to_string(),
                is_regex: false,
                line_number: 32,
                alias: None,
            },
            TokenDefinition {
                name: r#"SLASH"#.to_string(),
                pattern: r#"/"#.to_string(),
                is_regex: false,
                line_number: 33,
                alias: None,
            },
            TokenDefinition {
                name: r#"PERCENT"#.to_string(),
                pattern: r#"%"#.to_string(),
                is_regex: false,
                line_number: 34,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 36,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 37,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 38,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMICOLON"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 39,
                alias: None,
            },
            TokenDefinition {
                name: r#"DOT"#.to_string(),
                pattern: r#"."#.to_string(),
                is_regex: false,
                line_number: 40,
                alias: None,
            },
        ],
        keywords: vec![r#"SELECT"#.to_string(), r#"FROM"#.to_string(), r#"WHERE"#.to_string(), r#"GROUP"#.to_string(), r#"BY"#.to_string(), r#"HAVING"#.to_string(), r#"ORDER"#.to_string(), r#"LIMIT"#.to_string(), r#"OFFSET"#.to_string(), r#"INSERT"#.to_string(), r#"INTO"#.to_string(), r#"VALUES"#.to_string(), r#"UPDATE"#.to_string(), r#"SET"#.to_string(), r#"DELETE"#.to_string(), r#"CREATE"#.to_string(), r#"DROP"#.to_string(), r#"TABLE"#.to_string(), r#"IF"#.to_string(), r#"EXISTS"#.to_string(), r#"NOT"#.to_string(), r#"AND"#.to_string(), r#"OR"#.to_string(), r#"NULL"#.to_string(), r#"IS"#.to_string(), r#"IN"#.to_string(), r#"BETWEEN"#.to_string(), r#"LIKE"#.to_string(), r#"AS"#.to_string(), r#"DISTINCT"#.to_string(), r#"ALL"#.to_string(), r#"UNION"#.to_string(), r#"INTERSECT"#.to_string(), r#"EXCEPT"#.to_string(), r#"JOIN"#.to_string(), r#"INNER"#.to_string(), r#"LEFT"#.to_string(), r#"RIGHT"#.to_string(), r#"OUTER"#.to_string(), r#"CROSS"#.to_string(), r#"FULL"#.to_string(), r#"ON"#.to_string(), r#"ASC"#.to_string(), r#"DESC"#.to_string(), r#"TRUE"#.to_string(), r#"FALSE"#.to_string(), r#"CASE"#.to_string(), r#"WHEN"#.to_string(), r#"THEN"#.to_string(), r#"ELSE"#.to_string(), r#"END"#.to_string(), r#"PRIMARY"#.to_string(), r#"KEY"#.to_string(), r#"UNIQUE"#.to_string(), r#"DEFAULT"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 100,
                alias: None,
            },
            TokenDefinition {
                name: r#"LINE_COMMENT"#.to_string(),
                pattern: r#"--[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 101,
                alias: None,
            },
            TokenDefinition {
                name: r#"BLOCK_COMMENT"#.to_string(),
                pattern: r#"\x2f\*([^*]|\*[^\x2f])*\*\x2f"#.to_string(),
                is_regex: true,
                line_number: 102,
                alias: None,
            },
        ],
        reserved_keywords: vec![],
        escapes: None,
        error_definitions: vec![],
        groups: HashMap::new(),
        case_sensitive: false,
        version: 1,
        case_insensitive: true,
        context_keywords: vec![],
        soft_keywords: vec![],
        layout_keywords: vec![],
    }
}
