// AUTO-GENERATED FILE — DO NOT EDIT
// Source: moslayout.tokens + moslayout.grammar
// Regenerate with: grammar-tools compile-tokens moslayout.tokens
//                  grammar-tools compile-grammar moslayout.grammar
//
// This file embeds both the token grammar and the parser grammar as native
// Rust data structures.  Call `token_grammar()` or `parser_grammar()` instead
// of reading and parsing the .tokens / .grammar files at runtime.

#[allow(unused_imports)]
use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};
#[allow(unused_imports)]
use std::collections::HashMap;

// ===========================================================================
// Token grammar (from moslayout.tokens)
// ===========================================================================
//
// Only three structural keywords: layout, slot, emit.
// Everything else (primitive names, property names, value keywords) is NAME.
// This keeps the grammar simple and avoids keyword conflicts with user-chosen
// slot/emit names.

pub fn token_grammar() -> TokenGrammar {
    TokenGrammar {
        definitions: vec![
            // NUMBER — integer or decimal
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+(\.[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 21,
                alias: None,
            },
            // NAME — PascalCase primitive names AND kebab-case part/slot names
            // AND keyword values like "row", "column", "true", "false".
            // The keyword list below takes priority when text matches exactly.
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z][a-zA-Z0-9]*(-[a-zA-Z][a-zA-Z0-9]*)*"#.to_string(),
                is_regex: true,
                line_number: 42,
                alias: None,
            },
            // Punctuation
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 53,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 54,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 55,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 56,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACKET"#.to_string(),
                pattern: r#"["#.to_string(),
                is_regex: false,
                line_number: 57,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACKET"#.to_string(),
                pattern: r#"]"#.to_string(),
                is_regex: false,
                line_number: 58,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 59,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 60,
                alias: None,
            },
        ],
        // Only three keywords; everything else is NAME and validated semantically.
        keywords: vec![
            r#"layout"#.to_string(),
            r#"slot"#.to_string(),
            r#"emit"#.to_string(),
        ],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"LINE_COMMENT"#.to_string(),
                pattern: r#"\/\/[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 11,
                alias: None,
            },
            TokenDefinition {
                name: r#"BLOCK_COMMENT"#.to_string(),
                pattern: r#"\/\*[\s\S]*?\*\/"#.to_string(),
                is_regex: true,
                line_number: 12,
                alias: None,
            },
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 13,
                alias: None,
            },
        ],
        reserved_keywords: vec![],
        escapes: Some(r#"standard"#.to_string()),
        error_definitions: vec![],
        groups: HashMap::new(),
        case_sensitive: true,
        version: 1,
        case_insensitive: false,
        context_keywords: vec![],
        soft_keywords: vec![],
        layout_keywords: vec![],
    }
}

// ===========================================================================
// Parser grammar (from moslayout.grammar)
// ===========================================================================
//
// file       = layout_def ;
// layout_def = KEYWORD NAME LBRACE { node } RBRACE ;
// node       = NAME [ part_name ] [ LPAREN prop_list RPAREN ]
//                   [ LBRACE { node } RBRACE ] ;
// part_name  = LBRACKET NAME RBRACKET ;
// prop_list  = prop { COMMA prop } ;
// prop       = NAME COLON prop_value   -- named prop
//            | KEYWORD COLON NAME ;    -- shorthand slot/emit binding (slot: label)
// prop_value = KEYWORD COLON NAME      -- slot/emit binding
//            | NAME                    -- keyword value (row, true, false …)
//            | NUMBER ;                -- numeric value

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        // file = layout_def ;
        GrammarRule {
            name: r#"file"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"layout_def"#.to_string() },
            line_number: 1,
        },
        // layout_def = KEYWORD NAME LBRACE { node } RBRACE ;
        GrammarRule {
            name: r#"layout_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },  // "layout"
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },     // ComponentName
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(
                    GrammarElement::RuleReference { name: r#"node"#.to_string() }
                )},
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ]},
            line_number: 5,
        },
        // node = NAME [ part_name ] [ LPAREN prop_list RPAREN ] [ LBRACE { node } RBRACE ] ;
        GrammarRule {
            name: r#"node"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },   // primitive tag
                GrammarElement::Optional { element: Box::new(
                    GrammarElement::RuleReference { name: r#"part_name"#.to_string() }
                )},
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"prop_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ]})},
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(
                        GrammarElement::RuleReference { name: r#"node"#.to_string() }
                    )},
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ]})},
            ]},
            line_number: 9,
        },
        // part_name = LBRACKET NAME RBRACKET ;
        GrammarRule {
            name: r#"part_name"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ]},
            line_number: 14,
        },
        // prop_list = prop { COMMA prop } ;
        GrammarRule {
            name: r#"prop_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"prop"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"prop"#.to_string() },
                ]})},
            ]},
            line_number: 16,
        },
        // prop = NAME COLON prop_value        — named prop: direction: row
        //      | KEYWORD COLON NAME ;          — shorthand: slot: label, emit: onClick
        //
        // The shorthand is sugar for single-slot leaf primitives (Text, Image)
        // where the prop name is implicit.  LL(1) because NAME ≠ KEYWORD.
        GrammarRule {
            name: r#"prop"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                // Named prop: NAME COLON prop_value
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"prop_value"#.to_string() },
                ]},
                // Shorthand slot/emit binding: KEYWORD COLON NAME
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ]},
            ]},
            line_number: 18,
        },
        // prop_value = KEYWORD COLON NAME | NAME | NUMBER ;
        // LL(1): KEYWORD → slot/emit binding; NAME → keyword value; NUMBER → numeric.
        GrammarRule {
            name: r#"prop_value"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ]},
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
            ]},
            line_number: 20,
        },
        ],
        version: 1,
    }
}
