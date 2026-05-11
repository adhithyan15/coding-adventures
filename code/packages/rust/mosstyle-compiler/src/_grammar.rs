// AUTO-GENERATED FILE — DO NOT EDIT
// Source: mosstyle.tokens + mosstyle.grammar
// Regenerate with: grammar-tools compile-tokens mosstyle.tokens
//                  grammar-tools compile-grammar mosstyle.grammar

#[allow(unused_imports)]
use grammar_tools::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};
use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};
#[allow(unused_imports)]
use std::collections::HashMap;

// ===========================================================================
// Token grammar (from mosstyle.tokens)
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
            // DIMENSION — number with unit suffix (4px, 1.5rem, 80ms)
            // Must precede NUMBER so "4px" is one token, not "4" + "px".
            TokenDefinition {
                name: r#"DIMENSION"#.to_string(),
                pattern: r#"[0-9]+(\.[0-9]+)?(px|rem|em|pt|ms|s|deg|%)"#.to_string(),
                is_regex: true,
                line_number: 23,
                alias: None,
            },
            // NUMBER — unitless numeric value
            TokenDefinition {
                name: r#"NUMBER"#.to_string(),
                pattern: r#"[0-9]+(\.[0-9]+)?"#.to_string(),
                is_regex: true,
                line_number: 26,
                alias: None,
            },
            // HASH_COLOR — hex color literal (#rgb, #rrggbb, #rrggbbaa)
            TokenDefinition {
                name: r#"HASH_COLOR"#.to_string(),
                pattern: r#"#[0-9a-fA-F]{3,8}"#.to_string(),
                is_regex: true,
                line_number: 29,
                alias: None,
            },
            // TOKEN_REF — Lattice design-token reference ($color-surface)
            TokenDefinition {
                name: r#"TOKEN_REF"#.to_string(),
                pattern: r#"\$[a-z][a-z0-9]*(-[a-z][a-z0-9]*)*"#.to_string(),
                is_regex: true,
                line_number: 32,
                alias: None,
            },
            // NAME — identifiers (component names, part names, keyword values)
            TokenDefinition {
                name: r#"NAME"#.to_string(),
                pattern: r#"[a-zA-Z][a-zA-Z0-9]*(-[a-zA-Z][a-zA-Z0-9]*)*"#.to_string(),
                is_regex: true,
                line_number: 44,
                alias: None,
            },
            // Punctuation
            TokenDefinition {
                name: r#"LBRACE"#.to_string(),
                pattern: r#"{"#.to_string(),
                is_regex: false,
                line_number: 50,
                alias: None,
            },
            TokenDefinition {
                name: r#"RBRACE"#.to_string(),
                pattern: r#"}"#.to_string(),
                is_regex: false,
                line_number: 51,
                alias: None,
            },
            TokenDefinition {
                name: r#"LPAREN"#.to_string(),
                pattern: r#"("#.to_string(),
                is_regex: false,
                line_number: 52,
                alias: None,
            },
            TokenDefinition {
                name: r#"RPAREN"#.to_string(),
                pattern: r#")"#.to_string(),
                is_regex: false,
                line_number: 53,
                alias: None,
            },
            TokenDefinition {
                name: r#"SEMICOLON"#.to_string(),
                pattern: r#";"#.to_string(),
                is_regex: false,
                line_number: 54,
                alias: None,
            },
            TokenDefinition {
                name: r#"COLON"#.to_string(),
                pattern: r#":"#.to_string(),
                is_regex: false,
                line_number: 55,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMA"#.to_string(),
                pattern: r#","#.to_string(),
                is_regex: false,
                line_number: 56,
                alias: None,
            },
        ],
        // Three structural keywords.
        keywords: vec![
            r#"style"#.to_string(),
            r#"part"#.to_string(),
            r#"state"#.to_string(),
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
// Parser grammar (from mosstyle.grammar)
// ===========================================================================
//
// file          = style_def ;
// style_def     = KEYWORD NAME LBRACE { part_def } RBRACE ;
// part_def      = KEYWORD NAME LBRACE { part_item } RBRACE ;
// part_item     = state_block | property_decl ;
// state_block   = KEYWORD NAME LBRACE { property_decl } RBRACE ;
// property_decl = NAME COLON style_value SEMICOLON ;
// style_value   = TOKEN_REF | HASH_COLOR | DIMENSION | NUMBER | STRING | NAME ;
//
// LL(1) disambiguation for part_item:
//   - KEYWORD ("state") → state_block
//   - NAME              → property_decl

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        // file = style_def ;
        GrammarRule {
            name: r#"file"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"style_def"#.to_string() },
            line_number: 1,
        },
        // style_def = KEYWORD NAME LBRACE { part_def } RBRACE ;
        GrammarRule {
            name: r#"style_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },  // "style"
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },     // ComponentName
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(
                    GrammarElement::RuleReference { name: r#"part_def"#.to_string() }
                )},
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ]},
            line_number: 4,
        },
        // part_def = KEYWORD NAME LBRACE { part_item } RBRACE ;
        GrammarRule {
            name: r#"part_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },  // "part"
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },     // part-name
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(
                    GrammarElement::RuleReference { name: r#"part_item"#.to_string() }
                )},
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ]},
            line_number: 7,
        },
        // part_item = state_block | property_decl ;
        // FIRST(state_block) = {KEYWORD}; FIRST(property_decl) = {NAME} → LL(1).
        GrammarRule {
            name: r#"part_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"state_block"#.to_string() },
                GrammarElement::RuleReference { name: r#"property_decl"#.to_string() },
            ]},
            line_number: 10,
        },
        // state_block = KEYWORD NAME LBRACE { property_decl } RBRACE ;
        GrammarRule {
            name: r#"state_block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },  // "state"
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },     // state-name
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(
                    GrammarElement::RuleReference { name: r#"property_decl"#.to_string() }
                )},
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ]},
            line_number: 13,
        },
        // property_decl = NAME COLON style_value SEMICOLON ;
        GrammarRule {
            name: r#"property_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"style_value"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ]},
            line_number: 17,
        },
        // style_value = TOKEN_REF | HASH_COLOR | DIMENSION | NUMBER | STRING | NAME ;
        // Listed most-specific first for LL(1) correctness.
        GrammarRule {
            name: r#"style_value"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"TOKEN_REF"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH_COLOR"#.to_string() },
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ]},
            line_number: 20,
        },
        ],
        version: 1,
    }
}
