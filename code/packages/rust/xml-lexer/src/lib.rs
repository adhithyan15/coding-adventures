//! XML lexer backed by compiled token grammar and callback-driven pattern groups.

use lexer::grammar_lexer::{GrammarLexer, LexerContext};
use lexer::token::Token;

mod _grammar;

fn effective_type_name(token: &Token) -> &str {
    match &token.type_name {
        Some(name) => name.as_str(),
        None => match token.type_ {
            lexer::token::TokenType::Eof => "EOF",
            _ => "UNKNOWN",
        },
    }
}

pub fn xml_on_token(token: &Token, ctx: &mut LexerContext) {
    let type_name = effective_type_name(token);

    match type_name {
        "OPEN_TAG_START" | "CLOSE_TAG_START" => {
            ctx.push_group("tag")
                .expect("'tag' group must exist in xml.tokens grammar");
        }
        "TAG_CLOSE" | "SELF_CLOSE" => {
            ctx.pop_group();
        }
        "COMMENT_START" => {
            ctx.push_group("comment")
                .expect("'comment' group must exist in xml.tokens grammar");
            ctx.set_skip_enabled(false);
        }
        "COMMENT_END" => {
            ctx.pop_group();
            ctx.set_skip_enabled(true);
        }
        "CDATA_START" => {
            ctx.push_group("cdata")
                .expect("'cdata' group must exist in xml.tokens grammar");
            ctx.set_skip_enabled(false);
        }
        "CDATA_END" => {
            ctx.pop_group();
            ctx.set_skip_enabled(true);
        }
        "PI_START" => {
            ctx.push_group("pi")
                .expect("'pi' group must exist in xml.tokens grammar");
            ctx.set_skip_enabled(false);
        }
        "PI_END" => {
            ctx.pop_group();
            ctx.set_skip_enabled(true);
        }
        _ => {}
    }
}

pub fn create_xml_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.set_on_token(Some(Box::new(xml_on_token)));
    lexer
}

