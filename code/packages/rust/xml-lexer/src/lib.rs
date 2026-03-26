//! # XML Lexer — tokenizing XML source text with pattern groups.
//!
//! [XML](https://www.w3.org/XML/) (Extensible Markup Language) is a markup
//! language for structured data. Unlike JSON, XML is **context-sensitive**
//! at the lexical level: the same character has different meaning depending
//! on where it appears.
//!
//! For example, the equals sign `=`:
//! - Inside a tag `<div class="main">`: attribute delimiter
//! - Outside a tag `1 + 1 = 2`: plain text content
//!
//! A flat list of patterns cannot distinguish these contexts. This crate
//! solves the problem using **pattern groups** and an **on-token callback**
//! — two features of the grammar-driven lexer infrastructure.
//!
//! # Architecture
//!
//! ```text
//! xml.tokens           (grammar file — 5 pattern groups)
//!        |
//!        v
//! grammar-tools        (parses .tokens -> TokenGrammar with groups)
//!        |
//!        v
//! lexer::GrammarLexer  (tokenizes using active group's patterns)
//!        |
//!        v
//! xml-lexer            (this crate — callback + glue layer)
//! ```
//!
//! # Pattern Groups
//!
//! The `xml.tokens` grammar defines 5 pattern groups:
//!
//! | Group     | Active when...                        | Recognizes                           |
//! |-----------|---------------------------------------|--------------------------------------|
//! | default   | Between tags (initial state)          | TEXT, entities, tag/comment openers   |
//! | tag       | Inside `<tag ...>` or `</tag>`        | TAG_NAME, ATTR_EQUALS, ATTR_VALUE    |
//! | comment   | Inside `<!-- ... -->`                 | COMMENT_TEXT, COMMENT_END            |
//! | cdata     | Inside `<![CDATA[ ... ]]>`            | CDATA_TEXT, CDATA_END                |
//! | pi        | Inside `<? ... ?>`                    | PI_TARGET, PI_TEXT, PI_END           |
//!
//! # The Callback — `xml_on_token`
//!
//! The [`xml_on_token`] function fires after each token match. It examines
//! the token's type name and pushes/pops groups on the lexer's group stack:
//!
//! ```text
//! default ──OPEN_TAG_START──> tag ──TAG_CLOSE──> default
//!         ──CLOSE_TAG_START─> tag ──SELF_CLOSE─> default
//!         ──COMMENT_START───> comment ──COMMENT_END──> default
//!         ──CDATA_START─────> cdata ──CDATA_END──> default
//!         ──PI_START────────> pi ──PI_END──> default
//! ```
//!
//! For comment, CDATA, and PI groups, the callback also disables skip
//! patterns (so whitespace is preserved as content) and re-enables them
//! when leaving the group.
//!
//! # Public API
//!
//! - [`create_xml_lexer`] — returns a `GrammarLexer` with the callback registered.
//! - [`tokenize_xml`] — convenience function that returns `Vec<Token>` directly.
//! - [`xml_on_token`] — the callback function, exposed for testing and reuse.

use std::fs;

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::{GrammarLexer, LexerContext};
use lexer::token::Token;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `xml_rust.tokens` grammar file.
///
/// We use `env!("CARGO_MANIFEST_DIR")` to get the directory containing this
/// crate's `Cargo.toml` at compile time. From there, we navigate up to the
/// `grammars/` directory at the repository root.
///
/// The directory structure looks like:
///
/// ```text
/// code/
///   grammars/
///     xml_rust.tokens       <-- this is what we want
///   packages/
///     rust/
///       xml-lexer/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR points here
///         src/
///           lib.rs          <-- we are here
/// ```
///
/// We use `xml_rust.tokens` instead of `xml.tokens` because the original
/// grammar uses negative lookahead patterns (`(?!...)`) which Python's
/// regex engine supports but Rust's `regex` crate does not. The Rust
/// variant replaces lookahead with equivalent alternation patterns.
///
/// The relative path from CARGO_MANIFEST_DIR to the grammar file is:
/// `../../../grammars/xml_rust.tokens`
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/xml_rust.tokens")
}

