// AUTO-GENERATED FILE — DO NOT EDIT
// Source: xml.tokens
// Regenerate with: grammar-tools compile-tokens xml.tokens
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
                name: r#"TEXT"#.to_string(),
                pattern: r#"[^<&]+"#.to_string(),
                is_regex: true,
                line_number: 77,
                alias: None,
            },
            TokenDefinition {
                name: r#"ENTITY_REF"#.to_string(),
                pattern: r#"&[a-zA-Z][a-zA-Z0-9]*;"#.to_string(),
                is_regex: true,
                line_number: 78,
                alias: None,
            },
            TokenDefinition {
                name: r#"CHAR_REF_HEX"#.to_string(),
                pattern: r#"&#x[0-9a-fA-F]+;"#.to_string(),
                is_regex: true,
                line_number: 85,
                alias: Some(r#"CHAR_REF"#.to_string()),
            },
            TokenDefinition {
                name: r#"CHAR_REF_DEC"#.to_string(),
                pattern: r#"&#[0-9]+;"#.to_string(),
                is_regex: true,
                line_number: 86,
                alias: Some(r#"CHAR_REF"#.to_string()),
            },
            TokenDefinition {
                name: r#"COMMENT_START"#.to_string(),
                pattern: r#"<!--"#.to_string(),
                is_regex: false,
                line_number: 88,
                alias: None,
            },
            TokenDefinition {
                name: r#"CDATA_START"#.to_string(),
                pattern: r#"<![CDATA["#.to_string(),
                is_regex: false,
                line_number: 89,
                alias: None,
            },
            TokenDefinition {
                name: r#"PI_START"#.to_string(),
                pattern: r#"<?"#.to_string(),
                is_regex: false,
                line_number: 90,
                alias: None,
            },
            TokenDefinition {
                name: r#"CLOSE_TAG_START"#.to_string(),
                pattern: r#"</"#.to_string(),
                is_regex: false,
                line_number: 91,
                alias: None,
            },
            TokenDefinition {
                name: r#"OPEN_TAG_START"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 92,
                alias: None,
            },
        ],
        keywords: vec![],
        mode: None,
        skip_definitions: vec![
            TokenDefinition {
                name: r#"WHITESPACE"#.to_string(),
                pattern: r#"[ \t\r\n]+"#.to_string(),
                is_regex: true,
                line_number: 62,
                alias: None,
            },
        ],
        reserved_keywords: vec![],
        escapes: Some(r#"none"#.to_string()),
        error_definitions: vec![],
        groups: {
            let mut __map: HashMap<String, PatternGroup> = HashMap::new();
            let mut __g_cdata = PatternGroup { name: r#"cdata"#.to_string(), definitions: vec![
                    TokenDefinition {
                        name: r#"CDATA_END"#.to_string(),
                        pattern: r#"]]>"#.to_string(),
                        is_regex: false,
                        line_number: 150,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"CDATA_TEXT"#.to_string(),
                        pattern: r#"[^\]]+"#.to_string(),
                        is_regex: true,
                        line_number: 151,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"CDATA_BRACK"#.to_string(),
                        pattern: r#"]"#.to_string(),
                        is_regex: true,
                        line_number: 152,
                        alias: Some(r#"CDATA_TEXT"#.to_string()),
                    },
                ] };
            __map.insert(r#"cdata"#.to_string(), __g_cdata);
            let mut __g_comment = PatternGroup { name: r#"comment"#.to_string(), definitions: vec![
                    TokenDefinition {
                        name: r#"COMMENT_END"#.to_string(),
                        pattern: r#"-->"#.to_string(),
                        is_regex: false,
                        line_number: 133,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"COMMENT_TEXT"#.to_string(),
                        pattern: r#"[^-]+"#.to_string(),
                        is_regex: true,
                        line_number: 134,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"COMMENT_DASH"#.to_string(),
                        pattern: r#"-"#.to_string(),
                        is_regex: true,
                        line_number: 135,
                        alias: Some(r#"COMMENT_TEXT"#.to_string()),
                    },
                ] };
            __map.insert(r#"comment"#.to_string(), __g_comment);
            let mut __g_pi = PatternGroup { name: r#"pi"#.to_string(), definitions: vec![
                    TokenDefinition {
                        name: r#"PI_END"#.to_string(),
                        pattern: r#"?>"#.to_string(),
                        is_regex: false,
                        line_number: 184,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"PI_TARGET"#.to_string(),
                        pattern: r#"[a-zA-Z_][a-zA-Z0-9_:.-]*"#.to_string(),
                        is_regex: true,
                        line_number: 185,
                        alias: None,
                    },
                ] };
            __map.insert(r#"pi"#.to_string(), __g_pi);
            let mut __g_pi_body = PatternGroup { name: r#"pi_body"#.to_string(), definitions: vec![
                    TokenDefinition {
                        name: r#"PI_END"#.to_string(),
                        pattern: r#"?>"#.to_string(),
                        is_regex: false,
                        line_number: 188,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"PI_TEXT"#.to_string(),
                        pattern: r#"[^?]+"#.to_string(),
                        is_regex: true,
                        line_number: 189,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"PI_QMARK"#.to_string(),
                        pattern: r#"\?"#.to_string(),
                        is_regex: true,
                        line_number: 190,
                        alias: Some(r#"PI_TEXT"#.to_string()),
                    },
                ] };
            __map.insert(r#"pi_body"#.to_string(), __g_pi_body);
            let mut __g_tag = PatternGroup { name: r#"tag"#.to_string(), definitions: vec![
                    TokenDefinition {
                        name: r#"TAG_NAME"#.to_string(),
                        pattern: r#"[a-zA-Z_][a-zA-Z0-9_:.-]*"#.to_string(),
                        is_regex: true,
                        line_number: 107,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"ATTR_EQUALS"#.to_string(),
                        pattern: r#"="#.to_string(),
                        is_regex: false,
                        line_number: 108,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"ATTR_VALUE_DQ"#.to_string(),
                        pattern: r#""[^"]*""#.to_string(),
                        is_regex: true,
                        line_number: 109,
                        alias: Some(r#"ATTR_VALUE"#.to_string()),
                    },
                    TokenDefinition {
                        name: r#"ATTR_VALUE_SQ"#.to_string(),
                        pattern: r#"'[^']*'"#.to_string(),
                        is_regex: true,
                        line_number: 110,
                        alias: Some(r#"ATTR_VALUE"#.to_string()),
                    },
                    TokenDefinition {
                        name: r#"TAG_CLOSE"#.to_string(),
                        pattern: r#">"#.to_string(),
                        is_regex: false,
                        line_number: 111,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"SELF_CLOSE"#.to_string(),
                        pattern: r#"/>"#.to_string(),
                        is_regex: false,
                        line_number: 112,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"SLASH"#.to_string(),
                        pattern: r#"/"#.to_string(),
                        is_regex: false,
                        line_number: 113,
                        alias: None,
                    },
                ] };
            __map.insert(r#"tag"#.to_string(), __g_tag);
            __map
        },
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