pub fn tokenize_xml(source: &str) -> Vec<Token> {
    let mut xml_lexer = create_xml_lexer(source);
    xml_lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("XML tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper: extract (type_name, value) pairs excluding EOF.
    // -----------------------------------------------------------------------

    /// Extract the (type_name, value) pairs from a token stream, excluding
    /// the final EOF token. For XML tokens, the type name comes from the
    /// grammar's `type_name` field (e.g., "OPEN_TAG_START", "TAG_NAME").
    fn token_pairs(tokens: &[Token]) -> Vec<(&str, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (effective_type_name(t), t.value.as_str()))
            .collect()
    }

    /// Extract just the type names from a token stream, excluding EOF.
    fn token_types(tokens: &[Token]) -> Vec<&str> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| effective_type_name(t))
            .collect()
    }

    // =======================================================================
    // Basic Tags
    // =======================================================================

    /// A simple element: `<p>text</p>`.
    ///
    /// This is the most fundamental XML structure. The lexer should produce:
    /// OPEN_TAG_START, TAG_NAME, TAG_CLOSE, TEXT, CLOSE_TAG_START, TAG_NAME, TAG_CLOSE
    #[test]
    fn test_simple_element() {
        let tokens = tokenize_xml("<p>text</p>");
        let pairs = token_pairs(&tokens);
        assert_eq!(
            pairs,
            vec![
                ("OPEN_TAG_START", "<"),
                ("TAG_NAME", "p"),
                ("TAG_CLOSE", ">"),
                ("TEXT", "text"),
                ("CLOSE_TAG_START", "</"),
                ("TAG_NAME", "p"),
                ("TAG_CLOSE", ">"),
            ]
        );
    }

    /// Tags with XML namespace prefixes: `<ns:tag>`.
    ///
    /// Namespace prefixes use a colon separator. The TAG_NAME pattern
    /// allows colons, so `ns:tag` is a single TAG_NAME token.
    #[test]
    fn test_element_with_namespace() {
        let tokens = tokenize_xml("<ns:tag>content</ns:tag>");
        let types = token_types(&tokens);
        assert_eq!(
            types,
            vec![
                "OPEN_TAG_START", "TAG_NAME", "TAG_CLOSE",
                "TEXT",
                "CLOSE_TAG_START", "TAG_NAME", "TAG_CLOSE",
            ]
        );
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs[1], ("TAG_NAME", "ns:tag"));
    }

    /// An explicitly empty element: `<div></div>`.
    #[test]
    fn test_empty_element_explicit() {
        let tokens = tokenize_xml("<div></div>");
        let pairs = token_pairs(&tokens);
        assert_eq!(
            pairs,
            vec![
                ("OPEN_TAG_START", "<"),
                ("TAG_NAME", "div"),
                ("TAG_CLOSE", ">"),
                ("CLOSE_TAG_START", "</"),
                ("TAG_NAME", "div"),
                ("TAG_CLOSE", ">"),
            ]
        );
    }

    /// Self-closing tag: `<br/>`.
    ///
    /// A self-closing tag uses `/>` instead of a separate closing tag.
    /// The callback pops the "tag" group on SELF_CLOSE, just like TAG_CLOSE.
    #[test]
    fn test_self_closing_tag() {
        let tokens = tokenize_xml("<br/>");
        let pairs = token_pairs(&tokens);
        assert_eq!(
            pairs,
            vec![
                ("OPEN_TAG_START", "<"),
                ("TAG_NAME", "br"),
                ("SELF_CLOSE", "/>"),
            ]
        );
    }

    /// Self-closing with space before the slash: `<br />`.
    ///
    /// Whitespace inside tags is consumed by the skip pattern, so the
    /// space between `br` and `/>` is silently ignored.
    #[test]
    fn test_self_closing_with_space() {
        let tokens = tokenize_xml("<br />");
        let pairs = token_pairs(&tokens);
        assert_eq!(
            pairs,
            vec![
                ("OPEN_TAG_START", "<"),
                ("TAG_NAME", "br"),
                ("SELF_CLOSE", "/>"),
            ]
        );
    }

    // =======================================================================
    // Attributes
    // =======================================================================

    /// Attribute with double quotes: `class="main"`.
    ///
    /// Inside a tag, attribute names reuse the TAG_NAME pattern (same regex).
    /// The ATTR_VALUE token includes the surrounding quotes.
    #[test]
    fn test_double_quoted_attribute() {
        let tokens = tokenize_xml(r#"<div class="main">"#);
        let pairs = token_pairs(&tokens);
        assert_eq!(
            pairs,
            vec![
                ("OPEN_TAG_START", "<"),
                ("TAG_NAME", "div"),
                ("TAG_NAME", "class"),
                ("ATTR_EQUALS", "="),
                ("ATTR_VALUE", "\"main\""),
                ("TAG_CLOSE", ">"),
            ]
        );
    }

    /// Attribute with single quotes: `class='main'`.
    ///
    /// Both single and double quoted values alias to ATTR_VALUE.
    #[test]
    fn test_single_quoted_attribute() {
        let tokens = tokenize_xml("<div class='main'>");
        let pairs = token_pairs(&tokens);
        assert_eq!(
            pairs,
            vec![
                ("OPEN_TAG_START", "<"),
                ("TAG_NAME", "div"),
                ("TAG_NAME", "class"),
                ("ATTR_EQUALS", "="),
                ("ATTR_VALUE", "'main'"),
                ("TAG_CLOSE", ">"),
            ]
        );
    }

    /// Multiple attributes on one tag.
    #[test]
    fn test_multiple_attributes() {
        let tokens = tokenize_xml(r#"<a href="url" target="_blank">"#);
        let pairs = token_pairs(&tokens);
        let tag_names: Vec<&str> = pairs
            .iter()
            .filter(|(t, _)| *t == "TAG_NAME")
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(tag_names, vec!["a", "href", "target"]);

        let attr_values: Vec<&str> = pairs
            .iter()
            .filter(|(t, _)| *t == "ATTR_VALUE")
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(attr_values, vec!["\"url\"", "\"_blank\""]);
    }

    /// Attribute on a self-closing tag.
    #[test]
    fn test_attribute_on_self_closing() {
        let tokens = tokenize_xml(r#"<img src="photo.jpg"/>"#);
        let types = token_types(&tokens);
        assert!(types.contains(&"SELF_CLOSE"));
        assert!(types.contains(&"ATTR_VALUE"));
    }

    // =======================================================================
    // Comments
    // =======================================================================

    /// A simple comment: `<!-- hello -->`.
    ///
    /// The callback pushes "comment" group on COMMENT_START and disables
    /// skip so that the spaces around "hello" are preserved as COMMENT_TEXT.
    #[test]
    fn test_simple_comment() {
        let tokens = tokenize_xml("<!-- hello -->");
        let pairs = token_pairs(&tokens);
        assert_eq!(
            pairs,
            vec![
                ("COMMENT_START", "<!--"),
                ("COMMENT_TEXT", " hello "),
                ("COMMENT_END", "-->"),
            ]
        );
    }

    /// Whitespace inside comments is preserved (skip disabled).
    #[test]
    fn test_comment_preserves_whitespace() {
        let tokens = tokenize_xml("<!--  spaces  and\ttabs  -->");
        let texts: Vec<&str> = token_pairs(&tokens)
            .iter()
            .filter(|(t, _)| *t == "COMMENT_TEXT")
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(texts, vec!["  spaces  and\ttabs  "]);
    }

    /// Comments can contain single dashes (but not --).
    #[test]
    fn test_comment_with_dashes() {
        let tokens = tokenize_xml("<!-- a-b-c -->");
        let texts: Vec<&str> = token_pairs(&tokens)
            .iter()
            .filter(|(t, _)| *t == "COMMENT_TEXT")
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(texts, vec![" a-b-c "]);
    }

    /// Comment between two elements.
    #[test]
    fn test_comment_between_elements() {
        let tokens = tokenize_xml("<a/><!-- mid --><b/>");
        let types = token_types(&tokens);
        assert!(types.contains(&"COMMENT_START"));
        assert!(types.contains(&"COMMENT_END"));
    }

    // =======================================================================
    // CDATA Sections
    // =======================================================================

    /// A simple CDATA section.
    ///
    /// CDATA sections wrap raw character data — no entity processing,
    /// no tag recognition. The callback pushes "cdata" group and disables
    /// skip so whitespace is preserved.
    #[test]
    fn test_simple_cdata() {
        let tokens = tokenize_xml("<![CDATA[raw text]]>");
        let pairs = token_pairs(&tokens);
        assert_eq!(
            pairs,
            vec![
                ("CDATA_START", "<![CDATA["),
                ("CDATA_TEXT", "raw text"),
                ("CDATA_END", "]]>"),
            ]
        );
    }

    /// CDATA can contain `<` and `>` which are normally special.
    #[test]
    fn test_cdata_with_angle_brackets() {
        let tokens = tokenize_xml("<![CDATA[<not a tag>]]>");
        let texts: Vec<&str> = token_pairs(&tokens)
            .iter()
            .filter(|(t, _)| *t == "CDATA_TEXT")
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(texts, vec!["<not a tag>"]);
    }

    /// Whitespace in CDATA is preserved.
    #[test]
    fn test_cdata_preserves_whitespace() {
        let tokens = tokenize_xml("<![CDATA[  hello\n  world  ]]>");
        let texts: Vec<&str> = token_pairs(&tokens)
            .iter()
            .filter(|(t, _)| *t == "CDATA_TEXT")
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(texts, vec!["  hello\n  world  "]);
    }

    /// CDATA can contain `]` without ending (needs `]]>`).
    #[test]
    fn test_cdata_with_single_bracket() {
        let tokens = tokenize_xml("<![CDATA[a]b]]>");
        let texts: Vec<&str> = token_pairs(&tokens)
            .iter()
            .filter(|(t, _)| *t == "CDATA_TEXT")
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(texts, vec!["a]b"]);
    }

    // =======================================================================
    // Processing Instructions
    // =======================================================================

    /// The XML declaration: `<?xml version="1.0"?>`.
    ///
    /// Processing instructions have a target name and optional text content.
    /// The callback pushes "pi" group and disables skip so whitespace in
    /// the PI content is preserved.
    #[test]
    fn test_xml_declaration() {
        let tokens = tokenize_xml("<?xml version=\"1.0\"?>");
        let pairs = token_pairs(&tokens);
        assert_eq!(
            pairs,
            vec![
                ("PI_START", "<?"),
                ("PI_TARGET", "xml"),
                ("PI_TEXT", " version=\"1.0\""),
                ("PI_END", "?>"),
            ]
        );
    }

    /// A stylesheet processing instruction.
    #[test]
    fn test_stylesheet_pi() {
        let tokens = tokenize_xml("<?xml-stylesheet type=\"text/xsl\"?>");
        let types = token_types(&tokens);
        assert_eq!(types[0], "PI_START");
        assert_eq!(types[1], "PI_TARGET");
        assert_eq!(*types.last().unwrap(), "PI_END");
    }

    // =======================================================================
    // Entity and Character References
    // =======================================================================

    /// Named entity reference: `&amp;`.
    ///
    /// Entity references are recognized in the default group (between tags).
    /// They split text content into separate TEXT and ENTITY_REF tokens.
    #[test]
    fn test_named_entity() {
        let tokens = tokenize_xml("a&amp;b");
        let pairs = token_pairs(&tokens);
        assert_eq!(
            pairs,
            vec![
                ("TEXT", "a"),
                ("ENTITY_REF", "&amp;"),
                ("TEXT", "b"),
            ]
        );
    }

    /// Decimal character reference: `&#65;`.
    #[test]
    fn test_decimal_char_ref() {
        let tokens = tokenize_xml("&#65;");
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs, vec![("CHAR_REF", "&#65;")]);
    }

    /// Hexadecimal character reference: `&#x41;`.
    #[test]
    fn test_hex_char_ref() {
        let tokens = tokenize_xml("&#x41;");
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs, vec![("CHAR_REF", "&#x41;")]);
    }

    /// Multiple entity references in text.
    #[test]
    fn test_multiple_entities() {
        let tokens = tokenize_xml("&lt;hello&gt;");
        let types = token_types(&tokens);
        assert_eq!(types, vec!["ENTITY_REF", "TEXT", "ENTITY_REF"]);
    }

    // =======================================================================
    // Nested and Mixed Content
    // =======================================================================

    /// Nested elements: `<a><b>text</b></a>`.
    ///
    /// The callback pushes/pops the "tag" group independently for each tag.
    /// After each TAG_CLOSE, the lexer returns to the default group where
    /// it can see TEXT or another OPEN_TAG_START.
    #[test]
    fn test_nested_elements() {
        let tokens = tokenize_xml("<a><b>text</b></a>");
        let types = token_types(&tokens);
        assert_eq!(
            types.iter().filter(|t| **t == "OPEN_TAG_START").count(),
            2
        );
        assert_eq!(
            types.iter().filter(|t| **t == "CLOSE_TAG_START").count(),
            2
        );
    }

    /// Text mixed with child elements.
    #[test]
    fn test_mixed_content() {
        let tokens = tokenize_xml("<p>Hello <b>world</b>!</p>");
        let texts: Vec<&str> = token_pairs(&tokens)
            .iter()
            .filter(|(t, _)| *t == "TEXT")
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(texts, vec!["Hello ", "world", "!"]);
    }

    /// A small but complete XML document.
    ///
    /// This test exercises all five pattern groups in a single document:
    /// processing instruction, comment, tag with attributes, text content,
    /// entity reference, and EOF.
    #[test]
    fn test_full_document() {
        let source = concat!(
            "<?xml version=\"1.0\"?>",
            "<!-- A greeting -->",
            "<root lang=\"en\">",
            "<greeting>Hello &amp; welcome</greeting>",
            "</root>",
        );
        let tokens = tokenize_xml(source);
        let types = token_types(&tokens);

        // PI present
        assert!(types.contains(&"PI_START"));
        assert!(types.contains(&"PI_END"));

        // Comment present
        assert!(types.contains(&"COMMENT_START"));
        assert!(types.contains(&"COMMENT_END"));

        // Tags present (root + greeting = 2 each)
        assert_eq!(
            types.iter().filter(|t| **t == "OPEN_TAG_START").count(),
            2
        );
        assert_eq!(
            types.iter().filter(|t| **t == "CLOSE_TAG_START").count(),
            2
        );

        // Entity ref present
        assert!(types.contains(&"ENTITY_REF"));

        // Last token is EOF
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    /// CDATA section inside an element.
    #[test]
    fn test_cdata_inside_element() {
        let tokens = tokenize_xml("<script><![CDATA[x < y]]></script>");
        let types = token_types(&tokens);
        assert!(types.contains(&"CDATA_START"));
        assert!(types.contains(&"CDATA_TEXT"));
        assert!(types.contains(&"CDATA_END"));
    }

    // =======================================================================
    // Edge Cases
    // =======================================================================

    /// Empty input produces only EOF.
    #[test]
    fn test_empty_string() {
        let tokens = tokenize_xml("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].type_, TokenType::Eof);
    }

    /// Plain text with no tags.
    #[test]
    fn test_text_only() {
        let tokens = tokenize_xml("just text");
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs, vec![("TEXT", "just text")]);
    }

    /// Whitespace between tags is consumed by skip patterns.
    ///
    /// The XML grammar has a skip pattern for whitespace in the default group,
    /// so spaces between tags produce no TEXT tokens.
    #[test]
    fn test_whitespace_between_tags_skipped() {
        let tokens = tokenize_xml("<a> <b> </b> </a>");
        let texts: Vec<&str> = token_pairs(&tokens)
            .iter()
            .filter(|(t, _)| *t == "TEXT")
            .map(|(_, v)| *v)
            .collect();
        // Whitespace-only segments between tags are consumed by skip
        assert!(texts.is_empty());
    }

    /// The last token is always EOF.
    #[test]
    fn test_eof_always_last() {
        let tokens = tokenize_xml("<root/>");
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    /// Factory function returns a working lexer.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_xml_lexer("<p>hi</p>");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");
        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    /// Deeply nested elements work correctly.
    ///
    /// The group stack pushes/pops for each tag independently, so nesting
    /// depth is limited only by stack size, not by the lexer design.
    #[test]
    fn test_deeply_nested() {
        let tokens = tokenize_xml("<a><b><c>deep</c></b></a>");
        let types = token_types(&tokens);
        assert_eq!(
            types.iter().filter(|t| **t == "OPEN_TAG_START").count(),
            3
        );
        assert_eq!(
            types.iter().filter(|t| **t == "CLOSE_TAG_START").count(),
            3
        );
        // The text "deep" should be present
        let texts: Vec<&str> = token_pairs(&tokens)
            .iter()
            .filter(|(t, _)| *t == "TEXT")
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(texts, vec!["deep"]);
    }

    /// Multiple self-closing tags in sequence.
    #[test]
    fn test_multiple_self_closing() {
        let tokens = tokenize_xml("<br/><hr/><img/>");
        let types = token_types(&tokens);
        assert_eq!(
            types.iter().filter(|t| **t == "SELF_CLOSE").count(),
            3
        );
        assert_eq!(
            types.iter().filter(|t| **t == "OPEN_TAG_START").count(),
            3
        );
    }

    /// Tag names with hyphens and dots.
    ///
    /// The TAG_NAME regex allows `[a-zA-Z_][a-zA-Z0-9_:.-]*`, so names
    /// like `my-tag` and `my.tag` are valid single tokens.
    #[test]
    fn test_tag_name_with_special_chars() {
        let tokens = tokenize_xml("<my-tag.v2>text</my-tag.v2>");
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs[1], ("TAG_NAME", "my-tag.v2"));
        assert_eq!(pairs[5], ("TAG_NAME", "my-tag.v2"));
    }

    /// Empty comment.
    #[test]
    fn test_empty_comment() {
        let tokens = tokenize_xml("<!---->");
        let pairs = token_pairs(&tokens);
        assert_eq!(
            pairs,
            vec![
                ("COMMENT_START", "<!--"),
                ("COMMENT_END", "-->"),
            ]
        );
    }

    /// Comment and CDATA after each other.
    #[test]
    fn test_comment_then_cdata() {
        let tokens = tokenize_xml("<!-- comment --><![CDATA[data]]>");
        let types = token_types(&tokens);
        assert!(types.contains(&"COMMENT_START"));
        assert!(types.contains(&"COMMENT_END"));
        assert!(types.contains(&"CDATA_START"));
        assert!(types.contains(&"CDATA_END"));
    }
}

