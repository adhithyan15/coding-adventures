//! Verilog lexer backed by compiled token grammar.

pub mod preprocessor;

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub const DEFAULT_VERSION: &str = "2005";
pub const SUPPORTED_VERSIONS: &[&str] = _grammar::SUPPORTED_VERSIONS;

fn validate_version(version: &str) -> Result<&str, String> {
    if SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown Verilog version '{version}'. Valid values: {}",
            SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_verilog_lexer(source: &str) -> GrammarLexer<'_> {
    create_verilog_lexer_with_version(source, DEFAULT_VERSION)
        .expect("compiled Verilog token grammar missing default version")
}

pub fn create_verilog_lexer_with_version<'src>(
    source: &'src str,
    version: &str,
) -> Result<GrammarLexer<'src>, String> {
    let version = validate_version(version)?;
    let grammar = _grammar::token_grammar(version)
        .expect("compiled Verilog token grammar missing supported version");
    Ok(GrammarLexer::new(source, &grammar))
}

pub fn tokenize_verilog(source: &str) -> Vec<Token> {
    tokenize_verilog_with_version(source, DEFAULT_VERSION)
        .expect("compiled Verilog token grammar missing default version")
}

pub fn tokenize_verilog_with_version(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let mut lexer = create_verilog_lexer_with_version(source, version)?;
    lexer
        .tokenize()
        .map_err(|e| format!("Verilog tokenization failed: {e}"))
}

