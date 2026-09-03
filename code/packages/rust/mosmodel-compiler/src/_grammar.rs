// AUTO-GENERATED FILE — DO NOT EDIT
// Source: mosmodel.tokens + mosmodel.grammar
// Regenerate with: grammar-tools compile-combined mosmodel.tokens mosmodel.grammar
//
// This file embeds both the token grammar and the parser grammar as native
// Rust data structures.  Call `token_grammar()` or `parser_grammar()` instead
// of reading and parsing the .tokens / .grammar files at runtime.

#[allow(unused_imports)]
use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
#[allow(unused_imports)]
use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};
#[allow(unused_imports)]
use std::collections::HashMap;

// ===========================================================================
// Token grammar (from mosmodel.tokens)
// ===========================================================================

pub fn token_grammar() -> TokenGrammar {
    TokenGrammar {
        definitions: vec![
            TokenDefinition {
                name: r#"STRING"#.to_string(),
                pattern: r#""([^"\\\n]|\\.)*""#.to_string(),
                is_regex: true,
                line_number: 23,
                alias: None,
            },
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+(\.[0-9]*)?"#.to_string(),
                is_regex: true,
                line_number: 24,
                alias: None,
            },
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z][a-zA-Z0-9]*(-[a-zA-Z][a-zA-Z0-9]*)*"#.to_string(),
                is_regex: true,
                line_number: 52,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 58,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
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
                name: r#"LANGLE"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 62,
                alias: None,
            },
            TokenDefinition {
                name: r#"RANGLE"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 63,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 64,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMICOLON"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 65,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 66,
                alias: None,
            },
            TokenDefinition {
                name: r#"EQUALS"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 67,
                alias: None,
            },
        ],
        keywords: vec![r#"component"#.to_string(), r#"slot"#.to_string(), r#"emit"#.to_string(), r#"list"#.to_string(), r#"text"#.to_string(), r#"number"#.to_string(), r#"bool"#.to_string(), r#"image"#.to_string(), r#"color"#.to_string(), r#"node"#.to_string(), r#"true"#.to_string(), r#"false"#.to_string()],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"LINE_COMMENT"#.to_string(),
                pattern: r#"\/\/[^\n]*"#.to_string(),
                is_regex: true,
                line_number: 15,
                alias: None,
            },
            TokenDefinition {
                name: r#"BLOCK_COMMENT"#.to_string(),
                pattern: r#"\/\*[\s\S]*?\*\/"#.to_string(),
                is_regex: true,
                line_number: 16,
                alias: None,
            },
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 17,
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
        start_mode: None,
        transitions: vec![],
    }
}

// ===========================================================================
// Parser grammar (from mosmodel.grammar)
// ===========================================================================

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"file"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"component_def"#.to_string() },
            line_number: 23,
        },
        GrammarRule {
            name: r#"component_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"member"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 38,
        },
        GrammarRule {
            name: r#"member"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"slot_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"emit_decl"#.to_string() },
            ] },
            line_number: 40,
        },
        GrammarRule {
            name: r#"slot_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"slot_type"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"slot_default"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 54,
        },
        GrammarRule {
            name: r#"slot_type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"list_type"#.to_string() },
                GrammarElement::RuleReference { name: r#"scalar_type"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 63,
        },
        GrammarRule {
            name: r#"scalar_type"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
            line_number: 68,
        },
        GrammarRule {
            name: r#"list_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"LANGLE"#.to_string() },
                GrammarElement::RuleReference { name: r#"inner_type"#.to_string() },
                GrammarElement::TokenReference { name: r#"RANGLE"#.to_string() },
            ] },
            line_number: 74,
        },
        GrammarRule {
            name: r#"inner_type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"list_type"#.to_string() },
                GrammarElement::RuleReference { name: r#"scalar_type"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 76,
        },
        GrammarRule {
            name: r#"slot_default"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
            ] },
            line_number: 79,
        },
        GrammarRule {
            name: r#"emit_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"emit_param_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 91,
        },
        GrammarRule {
            name: r#"emit_param_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"emit_param"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"emit_param"#.to_string() },
                    ] }) },
            ] },
            line_number: 93,
        },
        GrammarRule {
            name: r#"emit_param"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"emit_payload_type"#.to_string() },
            ] },
            line_number: 96,
        },
        GrammarRule {
            name: r#"emit_payload_type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 99,
        },
    ],
        version: 1,
    }
}
