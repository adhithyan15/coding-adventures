// AUTO-GENERATED FILE — DO NOT EDIT
// Source: mosmodel.tokens + mosmodel.grammar
// Regenerate with: grammar-tools compile-tokens mosmodel.tokens
//                  grammar-tools compile-grammar mosmodel.grammar
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
// Token grammar (from mosmodel.tokens)
// ===========================================================================

pub fn token_grammar() -> TokenGrammar {
    TokenGrammar {
        definitions: vec![
            // STRING — double-quoted string literal
            TokenDefinition {
                name: r#"STRING"#.to_string(),
                pattern: r#""([^"\\\n]|\\.)*""#.to_string(),
                is_regex: true,
                line_number: 20,
                alias: None,
            },
            // NUMBER — integer or decimal (e.g. 42, 3.14, 0)
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+(\.[0-9]*)?"#.to_string(),
                is_regex: true,
                line_number: 21,
                alias: None,
            },
            // NAME — PascalCase component names AND kebab-case slot/emit names.
            // Must come AFTER keywords are registered so the keyword matcher
            // takes priority when the text matches exactly.
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z][a-zA-Z0-9]*(-[a-zA-Z][a-zA-Z0-9]*)*"#.to_string(),
                is_regex: true,
                line_number: 46,
                alias: None,
            },
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 52,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 53,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 54,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 55,
                alias: None,
            },
            TokenDefinition {
                name: r#"LANGLE"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 56,
                alias: None,
            },
            TokenDefinition {
                name: r#"RANGLE"#.to_string(),
                pattern: r#">"#.to_string(),
                is_regex: false,
                line_number: 57,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 58,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMICOLON"#.to_string(),
                pattern: r#";"#.to_string(),
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
            TokenDefinition {
                name: r#"EQUALS"#.to_string(),
                pattern: r#"="#.to_string(),
                is_regex: false,
                line_number: 61,
                alias: None,
            },
        ],
        // Keywords take priority over NAME when matched exactly.
        keywords: vec![
            r#"component"#.to_string(),
            r#"slot"#.to_string(),
            r#"emit"#.to_string(),
            r#"list"#.to_string(),
            r#"text"#.to_string(),
            r#"number"#.to_string(),
            r#"bool"#.to_string(),
            r#"image"#.to_string(),
            r#"color"#.to_string(),
            r#"node"#.to_string(),
            r#"true"#.to_string(),
            r#"false"#.to_string(),
        ],
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
    }
}

// ===========================================================================
// Parser grammar (from mosmodel.grammar)
// ===========================================================================
//
// mosmodel grammar (reproduced for reference):
//
//   file            = component_def ;
//   component_def   = KEYWORD NAME LBRACE { member } RBRACE ;
//   member          = slot_decl | emit_decl ;
//   slot_decl       = KEYWORD NAME COLON slot_type [ EQUALS slot_default ] SEMICOLON ;
//   slot_type       = scalar_type | list_type | NAME ;
//   scalar_type     = KEYWORD ;
//   list_type       = KEYWORD LANGLE inner_type RANGLE ;
//   inner_type      = scalar_type | NAME ;
//   slot_default    = STRING | NUMBER | KEYWORD ;
//   emit_decl       = KEYWORD NAME [ LPAREN emit_param_list RPAREN ] SEMICOLON ;
//   emit_param_list = emit_param { COMMA emit_param } ;
//   emit_param      = NAME COLON emit_payload_type ;
//   emit_payload_type = KEYWORD | NAME ;

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        // file = component_def ;
        GrammarRule {
            name: r#"file"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"component_def"#.to_string() },
            line_number: 16,
        },
        // component_def = KEYWORD NAME LBRACE { member } RBRACE ;
        GrammarRule {
            name: r#"component_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },  // "component"
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },     // PascalCase name
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(
                    GrammarElement::RuleReference { name: r#"member"#.to_string() }
                )},
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ]},
            line_number: 28,
        },
        // member = slot_decl | emit_decl ;
        GrammarRule {
            name: r#"member"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"slot_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"emit_decl"#.to_string() },
            ]},
            line_number: 32,
        },
        // slot_decl = KEYWORD NAME COLON slot_type [ EQUALS slot_default ] SEMICOLON ;
        GrammarRule {
            name: r#"slot_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },  // "slot"
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },     // kebab-case name
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"slot_type"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"slot_default"#.to_string() },
                ]})},
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ]},
            line_number: 43,
        },
        // slot_type = scalar_type | list_type | NAME ;
        GrammarRule {
            name: r#"slot_type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"list_type"#.to_string() },   // list<T> first (more specific)
                GrammarElement::RuleReference { name: r#"scalar_type"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },        // named component type
            ]},
            line_number: 52,
        },
        // scalar_type = KEYWORD ;
        GrammarRule {
            name: r#"scalar_type"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
            line_number: 57,
        },
        // list_type = KEYWORD LANGLE inner_type RANGLE ;
        GrammarRule {
            name: r#"list_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },  // "list"
                GrammarElement::TokenReference { name: r#"LANGLE"#.to_string() },
                GrammarElement::RuleReference { name: r#"inner_type"#.to_string() },
                GrammarElement::TokenReference { name: r#"RANGLE"#.to_string() },
            ]},
            line_number: 60,
        },
        // inner_type = scalar_type | NAME ;
        GrammarRule {
            name: r#"inner_type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"scalar_type"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ]},
            line_number: 65,
        },
        // slot_default = STRING | NUMBER | KEYWORD ;
        GrammarRule {
            name: r#"slot_default"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },  // true | false
            ]},
            line_number: 68,
        },
        // emit_decl = KEYWORD NAME [ LPAREN emit_param_list RPAREN ] SEMICOLON ;
        GrammarRule {
            name: r#"emit_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },  // "emit"
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },     // camelCase name
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"emit_param_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ]})},
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ]},
            line_number: 78,
        },
        // emit_param_list = emit_param { COMMA emit_param } ;
        GrammarRule {
            name: r#"emit_param_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"emit_param"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"emit_param"#.to_string() },
                ]})},
            ]},
            line_number: 85,
        },
        // emit_param = NAME COLON emit_payload_type ;
        GrammarRule {
            name: r#"emit_param"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"emit_payload_type"#.to_string() },
            ]},
            line_number: 89,
        },
        // emit_payload_type = KEYWORD | NAME ;
        GrammarRule {
            name: r#"emit_payload_type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ]},
            line_number: 93,
        },
        ],
        version: 1,
    }
}