pub fn tokenize_verilog_preprocessed(source: &str) -> Vec<Token> {
    let preprocessed = preprocessor::verilog_preprocess(source);
    tokenize_verilog(&preprocessed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper: collect token (type_, value) pairs excluding EOF.
    // -----------------------------------------------------------------------

    fn token_pairs(tokens: &[Token]) -> Vec<(TokenType, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: Simple module declaration
    // -----------------------------------------------------------------------

    /// Verify that a basic module declaration is tokenized correctly.
    #[test]
    fn test_tokenize_module_declaration() {
        let tokens = tokenize_verilog("module top; endmodule");
        let pairs = token_pairs(&tokens);

        assert!(pairs.len() >= 3, "Expected at least 3 tokens, got {}", pairs.len());
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "module");
        assert_eq!(pairs[1].0, TokenType::Name);
        assert_eq!(pairs[1].1, "top");
    }

    // -----------------------------------------------------------------------
    // Test 2: Keywords are recognized
    // -----------------------------------------------------------------------

    /// Verilog keywords should be classified as KEYWORD tokens, not NAME.
    #[test]
    fn test_keywords() {
        let keywords = [
            "module", "endmodule", "wire", "reg", "input", "output",
            "always", "assign", "begin", "end", "if", "else",
            "case", "endcase", "for", "parameter", "localparam",
        ];

        for kw in &keywords {
            let source = format!("{kw};");
            let tokens = tokenize_verilog(&source);
            let pairs = token_pairs(&tokens);

            assert_eq!(
                pairs[0].0, TokenType::Keyword,
                "Expected '{}' to be a KEYWORD, got {:?}",
                kw, pairs[0].0
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: Operators
    // -----------------------------------------------------------------------

    /// Arithmetic and bitwise operators should be tokenized correctly.
    #[test]
    fn test_operators() {
        let tokens = tokenize_verilog("a + b - c * d / e;");
        let pairs = token_pairs(&tokens);

        let ops: Vec<&str> = pairs
            .iter()
            .filter(|(_, v)| ["+", "-", "*", "/"].contains(v))
            .map(|(_, v)| *v)
            .collect();

        assert_eq!(ops, vec!["+", "-", "*", "/"]);
    }

    // -----------------------------------------------------------------------
    // Test 4: Multi-character operators
    // -----------------------------------------------------------------------

    /// Multi-character operators like ===, !==, <<<, >>> should be tokenized
    /// as single tokens.
    #[test]
    fn test_multi_char_operators() {
        let tokens = tokenize_verilog("a === b !== c;");
        let pairs = token_pairs(&tokens);

        let has_case_eq = pairs.iter().any(|(_, v)| *v == "===");
        let has_case_neq = pairs.iter().any(|(_, v)| *v == "!==");

        assert!(has_case_eq, "Expected '===' token");
        assert!(has_case_neq, "Expected '!==' token");
    }

    #[test]
    fn test_shift_operators() {
        let tokens = tokenize_verilog("a << b >> c <<< d >>> e;");
        let pairs = token_pairs(&tokens);

        let has_lshift = pairs.iter().any(|(_, v)| *v == "<<");
        let has_rshift = pairs.iter().any(|(_, v)| *v == ">>");
        let has_alshift = pairs.iter().any(|(_, v)| *v == "<<<");
        let has_arshift = pairs.iter().any(|(_, v)| *v == ">>>");

        assert!(has_lshift, "Expected '<<' token");
        assert!(has_rshift, "Expected '>>' token");
        assert!(has_alshift, "Expected '<<<' token");
        assert!(has_arshift, "Expected '>>>' token");
    }

    // -----------------------------------------------------------------------
    // Test 5: String literals
    // -----------------------------------------------------------------------

    /// Verilog supports double-quoted strings.
    #[test]
    fn test_strings() {
        let tokens = tokenize_verilog("$display(\"hello world\");");
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token");
    }

    // -----------------------------------------------------------------------
    // Test 6: Number literals
    // -----------------------------------------------------------------------

    /// Verilog supports plain integers.
    #[test]
    fn test_plain_numbers() {
        let tokens = tokenize_verilog("42;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "42");
    }

    /// Verilog sized numbers like 4'b1010 and 8'hFF.
    #[test]
    fn test_sized_numbers() {
        let tokens = tokenize_verilog("4'b1010;");
        let pairs = token_pairs(&tokens);

        // The sized number should be a single token
        assert_eq!(pairs[0].1, "4'b1010");
    }

    #[test]
    fn test_hex_sized_numbers() {
        let tokens = tokenize_verilog("8'hFF;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].1, "8'hFF");
    }

    #[test]
    fn test_real_numbers() {
        let tokens = tokenize_verilog("3.14;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].1, "3.14");
    }

    // -----------------------------------------------------------------------
    // Test 7: System identifiers
    // -----------------------------------------------------------------------

    /// System tasks like $display and $finish should be tokenized.
    #[test]
    fn test_system_identifiers() {
        let tokens = tokenize_verilog("$display;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].1, "$display");
    }

    // -----------------------------------------------------------------------
    // Test 8: Compiler directives
    // -----------------------------------------------------------------------

    /// Directives like `define should be tokenized as DIRECTIVE tokens
    /// when preprocessing is NOT applied.
    #[test]
    fn test_directives() {
        let tokens = tokenize_verilog("`define WIDTH 8");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].1, "`define");
    }

    // -----------------------------------------------------------------------
    // Test 9: Delimiters
    // -----------------------------------------------------------------------

    /// All delimiter tokens should be recognized.
    #[test]
    fn test_delimiters() {
        let tokens = tokenize_verilog("(){}[];,.#@");
        let pairs = token_pairs(&tokens);

        let values: Vec<&str> = pairs.iter().map(|(_, v)| *v).collect();
        assert!(values.contains(&"("));
        assert!(values.contains(&")"));
        assert!(values.contains(&"{"));
        assert!(values.contains(&"}"));
        assert!(values.contains(&"["));
        assert!(values.contains(&"]"));
        assert!(values.contains(&";"));
        assert!(values.contains(&","));
        assert!(values.contains(&"."));
        assert!(values.contains(&"#"));
        assert!(values.contains(&"@"));
    }

    // -----------------------------------------------------------------------
    // Test 10: Comments are skipped
    // -----------------------------------------------------------------------

    /// Single-line and block comments should be skipped.
    #[test]
    fn test_comments_skipped() {
        let tokens = tokenize_verilog("wire a; // comment\nwire b;");
        let pairs = token_pairs(&tokens);

        // "comment" should NOT appear as a token
        let has_comment_text = pairs.iter().any(|(_, v)| v.contains("comment"));
        assert!(!has_comment_text, "Comments should be skipped");
    }

    #[test]
    fn test_block_comments_skipped() {
        let tokens = tokenize_verilog("wire a; /* block comment */ wire b;");
        let pairs = token_pairs(&tokens);

        let has_comment_text = pairs.iter().any(|(_, v)| v.contains("block"));
        assert!(!has_comment_text, "Block comments should be skipped");
    }

    // -----------------------------------------------------------------------
    // Test 11: Whitespace is skipped
    // -----------------------------------------------------------------------

    /// Whitespace between tokens should be consumed without producing tokens.
    #[test]
    fn test_whitespace_skipped() {
        let compact = tokenize_verilog("wire a;");
        let spaced = tokenize_verilog("wire  a  ;");

        let pairs_compact = token_pairs(&compact);
        let pairs_spaced = token_pairs(&spaced);

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
    }

    // -----------------------------------------------------------------------
    // Test 12: Factory function
    // -----------------------------------------------------------------------

    /// The `create_verilog_lexer` factory function should return a
    /// working `GrammarLexer`.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_verilog_lexer("42;");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 13: Module with ports
    // -----------------------------------------------------------------------

    /// A module declaration with ports exercises keywords, identifiers,
    /// parentheses, and commas.
    #[test]
    fn test_module_with_ports() {
        let tokens = tokenize_verilog("module adder(input a, input b, output sum);");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "module");
        assert_eq!(pairs[1].0, TokenType::Name);
        assert_eq!(pairs[1].1, "adder");
    }

    // -----------------------------------------------------------------------
    // Test 14: Sensitivity list with @
    // -----------------------------------------------------------------------

    /// The @ token and sensitivity keywords (posedge, negedge) should work.
    #[test]
    fn test_sensitivity_list() {
        let tokens = tokenize_verilog("always @(posedge clk) begin end");
        let pairs = token_pairs(&tokens);

        let has_at = pairs.iter().any(|(_, v)| *v == "@");
        let has_posedge = pairs.iter().any(|(_, v)| *v == "posedge");
        assert!(has_at, "Expected '@' token");
        assert!(has_posedge, "Expected 'posedge' keyword");
    }

    // -----------------------------------------------------------------------
    // Test 15: Assign statement
    // -----------------------------------------------------------------------

    /// Continuous assignment with the assign keyword.
    #[test]
    fn test_assign_statement() {
        let tokens = tokenize_verilog("assign out = a & b;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "assign");
    }

    // -----------------------------------------------------------------------
    // Test 16: Preprocessed tokenization
    // -----------------------------------------------------------------------

    /// Tokenize with preprocessing — macros should be expanded.
    #[test]
    fn test_preprocessed_tokenization() {
        let source = "`define WIDTH 8\nreg [`WIDTH-1:0] data;";
        let tokens = tokenize_verilog_preprocessed(source);
        let pairs = token_pairs(&tokens);

        // After preprocessing, `WIDTH becomes 8
        // We should see NUMBER(8), not DIRECTIVE(`WIDTH)
        let has_eight = pairs.iter().any(|(t, v)| *t == TokenType::Number && *v == "8");
        assert!(has_eight, "Expected NUMBER(8) after preprocessing");

        // No DIRECTIVE tokens should remain for `WIDTH
        let has_width_directive = pairs.iter().any(|(_, v)| *v == "`WIDTH");
        assert!(!has_width_directive, "Preprocessor should have expanded `WIDTH");
    }

    // -----------------------------------------------------------------------
    // Test 17: Conditional preprocessing + tokenization
    // -----------------------------------------------------------------------

    #[test]
    fn test_preprocessed_ifdef() {
        let source = "\
`define USE_FAST
`ifdef USE_FAST
wire fast_path;
`else
wire slow_path;
`endif";
        let tokens = tokenize_verilog_preprocessed(source);
        let pairs = token_pairs(&tokens);

        let has_fast = pairs.iter().any(|(_, v)| *v == "fast_path");
        let has_slow = pairs.iter().any(|(_, v)| *v == "slow_path");
        assert!(has_fast, "Expected fast_path in output");
        assert!(!has_slow, "slow_path should be excluded by ifdef");
    }

    // -----------------------------------------------------------------------
    // Test 18: Bitwise operators
    // -----------------------------------------------------------------------

    #[test]
    fn test_bitwise_operators() {
        let tokens = tokenize_verilog("a & b | c ^ d;");
        let pairs = token_pairs(&tokens);

        let ops: Vec<&str> = pairs
            .iter()
            .filter(|(_, v)| ["&", "|", "^"].contains(v))
            .map(|(_, v)| *v)
            .collect();

        assert_eq!(ops, vec!["&", "|", "^"]);
    }

    // -----------------------------------------------------------------------
    // Test 19: Logical operators
    // -----------------------------------------------------------------------

    #[test]
    fn test_logical_operators() {
        let tokens = tokenize_verilog("a && b || c;");
        let pairs = token_pairs(&tokens);

        let has_land = pairs.iter().any(|(_, v)| *v == "&&");
        let has_lor = pairs.iter().any(|(_, v)| *v == "||");

        assert!(has_land, "Expected '&&' token");
        assert!(has_lor, "Expected '||' token");
    }

    // -----------------------------------------------------------------------
    // Test 20: Comparison operators
    // -----------------------------------------------------------------------

    #[test]
    fn test_comparison_operators() {
        let tokens = tokenize_verilog("a == b != c <= d >= e;");
        let pairs = token_pairs(&tokens);

        let has_eq = pairs.iter().any(|(_, v)| *v == "==");
        let has_neq = pairs.iter().any(|(_, v)| *v == "!=");
        let has_leq = pairs.iter().any(|(_, v)| *v == "<=");
        let has_geq = pairs.iter().any(|(_, v)| *v == ">=");

        assert!(has_eq, "Expected '==' token");
        assert!(has_neq, "Expected '!=' token");
        assert!(has_leq, "Expected '<=' token");
        assert!(has_geq, "Expected '>=' token");
    }

    // -----------------------------------------------------------------------
    // Test 21: Unary operators
    // -----------------------------------------------------------------------

    #[test]
    fn test_unary_operators() {
        let tokens = tokenize_verilog("~a !b;");
        let pairs = token_pairs(&tokens);

        let has_tilde = pairs.iter().any(|(_, v)| *v == "~");
        let has_bang = pairs.iter().any(|(_, v)| *v == "!");

        assert!(has_tilde, "Expected '~' token");
        assert!(has_bang, "Expected '!' token");
    }

    // -----------------------------------------------------------------------
    // Test 22: Ternary operator
    // -----------------------------------------------------------------------

    #[test]
    fn test_ternary_operator() {
        let tokens = tokenize_verilog("assign out = sel ? a : b;");
        let pairs = token_pairs(&tokens);

        let has_question = pairs.iter().any(|(_, v)| *v == "?");
        let has_colon = pairs.iter().any(|(_, v)| *v == ":");

        assert!(has_question, "Expected '?' token");
        assert!(has_colon, "Expected ':' token");
    }

    // -----------------------------------------------------------------------
    // Test 23: Full module example
    // -----------------------------------------------------------------------

    /// A complete small module should tokenize without errors.
    #[test]
    fn test_full_module() {
        let source = r#"
module mux2to1(
    input a,
    input b,
    input sel,
    output reg out
);
    always @(*) begin
        if (sel)
            out = b;
        else
            out = a;
    end
endmodule
"#;
        let tokens = tokenize_verilog(source);
        let pairs = token_pairs(&tokens);

        // Should have many tokens without errors
        assert!(pairs.len() > 20, "Expected many tokens from full module");

        // Check key structural tokens
        let first_kw = pairs.iter().find(|(t, _)| *t == TokenType::Keyword);
        assert_eq!(first_kw.unwrap().1, "module");
    }

    // -----------------------------------------------------------------------
    // Test 24: Escaped identifier
    // -----------------------------------------------------------------------

    #[test]
    fn test_escaped_identifier() {
        let tokens = tokenize_verilog("wire \\my.signal ;");
        let pairs = token_pairs(&tokens);

        let has_escaped = pairs.iter().any(|(_, v)| v.starts_with('\\'));
        assert!(has_escaped, "Expected an escaped identifier token");
    }

    // -----------------------------------------------------------------------
    // Test 25: Power operator
    // -----------------------------------------------------------------------

    #[test]
    fn test_power_operator() {
        let tokens = tokenize_verilog("2 ** 3;");
        let pairs = token_pairs(&tokens);

        let has_power = pairs.iter().any(|(_, v)| *v == "**");
        assert!(has_power, "Expected '**' token");
    }

    // -----------------------------------------------------------------------
    // Test 26: Trigger operator
    // -----------------------------------------------------------------------

    #[test]
    fn test_trigger_operator() {
        let tokens = tokenize_verilog("-> event1;");
        let pairs = token_pairs(&tokens);

        let has_trigger = pairs.iter().any(|(_, v)| *v == "->");
        assert!(has_trigger, "Expected '->' token");
    }

    #[test]
    fn test_default_version_matches_explicit_2005() {
        let default_tokens = tokenize_verilog("module top; endmodule");
        let explicit_tokens =
            tokenize_verilog_with_version("module top; endmodule", "2005").unwrap();
        assert_eq!(default_tokens.len(), explicit_tokens.len());
    }

    #[test]
    fn test_unknown_version_rejected() {
        let err = tokenize_verilog_with_version("module top; endmodule", "2099")
            .expect_err("unknown versions should be rejected");
        assert!(err.contains("Unknown Verilog version"));
    }
}

