//! # mosaic-lexer — Tokenizing `.mosaic` source text.
//!
//! The Mosaic Component Description Language (CDL) describes UI component
//! structure with named typed slots. This crate tokenizes `.mosaic` source text
//! into a vector of [`Token`] values using the grammar-driven lexer infrastructure.
//!
//! # Architecture
//!
//! ```text
//! mosaic.tokens          (grammar file on disk)
//!        |
//!        v
//! grammar-tools          (parses .tokens → TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer    (tokenizes source using TokenGrammar)
//! ```
//!
//! # Public API
//!
//! - [`tokenize`] — tokenize a `.mosaic` source string, returning `Vec<Token>`.
//! - [`create_mosaic_lexer`] — low-level factory returning a `GrammarLexer`.
//!
//! # Mosaic token categories
//!
//! | Category       | Examples                                              |
//! |----------------|-------------------------------------------------------|
//! | Keywords       | `component`, `slot`, `import`, `when`, `each`        |
//! | Identifiers    | `ProfileCard`, `avatar-url`, `padding`                |
//! | Strings        | `"./button.mosaic"`, `"Hello"`                        |
//! | Numbers        | `42`, `-3.14`                                         |
//! | Dimensions     | `16dp`, `100%`, `1.5sp`                               |
//! | Colors         | `#fff`, `#2563eb`, `#rrggbbaa`                        |
//! | Delimiters     | `{`, `}`, `<`, `>`, `:`, `;`, `,`, `.`, `=`, `@`     |
//! | Skip           | whitespace, `//` line comments, `/* */` block comments|

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;
mod _grammar;

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for Mosaic source text.
///
/// The returned lexer has the Mosaic token grammar loaded and is ready
/// to call `.tokenize()` on. Use this when you need fine-grained control
/// over tokenization (e.g. custom error handling or position tracking).
///
/// # Panics
///
/// Panics if the `mosaic.tokens` grammar file cannot be read or parsed.
/// This indicates a broken build or missing file — it should never happen
/// in a correctly cloned repository.
///
/// # Example
///
/// ```no_run
/// use mosaic_lexer::create_mosaic_lexer;
///
/// let mut lexer = create_mosaic_lexer("component Button { }");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// ```
pub fn create_mosaic_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize Mosaic source text into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, and tokenization in one call. The returned vector always
/// ends with an `EOF` token.
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed, or if the source
/// contains an unexpected character.
///
/// # Example
///
/// ```no_run
/// use mosaic_lexer::tokenize;
///
/// let tokens = tokenize(r#"
///   component Label {
///     slot text: text;
///     Text { content: @text; }
///   }
/// "#);
/// for tok in &tokens {
///     println!("{:?} {:?}", tok.type_, tok.value);
/// }
/// ```
pub fn tokenize(source: &str) -> Vec<Token> {
    let mut lexer = create_mosaic_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mosaic tokenization failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper: collect (type_name_or_type, value) for non-EOF tokens.
    // -----------------------------------------------------------------------

    /// Returns (canonical name, value) for each non-EOF token.
    ///
    /// For tokens whose type maps to a `TokenType` variant (e.g. STRING →
    /// TokenType::String), we use the variant debug name. For custom grammar
    /// token types that fall back to `TokenType::Name`, we use `type_name`.
    fn token_names(tokens: &[Token]) -> Vec<(String, String)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| {
                let name = match &t.type_name {
                    Some(n) => n.clone(),
                    None => format!("{:?}", t.type_),
                };
                (name, t.value.clone())
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: `component` keyword
    // -----------------------------------------------------------------------

    /// The word `component` must tokenize as a KEYWORD token.
    /// The value must be the literal text.
    #[test]
    fn test_component_keyword() {
        let tokens = tokenize("component");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].1, "component");
    }

    // -----------------------------------------------------------------------
    // Test 2: `slot` keyword
    // -----------------------------------------------------------------------

    #[test]
    fn test_slot_keyword() {
        let tokens = tokenize("slot");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].1, "slot");
    }

    // -----------------------------------------------------------------------
    // Test 3: NAME token (component identifier)
    // -----------------------------------------------------------------------

    /// Identifiers that are not keywords must tokenize as NAME tokens.
    #[test]
    fn test_name_token() {
        let tokens = tokenize("ProfileCard");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].1, "ProfileCard");
    }

    // -----------------------------------------------------------------------
    // Test 4: NAME token with hyphens
    // -----------------------------------------------------------------------

    /// Mosaic allows hyphens in identifiers for CSS-style property names.
    #[test]
    fn test_name_with_hyphens() {
        let tokens = tokenize("avatar-url");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].1, "avatar-url");
    }

    // -----------------------------------------------------------------------
    // Test 5: STRING token
    // -----------------------------------------------------------------------

    /// Double-quoted strings should be tokenized as STRING tokens.
    #[test]
    fn test_string_token() {
        let tokens = tokenize("\"./button.mosaic\"");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].0, "String");
        // The value should contain the path content.
        assert!(names[0].1.contains("button.mosaic"));
    }

    // -----------------------------------------------------------------------
    // Test 6: NUMBER token
    // -----------------------------------------------------------------------

    #[test]
    fn test_number_token() {
        let tokens = tokenize("42");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].0, "Number");
        assert_eq!(names[0].1, "42");
    }

    // -----------------------------------------------------------------------
    // Test 7: DIMENSION token (number + unit)
    // -----------------------------------------------------------------------

    /// A dimension like `16dp` must be a single DIMENSION token, not split
    /// into a number and a name. This matters because the grammar requires
    /// DIMENSION to be matched before NUMBER (same as Lattice/CSS ordering).
    #[test]
    fn test_dimension_token() {
        let tokens = tokenize("16dp");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].0, "DIMENSION");
        assert_eq!(names[0].1, "16dp");
    }

    // -----------------------------------------------------------------------
    // Test 8: COLOR_HEX token
    // -----------------------------------------------------------------------

    #[test]
    fn test_color_hex_token() {
        let tokens = tokenize("#2563eb");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].0, "COLOR_HEX");
        assert_eq!(names[0].1, "#2563eb");
    }

    // -----------------------------------------------------------------------
    // Test 9: Three-digit color shorthand
    // -----------------------------------------------------------------------

    #[test]
    fn test_color_hex_short() {
        let tokens = tokenize("#fff");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].0, "COLOR_HEX");
        assert_eq!(names[0].1, "#fff");
    }

    // -----------------------------------------------------------------------
    // Test 10: Structural delimiters
    // -----------------------------------------------------------------------

    /// All 10 single-character delimiters from the grammar must tokenize.
    #[test]
    fn test_delimiters() {
        let tokens = tokenize("{ } < > : ; , . = @");
        let non_eof: Vec<&Token> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .collect();

        assert_eq!(non_eof.len(), 10, "Expected 10 delimiter tokens");
        assert_eq!(non_eof[0].value, "{");
        assert_eq!(non_eof[1].value, "}");
        assert_eq!(non_eof[9].value, "@");
    }

    // -----------------------------------------------------------------------
    // Test 11: Line comment is skipped
    // -----------------------------------------------------------------------

    #[test]
    fn test_line_comment_skipped() {
        let tokens = tokenize("// this is a comment\ncomponent");
        let names = token_names(&tokens);

        // Only the component keyword should appear.
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].1, "component");
    }

    // -----------------------------------------------------------------------
    // Test 12: Block comment is skipped
    // -----------------------------------------------------------------------

    #[test]
    fn test_block_comment_skipped() {
        let tokens = tokenize("/* multi\nline */slot");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].1, "slot");
    }

    // -----------------------------------------------------------------------
    // Test 13: Whitespace is skipped
    // -----------------------------------------------------------------------

    #[test]
    fn test_whitespace_skipped() {
        let compact = tokenize("component{}");
        let spaced = tokenize("component { }");

        let compact_vals: Vec<String> = compact
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.clone())
            .collect();
        let spaced_vals: Vec<String> = spaced
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.clone())
            .collect();

        assert_eq!(compact_vals, spaced_vals);
    }

    // -----------------------------------------------------------------------
    // Test 14: `when` and `each` keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_when_each_keywords() {
        let tokens = tokenize("when each");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 2);
        assert_eq!(names[0].1, "when");
        assert_eq!(names[1].1, "each");
    }

    // -----------------------------------------------------------------------
    // Test 15: `import from as` keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_import_from_as_keywords() {
        let tokens = tokenize("import from as");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 3);
        assert_eq!(names[0].1, "import");
        assert_eq!(names[1].1, "from");
        assert_eq!(names[2].1, "as");
    }

    // -----------------------------------------------------------------------
    // Test 16: Slot type keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_slot_type_keywords() {
        let tokens = tokenize("text number bool image color node list");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 7);
        let vals: Vec<String> = names.iter().map(|(_, v)| v.clone()).collect();
        assert!(vals.contains(&"text".to_string()));
        assert!(vals.contains(&"number".to_string()));
        assert!(vals.contains(&"bool".to_string()));
        assert!(vals.contains(&"image".to_string()));
        assert!(vals.contains(&"color".to_string()));
        assert!(vals.contains(&"node".to_string()));
        assert!(vals.contains(&"list".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 17: `true` and `false` keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_bool_keywords() {
        let tokens = tokenize("true false");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 2);
        assert_eq!(names[0].1, "true");
        assert_eq!(names[1].1, "false");
    }

    // -----------------------------------------------------------------------
    // Test 18: Minimal component declaration
    // -----------------------------------------------------------------------

    /// A tiny but complete component declaration exercises the full token set.
    #[test]
    fn test_minimal_component() {
        let src = r#"component Label { slot text: text; Text { content: @text; } }"#;
        let tokens = tokenize(src);

        // Should not panic and must end with EOF.
        assert_eq!(
            tokens.last().unwrap().type_,
            TokenType::Eof,
            "Last token must be EOF"
        );
        // There should be many tokens.
        assert!(tokens.len() > 10, "Expected more than 10 tokens");
    }

    // -----------------------------------------------------------------------
    // Test 19: Negative number
    // -----------------------------------------------------------------------

    #[test]
    fn test_negative_number() {
        let tokens = tokenize("-42");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].0, "Number");
        assert_eq!(names[0].1, "-42");
    }

    // -----------------------------------------------------------------------
    // Test 20: Percentage dimension
    // -----------------------------------------------------------------------

    #[test]
    fn test_percent_dimension() {
        let tokens = tokenize("100%");
        let names = token_names(&tokens);

        assert_eq!(names.len(), 1);
        assert_eq!(names[0].0, "DIMENSION");
        assert_eq!(names[0].1, "100%");
    }
}
