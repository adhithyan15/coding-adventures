//! # VHDL Lexer — tokenizing VHDL (IEEE 1076-2008) source code.
//!
//! [VHDL](https://en.wikipedia.org/wiki/VHDL) (VHSIC Hardware Description
//! Language) is a strongly-typed, verbose HDL designed by the US Department of
//! Defense. Unlike Verilog, which is terse and C-like, VHDL is Ada-like — it
//! favors explicit declarations, strong typing, and readability over
//! conciseness.
//!
//! This crate provides a lexer (tokenizer) for VHDL. It loads the
//! `vhdl.tokens` grammar file — a declarative description of every token in
//! VHDL — and feeds it to the generic [`GrammarLexer`] from the `lexer` crate.
//!
//! # No preprocessor
//!
//! Unlike Verilog (which has a C-like preprocessor with `` `define ``,
//! `` `ifdef ``, etc.), VHDL has **no preprocessor**. What you write is what
//! the compiler sees. Configurations and generics serve the role that macros
//! and conditional compilation play in Verilog.
//!
//! # Case insensitivity
//!
//! VHDL is case-insensitive: `ENTITY`, `Entity`, and `entity` all refer to
//! the same keyword. After tokenization, this crate normalizes all NAME and
//! KEYWORD token values to lowercase using [`str::to_lowercase()`]. This
//! ensures consistent downstream processing — parsers and tools only need
//! to match against lowercase strings.
//!
//! Extended identifiers (delimited by backslashes, like `\My Name\`) are
//! NOT normalized. The VHDL standard specifies that extended identifiers
//! preserve their case.
//!
//! # Architecture
//!
//! ```text
//! vhdl.tokens          (grammar file on disk)
//!        |
//!        v
//! grammar-tools        (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer  (tokenizes source using TokenGrammar)
//!        |
//!        v
//! vhdl-lexer           (THIS CRATE: wires grammar + lexer + case normalization)
//! ```
//!
//! # Token types
//!
//! The VHDL lexer produces these token categories:
//!
//! - **NAME** — identifiers like `clk`, `data_in`, `std_logic` (lowercased)
//! - **KEYWORD** — reserved words: `entity`, `architecture`, `signal`, etc. (lowercased)
//! - **NUMBER** — plain integers: `42`, `0`, `1_000_000`
//! - **REAL_NUMBER** — floating-point: `3.14`, `1.0E-3`
//! - **BASED_LITERAL** — based numbers: `16#FF#`, `2#1010#`
//! - **BIT_STRING** — bit string literals: `B"1010"`, `X"FF"`, `O"77"`
//! - **CHAR_LITERAL** — character literals: `'0'`, `'1'`, `'X'`, `'Z'`
//! - **STRING** — string literals: `"hello"`, `"He said ""hi"""`
//! - **EXTENDED_IDENT** — extended identifiers: `\my name\`, `\VHDL-2008\`
//! - **Operators** — `:=`, `<=`, `=>`, `/=`, `**`, `<>`, `+`, `-`, `*`, `/`
//! - **Delimiters** — `(`, `)`, `[`, `]`, `;`, `,`, `.`, `:`
//! - **EOF** — end of file

use std::collections::HashSet;
use std::fs;

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `vhdl.tokens` grammar file.
///
/// We use `env!("CARGO_MANIFEST_DIR")` to get the directory containing this
/// crate's `Cargo.toml` at compile time. From there, we navigate up to the
/// `grammars/` directory at the repository root.
///
/// ```text
/// code/
///   grammars/
///     vhdl.tokens           <-- this is what we want
///   packages/
///     rust/
///       vhdl-lexer/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR points here
///         src/
///           lib.rs          <-- we are here
/// ```
///
/// So the relative path from CARGO_MANIFEST_DIR to the grammar file is:
/// `../../../grammars/vhdl.tokens`
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/vhdl.tokens")
}

// ===========================================================================
// Case normalization
// ===========================================================================

