// AUTO-GENERATED FILE — DO NOT EDIT
// Source: xml_rust.tokens
// Regenerate with: grammar-tools compile-tokens xml_rust.tokens
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
                name: r#"TEXT"#.to_string(),
                pattern: r#"[^<&]+"#.to_string(),
                is_regex: true,
                line_number: 44,
                alias: None,
            },
            TokenDefinition {
                name: r#"ENTITY_REF"#.to_string(),
                pattern: r#"&[a-zA-Z][a-zA-Z0-9]*;"#.to_string(),
                is_regex: true,
                line_number: 45,
                alias: None,
            },
            TokenDefinition {
                name: r#"CHAR_REF"#.to_string(),
                pattern: r#"&#[0-9]+;|&#x[0-9a-fA-F]+;"#.to_string(),
                is_regex: true,
                line_number: 46,
                alias: None,
            },
            TokenDefinition {
                name: r#"COMMENT_START"#.to_string(),
                pattern: r#"<!--"#.to_string(),
                is_regex: false,
                line_number: 47,
                alias: None,
            },
            TokenDefinition {
                name: r#"CDATA_START"#.to_string(),
                pattern: r#"<![CDATA["#.to_string(),
                is_regex: false,
                line_number: 48,
                alias: None,
            },
            TokenDefinition {
                name: r#"PI_START"#.to_string(),
                pattern: r#"<?"#.to_string(),
                is_regex: false,
                line_number: 49,
                alias: None,
            },
            TokenDefinition {
                name: r#"CLOSE_TAG_START"#.to_string(),
                pattern: r#"</"#.to_string(),
                is_regex: false,
                line_number: 50,
                alias: None,
            },
            TokenDefinition {
                name: r#"OPEN_TAG_START"#.to_string(),
                pattern: r#"<"#.to_string(),
                is_regex: false,
                line_number: 51,
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
                line_number: 38,
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
                        name: r#"CDATA_TEXT"#.to_string(),
                        pattern: r#"([^\]]+|\][^\]]|\]\][^>])+"#.to_string(),
                        is_regex: true,
                        line_number: 90,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"CDATA_END"#.to_string(),
                        pattern: r#"]]>"#.to_string(),
                        is_regex: false,
                        line_number: 91,
                        alias: None,
                    },
                ] };
            __map.insert(r#"cdata"#.to_string(), __g_cdata);
            let mut __g_comment = PatternGroup { name: r#"comment"#.to_string(), definitions: vec![
                    TokenDefinition {
                        name: r#"COMMENT_TEXT"#.to_string(),
                        pattern: r#"([^-]+|-[^-]|--[^>])+"#.to_string(),
                        is_regex: true,
                        line_number: 77,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"COMMENT_END"#.to_string(),
                        pattern: r#"-->"#.to_string(),
                        is_regex: false,
                        line_number: 78,
                        alias: None,
                    },
                ] };
            __map.insert(r#"comment"#.to_string(), __g_comment);
            let mut __g_pi = PatternGroup { name: r#"pi"#.to_string(), definitions: vec![
                    TokenDefinition {
                        name: r#"PI_TARGET"#.to_string(),
                        pattern: r#"[a-zA-Z_][a-zA-Z0-9_:.-]*"#.to_string(),
                        is_regex: true,
                        line_number: 102,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"PI_TEXT"#.to_string(),
                        pattern: r#"([^?]+|\?[^>])+"#.to_string(),
                        is_regex: true,
                        line_number: 103,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"PI_END"#.to_string(),
                        pattern: r#"?>"#.to_string(),
                        is_regex: false,
                        line_number: 104,
                        alias: None,
                    },
                ] };
            __map.insert(r#"pi"#.to_string(), __g_pi);
            let mut __g_tag = PatternGroup { name: r#"tag"#.to_string(), definitions: vec![
                    TokenDefinition {
                        name: r#"TAG_NAME"#.to_string(),
                        pattern: r#"[a-zA-Z_][a-zA-Z0-9_:.-]*"#.to_string(),
                        is_regex: true,
                        line_number: 58,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"ATTR_EQUALS"#.to_string(),
                        pattern: r#"="#.to_string(),
                        is_regex: false,
                        line_number: 59,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"ATTR_VALUE_DQ"#.to_string(),
                        pattern: r#""[^"]*""#.to_string(),
                        is_regex: true,
                        line_number: 60,
                        alias: Some(r#"ATTR_VALUE"#.to_string()),
                    },
                    TokenDefinition {
                        name: r#"ATTR_VALUE_SQ"#.to_string(),
                        pattern: r#"'[^']*'"#.to_string(),
                        is_regex: true,
                        line_number: 61,
                        alias: Some(r#"ATTR_VALUE"#.to_string()),
                    },
                    TokenDefinition {
                        name: r#"TAG_CLOSE"#.to_string(),
                        pattern: r#">"#.to_string(),
                        is_regex: false,
                        line_number: 62,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"SELF_CLOSE"#.to_string(),
                        pattern: r#"/>"#.to_string(),
                        is_regex: false,
                        line_number: 63,
                        alias: None,
                    },
                    TokenDefinition {
                        name: r#"SLASH"#.to_string(),
                        pattern: r#"/"#.to_string(),
                        is_regex: false,
                        line_number: 64,
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
    }
}
