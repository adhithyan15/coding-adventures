// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use std::collections::HashMap;
use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition, PatternGroup};

pub fn XmlTokens() -> TokenGrammar {
    let mut groups = HashMap::new();
    groups.insert("comment".to_string(), PatternGroup {
        name: "comment".to_string(),
        definitions: vec![
            TokenDefinition { name: "COMMENT_TEXT".to_string(), pattern: "([^-]+|-[^-]|--[^>])+".to_string(), is_regex: true, line_number: 77, alias: None },
            TokenDefinition { name: "COMMENT_END".to_string(), pattern: "-->".to_string(), is_regex: false, line_number: 78, alias: None },
        ],
    });
    groups.insert("pi".to_string(), PatternGroup {
        name: "pi".to_string(),
        definitions: vec![
            TokenDefinition { name: "PI_TARGET".to_string(), pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*".to_string(), is_regex: true, line_number: 102, alias: None },
            TokenDefinition { name: "PI_TEXT".to_string(), pattern: "([^?]+|\\?[^>])+".to_string(), is_regex: true, line_number: 103, alias: None },
            TokenDefinition { name: "PI_END".to_string(), pattern: "?>".to_string(), is_regex: false, line_number: 104, alias: None },
        ],
    });
    groups.insert("tag".to_string(), PatternGroup {
        name: "tag".to_string(),
        definitions: vec![
            TokenDefinition { name: "TAG_NAME".to_string(), pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*".to_string(), is_regex: true, line_number: 58, alias: None },
            TokenDefinition { name: "ATTR_EQUALS".to_string(), pattern: "=".to_string(), is_regex: false, line_number: 59, alias: None },
            TokenDefinition { name: "ATTR_VALUE_DQ".to_string(), pattern: "\"[^\"]*\"".to_string(), is_regex: true, line_number: 60, alias: Some("ATTR_VALUE".to_string()) },
            TokenDefinition { name: "ATTR_VALUE_SQ".to_string(), pattern: "'[^']*'".to_string(), is_regex: true, line_number: 61, alias: Some("ATTR_VALUE".to_string()) },
            TokenDefinition { name: "TAG_CLOSE".to_string(), pattern: ">".to_string(), is_regex: false, line_number: 62, alias: None },
            TokenDefinition { name: "SELF_CLOSE".to_string(), pattern: "/>".to_string(), is_regex: false, line_number: 63, alias: None },
            TokenDefinition { name: "SLASH".to_string(), pattern: "/".to_string(), is_regex: false, line_number: 64, alias: None },
        ],
    });
    groups.insert("cdata".to_string(), PatternGroup {
        name: "cdata".to_string(),
        definitions: vec![
            TokenDefinition { name: "CDATA_TEXT".to_string(), pattern: "([^\\]]+|\\][^\\]]|\\]\\][^>])+".to_string(), is_regex: true, line_number: 90, alias: None },
            TokenDefinition { name: "CDATA_END".to_string(), pattern: "]]>".to_string(), is_regex: false, line_number: 91, alias: None },
        ],
    });
    TokenGrammar {
        version: 1,
        case_insensitive: false,
        case_sensitive: true,
        mode: None,
        escapes: Some("none".to_string()),
        keywords: vec![],
        reserved_keywords: vec![],
        definitions: vec![
            TokenDefinition { name: "TEXT".to_string(), pattern: "[^<&]+".to_string(), is_regex: true, line_number: 44, alias: None },
            TokenDefinition { name: "ENTITY_REF".to_string(), pattern: "&[a-zA-Z][a-zA-Z0-9]*;".to_string(), is_regex: true, line_number: 45, alias: None },
            TokenDefinition { name: "CHAR_REF".to_string(), pattern: "&#[0-9]+;|&#x[0-9a-fA-F]+;".to_string(), is_regex: true, line_number: 46, alias: None },
            TokenDefinition { name: "COMMENT_START".to_string(), pattern: "<!--".to_string(), is_regex: false, line_number: 47, alias: None },
            TokenDefinition { name: "CDATA_START".to_string(), pattern: "<![CDATA[".to_string(), is_regex: false, line_number: 48, alias: None },
            TokenDefinition { name: "PI_START".to_string(), pattern: "<?".to_string(), is_regex: false, line_number: 49, alias: None },
            TokenDefinition { name: "CLOSE_TAG_START".to_string(), pattern: "</".to_string(), is_regex: false, line_number: 50, alias: None },
            TokenDefinition { name: "OPEN_TAG_START".to_string(), pattern: "<".to_string(), is_regex: false, line_number: 51, alias: None },
        ],
        skip_definitions: vec![
            TokenDefinition { name: "WHITESPACE".to_string(), pattern: "[ \\t\\r\\n]+".to_string(), is_regex: true, line_number: 38, alias: None },
        ],
        error_definitions: vec![
        ],
        groups,
    }
}