/// Normalize token values for VHDL's case-insensitive semantics.
///
/// VHDL treats identifiers and keywords as case-insensitive. The standard
/// says that `ENTITY`, `Entity`, and `entity` are all the same thing.
/// We implement this by lowercasing NAME and KEYWORD token values after
/// tokenization.
///
/// ## Why this is tricky
///
/// The grammar-driven lexer maps many VHDL token types (CHAR_LITERAL,
/// BIT_STRING, BASED_LITERAL, EXTENDED_IDENT, REAL_NUMBER, etc.) to the
/// generic `TokenType::Name` because the `TokenType` enum doesn't have
/// dedicated variants for them. To distinguish "real" identifiers from
/// these other token types, we check the `type_name` field:
///
/// - `type_name == Some("NAME")` — a real identifier, must be lowercased
/// - `type_name == Some("CHAR_LITERAL")` — character literal, preserve case
/// - `type_name == Some("BIT_STRING")` — bit string, preserve case
/// - `type_name == Some("EXTENDED_IDENT")` — extended identifier, preserve case
/// - etc.
///
/// ## Keyword promotion
///
/// Because the grammar's keyword list is lowercase ("entity", "signal", etc.),
/// the lexer fails to promote uppercase identifiers like "ENTITY" to keywords.
/// After lowercasing NAME tokens, we re-check them against the keyword set and
/// promote matches to `TokenType::Keyword`.
///
/// ## What gets normalized and what doesn't
///
/// | Token type      | type_name        | Normalized? | Reason                          |
/// |-----------------|------------------|-------------|---------------------------------|
/// | Identifier      | NAME             | Yes         | VHDL is case-insensitive        |
/// | Keyword         | (any)            | Yes         | Keywords are identifiers too    |
/// | String          | STRING           | No          | String content is user data     |
/// | Character lit   | CHAR_LITERAL     | No          | 'A' and 'a' are different chars |
/// | Bit string      | BIT_STRING       | No          | X"FF" is a numeric literal      |
/// | Based literal   | BASED_LITERAL    | No          | 16#FF# is a numeric literal     |
/// | Extended ident  | EXTENDED_IDENT   | No          | VHDL standard says preserve case|
/// | Real number     | REAL_NUMBER      | No          | 1.0E-3 is a numeric literal     |
/// | Operators       | (various)        | No          | No alphabetic content           |
fn normalize_case(tokens: &mut [Token], keywords: &HashSet<String>) {
    for token in tokens.iter_mut() {
        // Step 1: Determine if this token should be case-normalized.
        //
        // We only normalize tokens that represent VHDL identifiers — those
        // with type_name "NAME" — and existing keywords. All other token
        // types (CHAR_LITERAL, BIT_STRING, BASED_LITERAL, EXTENDED_IDENT,
        // REAL_NUMBER, STRING, operators, delimiters) keep their original case.
        let should_normalize = match token.type_ {
            // Existing keywords always get normalized.
            TokenType::Keyword => true,

            // For Name tokens, check the type_name to distinguish real
            // identifiers from other token types that happen to use
            // TokenType::Name as a fallback.
            TokenType::Name => {
                match token.type_name.as_deref() {
                    // Explicit NAME — this is a real identifier.
                    Some("NAME") => true,
                    // No type_name — treat as identifier (shouldn't happen
                    // with grammar-driven lexer, but be safe).
                    None => true,
                    // Anything else (CHAR_LITERAL, BIT_STRING, BASED_LITERAL,
                    // EXTENDED_IDENT, REAL_NUMBER, operators like VAR_ASSIGN,
                    // ARROW, etc.) — do NOT normalize.
                    Some(_) => false,
                }
            }

            // All other TokenType variants (Number, String, Plus, etc.)
            // need no normalization.
            _ => false,
        };

        if should_normalize {
            // Step 2: Lowercase the value.
            token.value = token.value.to_lowercase();

            // Step 3: Keyword promotion.
            //
            // If this was a NAME token and its lowercased value matches
            // a keyword, promote it to Keyword type. This handles the case
            // where the source writes "ENTITY" — the grammar's keyword list
            // has "entity", so the initial tokenization produces a NAME.
            // After lowercasing, we can now match and promote.
            if token.type_ == TokenType::Name && keywords.contains(&token.value) {
                token.type_ = TokenType::Keyword;
            }
        }
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for VHDL source code.
///
/// This function:
/// 1. Reads the `vhdl.tokens` grammar file from disk.
/// 2. Parses it into a `TokenGrammar` using `grammar-tools`.
/// 3. Constructs a `GrammarLexer` with the grammar and the given source.
///
/// The returned lexer is ready to call `.tokenize()` on.
///
/// **Note:** This function does NOT apply case normalization. If you want
/// lowercased NAME/KEYWORD values (which is the VHDL-correct behavior),
/// use [`tokenize_vhdl`] instead, or manually call [`normalize_case`]
/// on the output.
///
/// # Panics
///
/// Panics if the grammar file cannot be read or parsed.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_vhdl_lexer::create_vhdl_lexer;
///
/// let mut lexer = create_vhdl_lexer("entity top is end entity top;");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{}", token);
/// }
/// ```
pub fn create_vhdl_lexer(source: &str) -> GrammarLexer<'_> {
    // Step 1: Read the grammar file from disk.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read vhdl.tokens: {e}"));

    // Step 2: Parse the grammar text into a structured TokenGrammar.
    //
    // The TokenGrammar contains:
    //   - Token definitions (NAME, NUMBER, BASED_LITERAL, REAL_NUMBER,
    //     BIT_STRING, CHAR_LITERAL, STRING, EXTENDED_IDENT, operators,
    //     delimiters)
    //   - Skip patterns (whitespace, single-line comments with --)
    //   - Keywords (entity, architecture, signal, process, etc.)
    //   - Mode: default (no indentation tracking)
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse vhdl.tokens: {e}"));

    // Step 3: Create and return the lexer.
    GrammarLexer::new(source, &grammar)
}

/// Tokenize VHDL source code into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, tokenization, and case normalization in one call. The
/// returned vector always ends with an `EOF` token.
///
/// All NAME and KEYWORD token values are lowercased to implement VHDL's
/// case-insensitive semantics. For example:
///
/// ```text
/// Input:   "ENTITY Top IS END ENTITY Top;"
/// Output:  KEYWORD("entity") NAME("top") KEYWORD("is") KEYWORD("end")
///          KEYWORD("entity") NAME("top") ...
/// ```
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed, or if the source
/// contains an unexpected character.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_vhdl_lexer::tokenize_vhdl;
///
/// let tokens = tokenize_vhdl("SIGNAL clk : STD_LOGIC;");
/// for token in &tokens {
///     println!("{:?} {:?}", token.type_, token.value);
/// }
/// // Output:
/// //   Keyword "signal"      <-- lowercased
/// //   Name "clk"
/// //   ...
/// //   Name "std_logic"      <-- lowercased
/// ```
pub fn tokenize_vhdl(source: &str) -> Vec<Token> {
    // Step 1: Read and parse the grammar file.
    //
    // We need the grammar both for creating the lexer AND for building
    // the keyword set used in case normalization. So we parse it here
    // rather than delegating to create_vhdl_lexer.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read vhdl.tokens: {e}"));
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse vhdl.tokens: {e}"));

    // Step 2: Build the keyword set for case-insensitive promotion.
    //
    // The grammar's keyword list is lowercase ("entity", "signal", etc.).
    // We'll use this set to promote NAME tokens whose lowercased value
    // matches a keyword.
    let keyword_set: HashSet<String> = grammar.keywords.iter().cloned().collect();

    // Step 3: Create the lexer and tokenize.
    let mut vhdl_lexer = GrammarLexer::new(source, &grammar);
    let mut tokens = vhdl_lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("VHDL tokenization failed: {e}"));

    // Step 4: Post-tokenize case normalization.
    //
    // This is the key difference from the Verilog lexer. Verilog is
    // case-sensitive (wire != Wire != WIRE), but VHDL is not. We lowercase
    // all NAME and KEYWORD values, then re-check for keyword promotion.
    normalize_case(&mut tokens, &keyword_set);

    tokens
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helper: collect token (type_, value) pairs excluding EOF.
    // -----------------------------------------------------------------------

    /// Strip EOF tokens to focus on the meaningful content.
    /// Returns a vector of (TokenType, value) pairs for easy assertion.
    fn token_pairs(tokens: &[Token]) -> Vec<(TokenType, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect()
    }

    // =======================================================================
    // Entity and architecture declarations
    // =======================================================================

    /// The most basic VHDL construct: an empty entity declaration.
    ///
    /// ```vhdl
    /// entity top is
    /// end entity top;
    /// ```
    ///
    /// An entity in VHDL is analogous to a module declaration in Verilog —
    /// it defines the external interface (ports) of a hardware block.
    #[test]
    fn test_entity_declaration() {
        let tokens = tokenize_vhdl("entity top is end entity top;");
        let pairs = token_pairs(&tokens);

        assert!(pairs.len() >= 5, "Expected at least 5 tokens, got {}", pairs.len());
        assert_eq!(pairs[0], (TokenType::Keyword, "entity"));
        assert_eq!(pairs[1], (TokenType::Name, "top"));
        assert_eq!(pairs[2], (TokenType::Keyword, "is"));
        assert_eq!(pairs[3], (TokenType::Keyword, "end"));
        assert_eq!(pairs[4], (TokenType::Keyword, "entity"));
        assert_eq!(pairs[5], (TokenType::Name, "top"));
    }

    /// An architecture body — the implementation of an entity.
    ///
    /// ```vhdl
    /// architecture rtl of top is
    /// begin
    /// end architecture rtl;
    /// ```
    ///
    /// "rtl" stands for Register Transfer Level, the most common
    /// architecture style in synthesizable VHDL.
    #[test]
    fn test_architecture_declaration() {
        let tokens = tokenize_vhdl("architecture rtl of top is begin end architecture rtl;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0], (TokenType::Keyword, "architecture"));
        assert_eq!(pairs[1], (TokenType::Name, "rtl"));
        assert_eq!(pairs[2], (TokenType::Keyword, "of"));
        assert_eq!(pairs[3], (TokenType::Name, "top"));
        assert_eq!(pairs[4], (TokenType::Keyword, "is"));
        assert_eq!(pairs[5], (TokenType::Keyword, "begin"));
    }

    // =======================================================================
    // Signal declarations
    // =======================================================================

    /// Signals are the "wires" of VHDL — they connect components and carry
    /// values between concurrent processes.
    ///
    /// ```vhdl
    /// signal clk : std_logic;
    /// signal data : std_logic_vector(7 downto 0);
    /// ```
    #[test]
    fn test_signal_declaration() {
        let tokens = tokenize_vhdl("signal clk : std_logic;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0], (TokenType::Keyword, "signal"));
        assert_eq!(pairs[1], (TokenType::Name, "clk"));
        // ':' is a COLON delimiter
        assert_eq!(pairs[2].1, ":");
        assert_eq!(pairs[3], (TokenType::Name, "std_logic"));
    }

    #[test]
    fn test_signal_with_vector_type() {
        let tokens = tokenize_vhdl("signal data : std_logic_vector(7 downto 0);");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0], (TokenType::Keyword, "signal"));
        assert_eq!(pairs[1], (TokenType::Name, "data"));

        // Verify "downto" is a keyword
        let has_downto = pairs.iter().any(|(t, v)| *t == TokenType::Keyword && *v == "downto");
        assert!(has_downto, "Expected 'downto' keyword");
    }

    // =======================================================================
    // Case insensitivity normalization
    // =======================================================================

    /// VHDL is case-insensitive. ENTITY, Entity, and entity must all
    /// produce the same token value ("entity").
    ///
    /// This is the KEY DIFFERENCE from the Verilog lexer. In Verilog,
    /// `Wire` and `wire` are different identifiers. In VHDL, they are
    /// the same.
    #[test]
    fn test_case_insensitive_keywords() {
        let upper = tokenize_vhdl("ENTITY TOP IS END ENTITY TOP;");
        let lower = tokenize_vhdl("entity top is end entity top;");
        let mixed = tokenize_vhdl("Entity Top Is End Entity Top;");

        let upper_pairs = token_pairs(&upper);
        let lower_pairs = token_pairs(&lower);
        let mixed_pairs = token_pairs(&mixed);

        // All three should produce identical token values
        assert_eq!(upper_pairs.len(), lower_pairs.len());
        assert_eq!(upper_pairs.len(), mixed_pairs.len());

        for i in 0..upper_pairs.len() {
            assert_eq!(
                upper_pairs[i], lower_pairs[i],
                "Token {} differs between UPPER and lower: {:?} vs {:?}",
                i, upper_pairs[i], lower_pairs[i]
            );
            assert_eq!(
                upper_pairs[i], mixed_pairs[i],
                "Token {} differs between UPPER and Mixed: {:?} vs {:?}",
                i, upper_pairs[i], mixed_pairs[i]
            );
        }
    }

    /// Identifiers (NAME tokens) should also be lowercased.
    #[test]
    fn test_case_insensitive_identifiers() {
        let tokens = tokenize_vhdl("signal MySignal : STD_LOGIC;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[1], (TokenType::Name, "mysignal"));
        assert_eq!(pairs[3], (TokenType::Name, "std_logic"));
    }

    /// Extended identifiers (backslash-delimited) must NOT be normalized.
    /// The VHDL standard says \My Signal\ and \my signal\ are different.
    #[test]
    fn test_extended_identifiers_preserve_case() {
        let tokens = tokenize_vhdl(r"\My Signal\");
        let pairs = token_pairs(&tokens);

        // The extended identifier should keep its original case
        assert_eq!(pairs[0].1, r"\My Signal\");
    }

    // =======================================================================
    // Character literals
    // =======================================================================

    /// Character literals are single characters between tick marks.
    /// These are the values of std_logic, VHDL's most important type:
    ///   '0' — logic low     '1' — logic high
    ///   'X' — unknown       'Z' — high impedance
    ///   'U' — uninitialized '-' — don't care
    #[test]
    fn test_character_literals() {
        let inputs   = ["'0'", "'1'", "'X'", "'Z'", "'U'", "'-'"];
        let expected = ["'0'", "'1'", "'X'", "'Z'", "'U'", "'-'"];
        for (input, exp) in inputs.iter().zip(expected.iter()) {
            let tokens = tokenize_vhdl(&format!("{input};"));
            let pairs = token_pairs(&tokens);

            assert_eq!(
                pairs[0].1, *exp,
                "Character literal {} should preserve its original spelling as {}",
                input, exp
            );
        }
    }

    // =======================================================================
    // Bit string literals
    // =======================================================================

    /// Bit strings are the VHDL equivalent of Verilog's sized literals.
    ///   Verilog: 4'b1010  ->  VHDL: B"1010"
    ///   Verilog: 8'hFF    ->  VHDL: X"FF"
    #[test]
    fn test_bit_string_literals() {
        // Binary bit string
        let tokens = tokenize_vhdl("B\"1010\";");
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs[0].1, "B\"1010\"");

        // Hex bit string
        let tokens = tokenize_vhdl("X\"FF\";");
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs[0].1, "X\"FF\"");

        // Octal bit string
        let tokens = tokenize_vhdl("O\"77\";");
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs[0].1, "O\"77\"");
    }

    /// Bit string prefixes are case-insensitive.
    #[test]
    fn test_bit_string_case_insensitive_prefix() {
        let lower = tokenize_vhdl("x\"FF\";");
        let upper = tokenize_vhdl("X\"FF\";");
        let lower_pairs = token_pairs(&lower);
        let upper_pairs = token_pairs(&upper);

        // Both should produce a bit string token (though the case of the
        // prefix itself may differ — the grammar matches both)
        assert_eq!(lower_pairs[0].0, upper_pairs[0].0);
    }

    // =======================================================================
    // Based literals
    // =======================================================================

    /// Based literals use the format: base#digits#
    ///   16#FF#   — hex 255
    ///   2#1010#  — binary 10
    ///   8#77#    — octal 63
    #[test]
    fn test_based_literals() {
        let tokens = tokenize_vhdl("16#FF#;");
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs[0].1, "16#FF#");

        let tokens = tokenize_vhdl("2#1010#;");
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs[0].1, "2#1010#");

        let tokens = tokenize_vhdl("8#77#;");
        let pairs = token_pairs(&tokens);
        assert_eq!(pairs[0].1, "8#77#");
    }

    // =======================================================================
    // All operators
    // =======================================================================

    /// Two-character operators in VHDL. Each has a specific meaning:
    ///
    /// | Operator | Name           | Meaning                           |
    /// |----------|----------------|-----------------------------------|
    /// | :=       | VAR_ASSIGN     | Variable assignment               |
    /// | <=       | LESS_EQUALS    | Signal assignment OR less-or-equal|
    /// | =>       | ARROW          | Port map association              |
    /// | /=       | NOT_EQUALS     | Not equal (cf. C's !=)            |
    /// | **       | POWER          | Exponentiation                    |
    /// | <>       | BOX            | Unconstrained range               |
    /// | >=       | GREATER_EQUALS | Greater than or equal             |
    #[test]
    fn test_two_char_operators() {
        let test_cases = [
            (":=", ":="),
            ("<=", "<="),
            ("=>", "=>"),
            ("/=", "/="),
            ("**", "**"),
            ("<>", "<>"),
            (">=", ">="),
        ];

        for (input, expected) in &test_cases {
            let source = format!("a {input} b;");
            let tokens = tokenize_vhdl(&source);
            let pairs = token_pairs(&tokens);

            let has_op = pairs.iter().any(|(_, v)| v == expected);
            assert!(has_op, "Expected '{}' operator in '{}'", expected, input);
        }
    }

    /// Single-character operators.
    #[test]
    fn test_single_char_operators() {
        let tokens = tokenize_vhdl("a + b - c * d / e;");
        let pairs = token_pairs(&tokens);

        let ops: Vec<&str> = pairs
            .iter()
            .filter(|(_, v)| ["+", "-", "*", "/"].contains(v))
            .map(|(_, v)| *v)
            .collect();

        assert_eq!(ops, vec!["+", "-", "*", "/"]);
    }

    /// The ampersand (&) in VHDL is concatenation, not bitwise AND.
    /// Bitwise AND is the `and` keyword.
    #[test]
    fn test_concatenation_operator() {
        let tokens = tokenize_vhdl("a & b;");
        let pairs = token_pairs(&tokens);

        let has_amp = pairs.iter().any(|(_, v)| *v == "&");
        assert!(has_amp, "Expected '&' (concatenation) operator");
    }

    // =======================================================================
    // Keyword operators
    // =======================================================================

    /// VHDL uses keyword operators instead of symbols for logical operations.
    /// This is part of VHDL's philosophy of readability:
    ///
    ///   Verilog: assign y = (a & b) | (c ^ d);
    ///   VHDL:    y <= (a and b) or (c xor d);
    #[test]
    fn test_keyword_operators_logical() {
        let keywords = ["and", "or", "xor", "nand", "nor", "xnor", "not"];

        for kw in &keywords {
            let source = format!("{kw};");
            let tokens = tokenize_vhdl(&source);
            let pairs = token_pairs(&tokens);

            assert_eq!(
                pairs[0].0, TokenType::Keyword,
                "Expected '{}' to be a KEYWORD, got {:?}",
                kw, pairs[0].0
            );
            assert_eq!(pairs[0].1, *kw);
        }
    }

    /// Arithmetic keyword operators: mod, rem, abs.
    #[test]
    fn test_keyword_operators_arithmetic() {
        let keywords = ["mod", "rem", "abs"];

        for kw in &keywords {
            let source = format!("a {kw} b;");
            let tokens = tokenize_vhdl(&source);
            let pairs = token_pairs(&tokens);

            let has_kw = pairs.iter().any(|(t, v)| *t == TokenType::Keyword && *v == *kw);
            assert!(has_kw, "Expected '{}' to be a KEYWORD", kw);
        }
    }

    /// Shift keyword operators: sll, srl, sla, sra, rol, ror.
    #[test]
    fn test_keyword_operators_shift() {
        let keywords = ["sll", "srl", "sla", "sra", "rol", "ror"];

        for kw in &keywords {
            let source = format!("a {kw} b;");
            let tokens = tokenize_vhdl(&source);
            let pairs = token_pairs(&tokens);

            let has_kw = pairs.iter().any(|(t, v)| *t == TokenType::Keyword && *v == *kw);
            assert!(has_kw, "Expected '{}' to be a KEYWORD", kw);
        }
    }

    // =======================================================================
    // Comments
    // =======================================================================

    /// VHDL uses -- for single-line comments (like Ada and Haskell).
    /// Comments should be skipped entirely.
    #[test]
    fn test_single_line_comments() {
        let tokens = tokenize_vhdl("signal a : std_logic; -- this is a comment\nsignal b : std_logic;");
        let pairs = token_pairs(&tokens);

        // "comment" should NOT appear as a token
        let has_comment_text = pairs.iter().any(|(_, v)| v.contains("comment"));
        assert!(!has_comment_text, "Comments should be skipped");

        // Both signal declarations should be present
        let signal_count = pairs.iter().filter(|(t, v)| *t == TokenType::Keyword && *v == "signal").count();
        assert_eq!(signal_count, 2, "Both signal declarations should be tokenized");
    }

    // =======================================================================
    // Strings
    // =======================================================================

    /// VHDL strings use double quotes. Embedded quotes are doubled:
    /// "He said ""hello""" contains: He said "hello"
    #[test]
    fn test_string_literals() {
        let tokens = tokenize_vhdl("\"hello world\";");
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token");
    }

    #[test]
    fn test_string_with_escaped_quotes() {
        let tokens = tokenize_vhdl("\"He said \"\"hi\"\"\";");
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token with escaped quotes");
    }

    // =======================================================================
    // Number literals
    // =======================================================================

    #[test]
    fn test_plain_numbers() {
        let tokens = tokenize_vhdl("42;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "42");
    }

    #[test]
    fn test_real_numbers() {
        let tokens = tokenize_vhdl("3.14;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].1, "3.14");
    }

    #[test]
    fn test_real_number_with_exponent() {
        let tokens = tokenize_vhdl("1.0E-3;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].1, "1.0E-3");
    }

    // =======================================================================
    // Delimiters
    // =======================================================================

    #[test]
    fn test_delimiters() {
        let tokens = tokenize_vhdl("();[],.:;");
        let pairs = token_pairs(&tokens);

        let values: Vec<&str> = pairs.iter().map(|(_, v)| *v).collect();
        assert!(values.contains(&"("));
        assert!(values.contains(&")"));
        assert!(values.contains(&";"));
        assert!(values.contains(&"["));
        assert!(values.contains(&"]"));
        assert!(values.contains(&","));
        assert!(values.contains(&"."));
        assert!(values.contains(&":"));
    }

    // =======================================================================
    // Keywords
    // =======================================================================

    /// Test that all major VHDL keywords are recognized.
    #[test]
    fn test_keywords() {
        let keywords = [
            "entity", "architecture", "signal", "variable", "constant",
            "process", "begin", "end", "if", "then", "else", "elsif",
            "case", "when", "for", "loop", "while", "component",
            "port", "generic", "map", "is", "of", "in", "out", "inout",
            "downto", "to", "others", "library", "use", "package",
            "function", "procedure", "return", "type", "subtype",
            "array", "record", "range", "null", "open", "after",
            "wait", "until", "report", "severity", "assert",
        ];

        for kw in &keywords {
            let source = format!("{kw};");
            let tokens = tokenize_vhdl(&source);
            let pairs = token_pairs(&tokens);

            assert_eq!(
                pairs[0].0, TokenType::Keyword,
                "Expected '{}' to be a KEYWORD, got {:?}",
                kw, pairs[0].0
            );
        }
    }

    // =======================================================================
    // Factory function
    // =======================================================================

    /// The `create_vhdl_lexer` factory function should return a working
    /// `GrammarLexer`.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_vhdl_lexer("42;");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // =======================================================================
    // Whitespace handling
    // =======================================================================

    #[test]
    fn test_whitespace_skipped() {
        let compact = tokenize_vhdl("signal a:std_logic;");
        let spaced = tokenize_vhdl("signal  a  :  std_logic  ;");

        let pairs_compact = token_pairs(&compact);
        let pairs_spaced = token_pairs(&spaced);

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
    }

    // =======================================================================
    // Complete VHDL snippets
    // =======================================================================

    /// A half adder — one of the simplest combinational circuits.
    /// This exercises entity declarations, port lists, architecture bodies,
    /// signal assignment (<=), and keyword operators (xor, and).
    #[test]
    fn test_half_adder() {
        let source = r#"
entity half_adder is
    port (
        a    : in  std_logic;
        b    : in  std_logic;
        sum  : out std_logic;
        carry: out std_logic
    );
end entity half_adder;

architecture rtl of half_adder is
begin
    sum   <= a xor b;
    carry <= a and b;
end architecture rtl;
"#;
        let tokens = tokenize_vhdl(source);
        let pairs = token_pairs(&tokens);

        // Should have many tokens without errors
        assert!(pairs.len() > 30, "Expected many tokens from half adder, got {}", pairs.len());

        // Check key structural tokens
        assert_eq!(pairs[0], (TokenType::Keyword, "entity"));

        // Check that xor and and are keywords
        let has_xor = pairs.iter().any(|(t, v)| *t == TokenType::Keyword && *v == "xor");
        let has_and = pairs.iter().any(|(t, v)| *t == TokenType::Keyword && *v == "and");
        assert!(has_xor, "Expected 'xor' keyword in half adder");
        assert!(has_and, "Expected 'and' keyword in half adder");

        // Check that <= (signal assignment) is present
        let has_assign = pairs.iter().any(|(_, v)| *v == "<=");
        assert!(has_assign, "Expected '<=' signal assignment");
    }

    /// A D flip-flop with asynchronous reset — the fundamental sequential
    /// building block. This exercises process statements, sensitivity lists,
    /// if/then/else, signal assignment, and rising_edge detection.
    #[test]
    fn test_d_flip_flop() {
        let source = r#"
process(clk, reset)
begin
    if reset = '1' then
        q <= '0';
    elsif clk = '1' then
        q <= d;
    end if;
end process;
"#;
        let tokens = tokenize_vhdl(source);
        let pairs = token_pairs(&tokens);

        assert!(pairs.len() > 20, "Expected many tokens from D flip-flop");

        // Check process keyword
        assert_eq!(pairs[0], (TokenType::Keyword, "process"));

        // Check character literals are preserved
        let has_char_1 = pairs.iter().any(|(_, v)| *v == "'1'");
        let has_char_0 = pairs.iter().any(|(_, v)| *v == "'0'");
        assert!(has_char_1, "Expected character literal '1'");
        assert!(has_char_0, "Expected character literal '0'");

        // Check elsif keyword
        let has_elsif = pairs.iter().any(|(t, v)| *t == TokenType::Keyword && *v == "elsif");
        assert!(has_elsif, "Expected 'elsif' keyword");
    }

    /// Entity with port map using the => arrow operator.
    #[test]
    fn test_port_map_arrow() {
        let source = "port map (a => x, b => y);";
        let tokens = tokenize_vhdl(source);
        let pairs = token_pairs(&tokens);

        let arrow_count = pairs.iter().filter(|(_, v)| *v == "=>").count();
        assert_eq!(arrow_count, 2, "Expected two '=>' arrows in port map");
    }

    /// Variable assignment uses := (not <= which is signal assignment).
    #[test]
    fn test_variable_assignment() {
        let source = "variable count : integer := 0;";
        let tokens = tokenize_vhdl(source);
        let pairs = token_pairs(&tokens);

        let has_var_assign = pairs.iter().any(|(_, v)| *v == ":=");
        assert!(has_var_assign, "Expected ':=' variable assignment operator");
    }

    /// The box (<>) operator appears in unconstrained array types.
    #[test]
    fn test_box_operator() {
        let source = "type word is array (natural range <>) of std_logic;";
        let tokens = tokenize_vhdl(source);
        let pairs = token_pairs(&tokens);

        let has_box = pairs.iter().any(|(_, v)| *v == "<>");
        assert!(has_box, "Expected '<>' box operator");
    }

    /// The tick (') operator for attribute access: signal'event, signal'length.
    #[test]
    fn test_tick_attribute() {
        // Note: after a closing paren or identifier, ' is the tick/attribute marker.
        // The lexer just produces a TICK token; the parser resolves the meaning.
        let tokens = tokenize_vhdl("clk'event");
        let pairs = token_pairs(&tokens);

        // Should have: NAME("clk"), TICK("'"), NAME("event")
        assert_eq!(pairs[0], (TokenType::Name, "clk"));
        // The tick might be combined with "event" or separate depending on grammar
        let has_tick = pairs.iter().any(|(_, v)| *v == "'");
        let has_event = pairs.iter().any(|(_, v)| *v == "event");
        assert!(has_tick, "Expected tick (') token");
        assert!(has_event, "Expected 'event' identifier");
    }

    /// The not-equals operator /= (unique to VHDL — most languages use !=).
    #[test]
    fn test_not_equals_operator() {
        let tokens = tokenize_vhdl("a /= b;");
        let pairs = token_pairs(&tokens);

        let has_neq = pairs.iter().any(|(_, v)| *v == "/=");
        assert!(has_neq, "Expected '/=' not-equals operator");
    }

    /// The pipe (|) operator used in case statement alternatives.
    #[test]
    fn test_pipe_operator() {
        let source = "when \"00\" | \"01\" => null;";
        let tokens = tokenize_vhdl(source);
        let pairs = token_pairs(&tokens);

        let has_pipe = pairs.iter().any(|(_, v)| *v == "|");
        assert!(has_pipe, "Expected '|' pipe operator");
    }

    /// Verify the power operator (**) works.
    #[test]
    fn test_power_operator() {
        let tokens = tokenize_vhdl("2 ** 3;");
        let pairs = token_pairs(&tokens);

        let has_power = pairs.iter().any(|(_, v)| *v == "**");
        assert!(has_power, "Expected '**' power operator");
    }

    /// Case-insensitive keyword operators should work when written in
    /// any case combination.
    #[test]
    fn test_case_insensitive_keyword_operators() {
        let tokens = tokenize_vhdl("a AND b OR c XOR d;");
        let pairs = token_pairs(&tokens);

        // After normalization, all should be lowercase keywords
        let has_and = pairs.iter().any(|(t, v)| *t == TokenType::Keyword && *v == "and");
        let has_or = pairs.iter().any(|(t, v)| *t == TokenType::Keyword && *v == "or");
        let has_xor = pairs.iter().any(|(t, v)| *t == TokenType::Keyword && *v == "xor");

        assert!(has_and, "Expected lowercased 'and' keyword");
        assert!(has_or, "Expected lowercased 'or' keyword");
        assert!(has_xor, "Expected lowercased 'xor' keyword");
    }

    /// A complete generic entity with multiple port types.
    #[test]
    fn test_entity_with_generics() {
        let source = r#"
entity counter is
    generic (
        WIDTH : integer := 8
    );
    port (
        clk   : in  std_logic;
        reset : in  std_logic;
        count : out std_logic_vector(WIDTH - 1 downto 0)
    );
end entity counter;
"#;
        let tokens = tokenize_vhdl(source);
        let pairs = token_pairs(&tokens);

        assert!(pairs.len() > 25, "Expected many tokens from generic entity");
        assert_eq!(pairs[0], (TokenType::Keyword, "entity"));

        let has_generic = pairs.iter().any(|(t, v)| *t == TokenType::Keyword && *v == "generic");
        assert!(has_generic, "Expected 'generic' keyword");

        let has_integer = pairs.iter().any(|(_, v)| *v == "integer");
        assert!(has_integer, "Expected 'integer' type name");
    }
}