// ===========================================================================
// XML On-Token Callback
// ===========================================================================
//
// This callback drives the pattern group transitions. It is the heart of
// the XML lexer — without it, the lexer would only use the default group
// and fail to tokenize anything inside tags, comments, CDATA, or PIs.
//
// The logic is a simple state machine:
// - Opening delimiters push a group onto the stack
// - Closing delimiters pop the group off the stack
// - Comment/CDATA/PI groups disable skip (whitespace is content)
// ===========================================================================

/// Resolve a token's effective type name.
///
/// The grammar-driven lexer uses `type_name` for custom token types defined
/// in the `.tokens` file (like `OPEN_TAG_START`, `TAG_NAME`, etc.). Built-in
/// types (like `Eof`) use the `type_` field instead. This helper returns
/// the type name string regardless of which field holds it.
///
/// Most XML tokens are custom types, so `type_name` is almost always `Some`.
/// The only exception is `EOF`, which is a built-in type.
fn effective_type_name(token: &Token) -> &str {
    match &token.type_name {
        Some(name) => name.as_str(),
        None => match token.type_ {
            lexer::token::TokenType::Eof => "EOF",
            _ => "UNKNOWN",
        },
    }
}

/// Callback that switches pattern groups for XML tokenization.
///
/// This function fires after each token match. It examines the token's
/// type name and pushes/pops pattern groups on the lexer's group stack:
///
/// - **`OPEN_TAG_START`** (`<`) or **`CLOSE_TAG_START`** (`</`):
///   Push the `"tag"` group so the lexer recognizes tag names, attributes,
///   and tag closers.
///
/// - **`TAG_CLOSE`** (`>`) or **`SELF_CLOSE`** (`/>`):
///   Pop the `"tag"` group to return to default (text content).
///
/// - **`COMMENT_START`** (`<!--`):
///   Push `"comment"` group and disable skip (whitespace is significant).
///
/// - **`COMMENT_END`** (`-->`):
///   Pop `"comment"` group and re-enable skip.
///
/// - **`CDATA_START`** (`<![CDATA[`):
///   Push `"cdata"` group and disable skip.
///
/// - **`CDATA_END`** (`]]>`):
///   Pop `"cdata"` group and re-enable skip.
///
/// - **`PI_START`** (`<?`):
///   Push `"pi"` group and disable skip.
///
/// - **`PI_END`** (`?>`):
///   Pop `"pi"` group and re-enable skip.
pub fn xml_on_token(token: &Token, ctx: &mut LexerContext) {
    let type_name = effective_type_name(token);

    match type_name {
        // --- Tag boundaries ---
        //
        // When we see `<` (open tag) or `</` (close tag), we enter tag
        // mode where the lexer recognizes tag names, attributes, etc.
        // When we see `>` or `/>`, we leave tag mode and return to the
        // default group where text content and entities are recognized.
        "OPEN_TAG_START" | "CLOSE_TAG_START" => {
            ctx.push_group("tag")
                .expect("'tag' group must exist in xml.tokens grammar");
        }
        "TAG_CLOSE" | "SELF_CLOSE" => {
            ctx.pop_group();
        }

        // --- Comment boundaries ---
        //
        // Comments preserve all whitespace — `<!-- hello  world -->` should
        // keep the double space. We disable skip patterns so the lexer does
        // not silently consume whitespace inside comments.
        "COMMENT_START" => {
            ctx.push_group("comment")
                .expect("'comment' group must exist in xml.tokens grammar");
            ctx.set_skip_enabled(false);
        }
        "COMMENT_END" => {
            ctx.pop_group();
            ctx.set_skip_enabled(true);
        }

        // --- CDATA boundaries ---
        //
        // CDATA sections are raw text — no entity processing, no tag
        // recognition. Like comments, whitespace is significant content.
        "CDATA_START" => {
            ctx.push_group("cdata")
                .expect("'cdata' group must exist in xml.tokens grammar");
            ctx.set_skip_enabled(false);
        }
        "CDATA_END" => {
            ctx.pop_group();
            ctx.set_skip_enabled(true);
        }

        // --- Processing instruction boundaries ---
        //
        // PIs like `<?xml version="1.0"?>` contain a target name and
        // optional text content. Whitespace is significant (it separates
        // the target from the content and appears within the content).
        "PI_START" => {
            ctx.push_group("pi")
                .expect("'pi' group must exist in xml.tokens grammar");
            ctx.set_skip_enabled(false);
        }
        "PI_END" => {
            ctx.pop_group();
            ctx.set_skip_enabled(true);
        }

        // All other tokens (TEXT, TAG_NAME, ATTR_VALUE, etc.) do not
        // trigger group transitions — the lexer stays in its current group.
        _ => {}
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for XML source text.
///
/// This function:
/// 1. Reads the `xml.tokens` grammar file from disk.
/// 2. Parses it into a `TokenGrammar` using `grammar-tools`.
/// 3. Constructs a `GrammarLexer` with the grammar and the given source.
/// 4. Registers the [`xml_on_token`] callback for pattern group switching.
///
/// The returned lexer is ready to call `.tokenize()` on. Use this when you
/// need access to the lexer object itself (e.g., for incremental tokenization
/// or custom error handling).
///
/// # Panics
///
/// Panics if the grammar file cannot be read or parsed. This should never
/// happen in practice — the grammar file is checked into the repository and
/// validated by the grammar-tools test suite.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_xml_lexer::create_xml_lexer;
///
/// let mut lexer = create_xml_lexer("<div>hello</div>");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{}", token);
/// }
/// ```
pub fn create_xml_lexer(source: &str) -> GrammarLexer<'_> {
    // Step 1: Read the grammar file from disk.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read xml_rust.tokens: {e}"));

    // Step 2: Parse the grammar text into a structured TokenGrammar.
    //
    // The TokenGrammar now contains pattern groups (tag, comment, cdata, pi)
    // in addition to the default group. Each group has its own set of token
    // patterns that are active only when that group is at the top of the stack.
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse xml_rust.tokens: {e}"));

    // Step 3: Create the lexer and register the callback.
    //
    // The callback is what makes this lexer context-sensitive. Without it,
    // only the default group's patterns would ever be active.
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.set_on_token(Some(Box::new(xml_on_token)));
    lexer
}

/// Tokenize XML source text into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, callback registration, and tokenization in one call.
/// The returned vector always ends with an `EOF` token.
///
/// # Token types
///
/// **Default group** (content between tags):
/// - **TEXT** — text content (e.g., `Hello world`)
/// - **ENTITY_REF** — entity reference (e.g., `&amp;`)
/// - **CHAR_REF** — character reference (e.g., `&#65;`, `&#x41;`)
/// - **OPEN_TAG_START** — `<`
/// - **CLOSE_TAG_START** — `</`
/// - **COMMENT_START** — `<!--`
/// - **CDATA_START** — `<![CDATA[`
/// - **PI_START** — `<?`
///
/// **Tag group** (inside tags):
/// - **TAG_NAME** — tag or attribute name (e.g., `div`, `class`)
/// - **ATTR_EQUALS** — `=`
/// - **ATTR_VALUE** — quoted attribute value (e.g., `"main"`)
/// - **TAG_CLOSE** — `>`
/// - **SELF_CLOSE** — `/>`
///
/// **Comment group**:
/// - **COMMENT_TEXT** — comment content
/// - **COMMENT_END** — `-->`
///
/// **CDATA group**:
/// - **CDATA_TEXT** — raw text content
/// - **CDATA_END** — `]]>`
///
/// **Processing instruction group**:
/// - **PI_TARGET** — PI target name (e.g., `xml`)
/// - **PI_TEXT** — PI content
/// - **PI_END** — `?>`
///
/// **Always present**:
/// - **EOF** — end of input
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed, or if the source
/// contains characters that don't match any token pattern in the active group.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_xml_lexer::tokenize_xml;
///
/// let tokens = tokenize_xml("<p>Hello &amp; world</p>");
/// for token in &tokens {
///     println!("{:?} {:?}", token.type_, token.value);
/// }
/// ```
pub fn tokenize_xml(source: &str) -> Vec<Token> {
    let mut xml_lexer = create_xml_lexer(source);
    xml_lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("XML tokenization failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

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
