//! Lattice lexer backed by compiled token grammar.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub fn create_lattice_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

pub fn tokenize_lattice(source: &str) -> Vec<Token> {
    let mut lexer = create_lattice_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Lattice tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper functions
    // -----------------------------------------------------------------------

    /// Collect (type_name_or_built_in, value) pairs, excluding the EOF token.
    ///
    /// We represent token types as strings for assertion simplicity:
    /// - Built-in types use their enum name: "Number", "String", "Colon", etc.
    /// - Custom types (VARIABLE, AT_KEYWORD, etc.) use the grammar name.
    fn token_info(tokens: &[Token]) -> Vec<(String, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| {
                let type_str = if let Some(ref name) = t.type_name {
                    name.clone()
                } else {
                    format!("{:?}", t.type_)
                };
                (type_str, t.value.as_str())
            })
            .collect()
    }

    /// Extract just the type name strings, excluding EOF.
    fn type_names(tokens: &[Token]) -> Vec<String> {
        token_info(tokens).into_iter().map(|(t, _)| t).collect()
    }

    /// Extract just the token value strings, excluding EOF.
    fn values(tokens: &[Token]) -> Vec<&str> {
        token_info(tokens).into_iter().map(|(_, v)| v).collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: Variable token ($color)
    // -----------------------------------------------------------------------

    /// The VARIABLE token matches `$` followed by an identifier. This is a
    /// Lattice-specific token — standard CSS never uses `$`.
    ///
    /// VARIABLE must come before DIMENSION/PERCENTAGE/NUMBER in the token
    /// grammar so that `$10px` is not mistakenly tokenized as `$` + DIMENSION.
    #[test]
    fn test_variable_token() {
        let tokens = tokenize_lattice("$color");
        let info = token_info(&tokens);

        assert_eq!(info.len(), 1, "Expected 1 non-EOF token");
        assert_eq!(info[0].0, "VARIABLE");
        assert_eq!(info[0].1, "$color");
    }

    // -----------------------------------------------------------------------
    // Test 2: Variable declaration
    // -----------------------------------------------------------------------

    /// A full variable declaration: `$primary: #4a90d9;`
    /// Produces: VARIABLE, COLON, HASH, SEMICOLON, EOF
    #[test]
    fn test_variable_declaration() {
        let tokens = tokenize_lattice("$primary: #4a90d9;");
        let info = token_info(&tokens);

        assert_eq!(info.len(), 4, "Expected VARIABLE COLON HASH SEMICOLON");
        assert_eq!(info[0].0, "VARIABLE");
        assert_eq!(info[0].1, "$primary");
        assert_eq!(info[1].0, "Colon");
        assert_eq!(info[1].1, ":");
        assert_eq!(info[2].0, "HASH");
        assert_eq!(info[2].1, "#4a90d9");
        assert_eq!(info[3].0, "Semicolon");
    }

    // -----------------------------------------------------------------------
    // Test 3: Comparison operators (Lattice extensions)
    // -----------------------------------------------------------------------

    /// Lattice adds four comparison operators for @if expressions.
    /// Each must be matched BEFORE its single-character components:
    ///   `==` before `=`, `!=` before `!`, `>=` before `>`, `<=` before `<`
    #[test]
    fn test_comparison_operators() {
        let tokens_eq = tokenize_lattice("==");
        let info_eq = token_info(&tokens_eq);
        assert_eq!(info_eq.len(), 1);
        // EQUALS_EQUALS maps to the built-in TokenType::EqualsEquals, so
        // type_name is None and Debug format gives "EqualsEquals".
        assert_eq!(info_eq[0].0, "EqualsEquals");
        assert_eq!(info_eq[0].1, "==");

        let tokens_ne = tokenize_lattice("!=");
        let info_ne = token_info(&tokens_ne);
        assert_eq!(info_ne.len(), 1);
        assert_eq!(info_ne[0].0, "NOT_EQUALS");
        assert_eq!(info_ne[0].1, "!=");

        let tokens_ge = tokenize_lattice(">=");
        let info_ge = token_info(&tokens_ge);
        assert_eq!(info_ge.len(), 1);
        assert_eq!(info_ge[0].0, "GREATER_EQUALS");
        assert_eq!(info_ge[0].1, ">=");

        let tokens_le = tokenize_lattice("<=");
        let info_le = token_info(&tokens_le);
        assert_eq!(info_le.len(), 1);
        assert_eq!(info_le[0].0, "LESS_EQUALS");
        assert_eq!(info_le[0].1, "<=");
    }

    // -----------------------------------------------------------------------
    // Test 4: CSS dimension, percentage, number tokens
    // -----------------------------------------------------------------------

    /// CSS numeric tokens. DIMENSION comes first in the grammar (absorbs
    /// the unit), then PERCENTAGE (absorbs the %), then plain NUMBER.
    #[test]
    fn test_numeric_tokens() {
        let tokens = tokenize_lattice("16px 50% 3.14");
        let info = token_info(&tokens);

        assert_eq!(info.len(), 3);
        assert_eq!(info[0].0, "DIMENSION");
        assert_eq!(info[0].1, "16px");
        assert_eq!(info[1].0, "PERCENTAGE");
        assert_eq!(info[1].1, "50%");
        assert_eq!(info[2].0, "Number");
        assert_eq!(info[2].1, "3.14");
    }

    // -----------------------------------------------------------------------
    // Test 5: String tokens (double and single quoted)
    // -----------------------------------------------------------------------

    /// Lattice supports both double-quoted and single-quoted strings.
    /// Both produce the same STRING token type (via the -> alias in the grammar).
    #[test]
    fn test_string_tokens() {
        let tokens_dq = tokenize_lattice("\"hello\"");
        let info_dq = token_info(&tokens_dq);
        assert_eq!(info_dq.len(), 1);
        assert_eq!(info_dq[0].0, "String");
        assert_eq!(info_dq[0].1, "hello");

        let tokens_sq = tokenize_lattice("'world'");
        let info_sq = token_info(&tokens_sq);
        assert_eq!(info_sq.len(), 1);
        assert_eq!(info_sq[0].0, "String");
        assert_eq!(info_sq[0].1, "world");
    }

    // -----------------------------------------------------------------------
    // Test 6: AT_KEYWORD tokens (@mixin, @if, @include, etc.)
    // -----------------------------------------------------------------------

    /// At-keywords tokenize as AT_KEYWORD. The grammar (not the lexer)
    /// distinguishes @mixin from @if from @media — they all produce the same
    /// token type. The grammar matches on the token's text value.
    #[test]
    fn test_at_keyword_tokens() {
        let tokens = tokenize_lattice("@mixin @include @if @else @for @each @function @return @use");
        let types = type_names(&tokens);
        let vals = values(&tokens);

        // Every non-EOF token should be AT_KEYWORD
        for t in &types {
            assert_eq!(t, "AT_KEYWORD", "Expected AT_KEYWORD, got {t}");
        }

        // Check specific values
        assert!(vals.contains(&"@mixin"));
        assert!(vals.contains(&"@include"));
        assert!(vals.contains(&"@if"));
        assert!(vals.contains(&"@else"));
        assert!(vals.contains(&"@for"));
    }

    // -----------------------------------------------------------------------
    // Test 7: FUNCTION token (name followed by open paren)
    // -----------------------------------------------------------------------

    /// CSS function calls are tokenized as a single FUNCTION token that
    /// includes the opening parenthesis: "rgb(" not "rgb" + "(". This
    /// enables unambiguous parsing of identifiers vs function calls.
    #[test]
    fn test_function_token() {
        let tokens = tokenize_lattice("rgb(");
        let info = token_info(&tokens);

        // After `rgb(` there's nothing — just an RPAREN needed to close it,
        // but we're just testing the FUNCTION token here
        assert!(!info.is_empty());
        assert_eq!(info[0].0, "FUNCTION");
        assert_eq!(info[0].1, "rgb(");
    }

    // -----------------------------------------------------------------------
    // Test 8: Whitespace and comments are skipped
    // -----------------------------------------------------------------------

    /// Whitespace and comments should be consumed without producing tokens.
    /// Lattice supports both CSS block comments (/* */) and line comments (//).
    #[test]
    fn test_whitespace_and_comments_skipped() {
        // Line comment
        let tokens_line = tokenize_lattice("$x: 1; // this is a comment\n$y: 2;");
        let vals_line = values(&tokens_line);
        // Should have $x, :, 1, ;, $y, :, 2, ; — no comment token
        assert!(vals_line.contains(&"$x"));
        assert!(vals_line.contains(&"$y"));
        assert!(!vals_line.iter().any(|v| v.contains("comment")));

        // Block comment
        let tokens_block = tokenize_lattice("$a: 1; /* block comment */ $b: 2;");
        let vals_block = values(&tokens_block);
        assert!(vals_block.contains(&"$a"));
        assert!(vals_block.contains(&"$b"));
        assert!(!vals_block.iter().any(|v| v.contains("block")));

        // Whitespace between tokens
        let tokens_ws = tokenize_lattice("$x:1;");
        let tokens_spaced = tokenize_lattice("$x  :  1  ;");
        assert_eq!(values(&tokens_ws), values(&tokens_spaced),
            "Whitespace should not affect token values");
    }

    // -----------------------------------------------------------------------
    // Test 9: CSS structural tokens (braces, brackets, parens)
    // -----------------------------------------------------------------------

    /// The structural delimiters used in CSS selectors and blocks.
    #[test]
    fn test_structural_tokens() {
        let tokens = tokenize_lattice("{ } ( ) [ ] ; : , .");
        let info = token_info(&tokens);

        assert_eq!(info.len(), 10);
        assert_eq!(info[0].0, "LBrace");  // {
        assert_eq!(info[1].0, "RBrace");  // }
        assert_eq!(info[2].0, "LParen");  // (
        assert_eq!(info[3].0, "RParen");  // )
        assert_eq!(info[4].0, "LBracket"); // [
        assert_eq!(info[5].0, "RBracket"); // ]
        assert_eq!(info[6].0, "Semicolon"); // ;
        assert_eq!(info[7].0, "Colon");   // :
        assert_eq!(info[8].0, "Comma");   // ,
        assert_eq!(info[9].0, "Dot");     // .
    }

    // -----------------------------------------------------------------------
    // Test 10: IDENT token vs FUNCTION token disambiguation
    // -----------------------------------------------------------------------

    /// `color` is an IDENT (no parenthesis follows immediately in the token).
    /// `color(` is a FUNCTION (includes the parenthesis in the token).
    /// This distinction matters for CSS value parsing.
    #[test]
    fn test_ident_vs_function() {
        let tokens_ident = tokenize_lattice("color");
        let info_ident = token_info(&tokens_ident);
        assert_eq!(info_ident.len(), 1);
        // IDENT → string_to_token_type returns Name (fallback), and since
        // name ≠ "NAME", the lexer stores type_name = Some("IDENT").
        assert_eq!(info_ident[0].0, "IDENT");
        assert_eq!(info_ident[0].1, "color");

        let tokens_func = tokenize_lattice("color(");
        let info_func = token_info(&tokens_func);
        assert_eq!(info_func.len(), 1);
        assert_eq!(info_func[0].0, "FUNCTION");
        assert_eq!(info_func[0].1, "color(");
    }

    // -----------------------------------------------------------------------
    // Test 11: HASH token (hex colors and ID selectors)
    // -----------------------------------------------------------------------

    /// HASH matches `#` followed by an identifier-like string. This covers
    /// both hex color values (#4a90d9) and CSS ID selectors (#my-element).
    #[test]
    fn test_hash_token() {
        let tokens_hex = tokenize_lattice("#4a90d9");
        let info_hex = token_info(&tokens_hex);
        assert_eq!(info_hex.len(), 1);
        assert_eq!(info_hex[0].0, "HASH");
        assert_eq!(info_hex[0].1, "#4a90d9");

        let tokens_short = tokenize_lattice("#fff");
        let info_short = token_info(&tokens_short);
        assert_eq!(info_short.len(), 1);
        assert_eq!(info_short[0].0, "HASH");
        assert_eq!(info_short[0].1, "#fff");
    }

    // -----------------------------------------------------------------------
    // Test 12: Full simple rule
    // -----------------------------------------------------------------------

    /// A complete CSS qualified rule: `h1 { color: red; }`.
    /// Verifies the full token sequence.
    #[test]
    fn test_simple_css_rule() {
        let tokens = tokenize_lattice("h1 { color: red; }");
        let vals = values(&tokens);

        assert!(vals.contains(&"h1"));
        assert!(vals.contains(&"color"));
        assert!(vals.contains(&"red"));
    }

    // -----------------------------------------------------------------------
    // Test 13: Variable with hyphen in name
    // -----------------------------------------------------------------------

    /// Variable names can contain hyphens (like CSS custom properties):
    /// `$font-size`, `$primary-color`, etc.
    #[test]
    fn test_variable_with_hyphen() {
        let tokens = tokenize_lattice("$font-size");
        let info = token_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].0, "VARIABLE");
        assert_eq!(info[0].1, "$font-size");
    }

    // -----------------------------------------------------------------------
    // Test 14: @if expression with comparison
    // -----------------------------------------------------------------------

    /// A real-world @if condition: `@if $theme == dark`
    /// Tests the interaction of AT_KEYWORD, VARIABLE, IDENT, and EQUALS_EQUALS.
    #[test]
    fn test_if_expression() {
        let tokens = tokenize_lattice("@if $theme == dark");
        let info = token_info(&tokens);

        assert_eq!(info.len(), 4, "Expected @if $theme == dark = 4 tokens");
        assert_eq!(info[0].0, "AT_KEYWORD");
        assert_eq!(info[0].1, "@if");
        assert_eq!(info[1].0, "VARIABLE");
        assert_eq!(info[1].1, "$theme");
        assert_eq!(info[2].0, "EqualsEquals");
        assert_eq!(info[2].1, "==");
        assert_eq!(info[3].0, "IDENT");
        assert_eq!(info[3].1, "dark");
    }

    // -----------------------------------------------------------------------
    // Test 15: Empty source produces only EOF
    // -----------------------------------------------------------------------

    /// Tokenizing an empty string should return a single EOF token.
    #[test]
    fn test_empty_source() {
        let tokens = tokenize_lattice("");
        assert_eq!(tokens.len(), 1, "Empty source should produce only EOF");
        assert_eq!(tokens[0].type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 16: CUSTOM_PROPERTY token (CSS variables like --color)
    // -----------------------------------------------------------------------

    /// CSS custom properties start with `--`. These are distinct from Lattice
    /// variables (which start with `$`). Custom properties are valid in the
    /// `property` position of a declaration.
    #[test]
    fn test_custom_property() {
        let tokens = tokenize_lattice("--my-color");
        let info = token_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].0, "CUSTOM_PROPERTY");
        assert_eq!(info[0].1, "--my-color");
    }

    // -----------------------------------------------------------------------
    // Test 17: create_lattice_lexer factory
    // -----------------------------------------------------------------------

    /// The factory function should return a working lexer that can tokenize.
    #[test]
    fn test_create_lattice_lexer() {
        let mut lexer = create_lattice_lexer("$x: 42px;");
        let result = lexer.tokenize();
        assert!(result.is_ok(), "Lexer should succeed: {:?}", result.err());
        let tokens = result.unwrap();
        assert!(tokens.len() >= 4, "Should have at least 4 tokens + EOF");
    }

    // -----------------------------------------------------------------------
    // Test 18: COLON_COLON for pseudo-elements
    // -----------------------------------------------------------------------

    /// `::` is tokenized as COLON_COLON (not two COLON tokens) to support
    /// CSS pseudo-elements like `::before`, `::after`, `::first-line`.
    #[test]
    fn test_colon_colon() {
        let tokens = tokenize_lattice("::");
        let info = token_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].0, "COLON_COLON");
        assert_eq!(info[0].1, "::");
    }

    // -----------------------------------------------------------------------
    // Test 19: URL token
    // -----------------------------------------------------------------------

    /// `url(...)` without quotes is a single URL_TOKEN. This is distinct from
    /// `url("...")` which is FUNCTION + STRING + RPAREN.
    #[test]
    fn test_url_token() {
        let tokens = tokenize_lattice("url(image.png)");
        let info = token_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].0, "URL_TOKEN");
        assert_eq!(info[0].1, "url(image.png)");
    }

    // -----------------------------------------------------------------------
    // Test 20: Mixin definition tokens
    // -----------------------------------------------------------------------

    /// @mixin with a FUNCTION token for the name (name followed by `(`).
    /// `@mixin button(` produces AT_KEYWORD + FUNCTION.
    #[test]
    fn test_mixin_definition_tokens() {
        let tokens = tokenize_lattice("@mixin button(");
        let info = token_info(&tokens);

        assert_eq!(info.len(), 2);
        assert_eq!(info[0].0, "AT_KEYWORD");
        assert_eq!(info[0].1, "@mixin");
        assert_eq!(info[1].0, "FUNCTION");
        assert_eq!(info[1].1, "button(");
    }
}

