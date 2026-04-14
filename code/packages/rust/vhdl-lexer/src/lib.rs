//! VHDL lexer backed by compiled token grammar with post-tokenization case normalization.

use std::collections::HashSet;

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};

mod _grammar;

pub const DEFAULT_VERSION: &str = "2008";
pub const SUPPORTED_VERSIONS: &[&str] = _grammar::SUPPORTED_VERSIONS;

fn validate_version(version: &str) -> Result<&str, String> {
    if SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown VHDL version '{version}'. Valid values: {}",
            SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

fn normalize_case(tokens: &mut [Token], keywords: &HashSet<String>) {
    for token in tokens.iter_mut() {
        let should_normalize = match token.type_ {
            TokenType::Keyword => true,
            TokenType::Name => match token.type_name.as_deref() {
                Some("NAME") => true,
                None => true,
                Some(_) => false,
            },
            _ => false,
        };

        if should_normalize {
            token.value = token.value.to_lowercase();
            if token.type_ == TokenType::Name && keywords.contains(&token.value) {
                token.type_ = TokenType::Keyword;
            }
        }
    }
}

pub fn create_vhdl_lexer(source: &str) -> GrammarLexer<'_> {
    create_vhdl_lexer_with_version(source, DEFAULT_VERSION)
        .expect("compiled VHDL token grammar missing default version")
}

pub fn create_vhdl_lexer_with_version<'src>(
    source: &'src str,
    version: &str,
) -> Result<GrammarLexer<'src>, String> {
    let version = validate_version(version)?;
    let grammar = _grammar::token_grammar(version)
        .expect("compiled VHDL token grammar missing supported version");
    Ok(GrammarLexer::new(source, &grammar))
}

pub fn tokenize_vhdl(source: &str) -> Vec<Token> {
    tokenize_vhdl_with_version(source, DEFAULT_VERSION)
        .expect("compiled VHDL token grammar missing default version")
}

pub fn tokenize_vhdl_with_version(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let version = validate_version(version)?;
    let grammar = _grammar::token_grammar(version)
        .expect("compiled VHDL token grammar missing supported version");
    let keyword_set: HashSet<String> = grammar.keywords.iter().cloned().collect();
    let mut vhdl_lexer = GrammarLexer::new(source, &grammar);
    let mut tokens = vhdl_lexer
        .tokenize()
        .map_err(|e| format!("VHDL tokenization failed: {e}"))?;
    normalize_case(&mut tokens, &keyword_set);
    Ok(tokens)
}

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

    #[test]
    fn test_default_version_matches_explicit_2008() {
        let default_tokens = tokenize_vhdl("entity top is end entity top;");
        let explicit_tokens =
            tokenize_vhdl_with_version("entity top is end entity top;", "2008").unwrap();
        assert_eq!(default_tokens.len(), explicit_tokens.len());
    }

    #[test]
    fn test_unknown_version_rejected() {
        let err = tokenize_vhdl_with_version("entity top is end entity top;", "2099")
            .expect_err("unknown versions should be rejected");
        assert!(err.contains("Unknown VHDL version"));
    }
}

