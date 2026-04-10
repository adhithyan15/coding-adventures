// lexer — Brainfuck tokenizer using the grammar-driven lexer infrastructure
// ==========================================================================
//
// This module is the tokenization layer for Brainfuck source code. It is one
// of three layers in the Brainfuck front-end pipeline:
//
//   brainfuck.tokens    (grammar file on disk — declarative token definitions)
//          |
//          v
//   grammar-tools       (parses .tokens → TokenGrammar struct)
//          |
//          v
//   lexer::GrammarLexer (tokenizes source using TokenGrammar)
//          |
//          v
//   Vec<Token>          (flat list of typed tokens)
//
// # Why is tokenization interesting for Brainfuck?
//
// Brainfuck is deceptively simple: it has only 8 meaningful characters and
// treats every other character as a comment. This makes it a great first
// example for the grammar-driven lexer because the grammar is tiny and the
// comment-skipping behavior shows off the `skip:` section of the .tokens
// format.
//
// # Token types produced
//
// The brainfuck.tokens grammar defines these tokens:
//
//   RIGHT      ">"    Move data pointer right
//   LEFT       "<"    Move data pointer left
//   INC        "+"    Increment current cell
//   DEC        "-"    Decrement current cell
//   OUTPUT     "."    Output current cell as ASCII
//   INPUT      ","    Read one byte into current cell
//   LOOP_START "["    Begin loop (jump past ] if cell == 0)
//   LOOP_END   "]"    End loop (jump back to [ if cell != 0)
//
// The skip: section silently consumes whitespace and any non-command
// character, so those never appear in the token stream. The parser sees
// only the eight command tokens plus EOF.
//
// # Path navigation
//
// This module lives inside the `brainfuck` crate. The crate's Cargo.toml
// is at `code/packages/rust/brainfuck/Cargo.toml`, so CARGO_MANIFEST_DIR
// points to `code/packages/rust/brainfuck/`. From there, the grammar file
// is at `../../../grammars/brainfuck.tokens`.

use std::fs;

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `brainfuck.tokens` grammar file.
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
///     brainfuck.tokens           <-- this is what we want
///   packages/
///     rust/
///       brainfuck/
///         Cargo.toml             <-- CARGO_MANIFEST_DIR points here
///         src/
///           lexer.rs             <-- we are here
/// ```
///
/// So the relative path from CARGO_MANIFEST_DIR to the grammar file is:
/// `../../../grammars/brainfuck.tokens`
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/brainfuck.tokens")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for Brainfuck source text.
///
/// This function:
/// 1. Reads the `brainfuck.tokens` grammar file from disk.
/// 2. Parses it into a `TokenGrammar` using `grammar-tools`.
/// 3. Constructs a `GrammarLexer` with the grammar and the given source.
///
/// The returned lexer is ready to call `.tokenize()` on. Use this when you
/// need access to the lexer object itself (e.g., for incremental tokenization
/// or custom error handling).
///
/// # How the grammar drives tokenization
///
/// The `brainfuck.tokens` file declares 8 literal token patterns (one per
/// command character) plus two `skip:` patterns:
///
/// - `WHITESPACE = /[ \t\r\n]+/` — consumed silently, advances line counter
/// - `COMMENT    = /[^><+\-.,\[\] \t\r\n]+/` — any non-command, non-whitespace
///
/// Because skip patterns are tried before token patterns, all comment text
/// is discarded before the command characters are matched. The token stream
/// the parser receives is clean: only the 8 commands and EOF.
///
/// # Panics
///
/// Panics if the grammar file cannot be read or parsed. This should never
/// happen in a correctly cloned repository — the grammar file is checked in
/// and validated by grammar-tools tests. A panic here indicates a broken
/// build or missing file.
///
/// # Example
///
/// ```no_run
/// use brainfuck::lexer::create_brainfuck_lexer;
///
/// let mut lexer = create_brainfuck_lexer("++[>+<-]");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{}", token);
/// }
/// ```
pub fn create_brainfuck_lexer(source: &str) -> GrammarLexer<'_> {
    // Step 1: Read the grammar file from disk.
    //
    // We read at runtime (not compile time via include_str!) because grammar
    // files may be updated independently of Rust source. This also avoids
    // embedding grammar text in the binary.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read brainfuck.tokens: {e}"));

    // Step 2: Parse the grammar text into a structured TokenGrammar.
    //
    // The TokenGrammar for Brainfuck contains:
    //   - 8 token definitions (one per command character)
    //   - 2 skip patterns (WHITESPACE and COMMENT)
    //   - No keywords (Brainfuck has none — all tokens are punctuation)
    //   - Mode: default (no indentation tracking)
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse brainfuck.tokens: {e}"));

    // Step 3: Create and return the lexer.
    //
    // The GrammarLexer compiles all token patterns into anchored regexes
    // and is ready to tokenize the source string. Because skip patterns
    // consume comments, the caller only ever sees command tokens and EOF.
    GrammarLexer::new(source, &grammar)
}

/// Tokenize Brainfuck source text into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, and tokenization in one call. The returned vector always
/// ends with an `EOF` token.
///
/// Comment characters (any character that is not `><+-.,[]`) are silently
/// discarded by the lexer's skip patterns. Only command tokens appear in
/// the output.
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed. Brainfuck source
/// never causes a tokenization error because every non-command character
/// is treated as a comment — there is no "unexpected character" case.
///
/// # Example
///
/// ```no_run
/// use brainfuck::lexer::tokenize_brainfuck;
///
/// let tokens = tokenize_brainfuck("++ hello [>+<-] comment");
/// // tokens: INC, INC, LOOP_START, RIGHT, INC, LEFT, DEC, LOOP_END, EOF
/// // ("hello" and "comment" are silently skipped as comments)
/// for token in &tokens {
///     println!("{:?}", token.type_name);
/// }
/// ```
pub fn tokenize_brainfuck(source: &str) -> Vec<Token> {
    // Create a fresh lexer for this source text.
    let mut bf_lexer = create_brainfuck_lexer(source);

    // Tokenize and unwrap.
    //
    // Unlike JSON, Brainfuck tokenization cannot fail on bad input: the
    // skip patterns catch every non-command character. So unwrap_or_else
    // with a panic is safe in practice.
    bf_lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Brainfuck tokenization failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper: extract type_name strings for non-EOF tokens.
    //
    // Brainfuck tokens are not built-in TokenType variants (they're not
    // NUMBER, STRING, etc.), so they all come back as TokenType::Name with
    // type_name set to the grammar name. This helper collects those names.
    // -----------------------------------------------------------------------

    /// Collect the type_name values from a token stream, excluding EOF.
    ///
    /// Because all Brainfuck token types are custom (not built-in TokenType
    /// variants), they appear as TokenType::Name with type_name = Some("INC"),
    /// Some("DEC"), etc. This helper makes test assertions concise.
    fn names(tokens: &[Token]) -> Vec<&str> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.type_name.as_deref().unwrap_or("?"))
            .collect()
    }

    /// Collect the raw string values from non-EOF tokens.
    fn values(tokens: &[Token]) -> Vec<&str> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: RIGHT command
    // -----------------------------------------------------------------------

    /// ">" produces a single RIGHT token.
    #[test]
    fn test_right_token() {
        let tokens = tokenize_brainfuck(">");
        let n = names(&tokens);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0], "RIGHT");
    }

    // -----------------------------------------------------------------------
    // Test 2: LEFT command
    // -----------------------------------------------------------------------

    /// "<" produces a single LEFT token.
    #[test]
    fn test_left_token() {
        let tokens = tokenize_brainfuck("<");
        let n = names(&tokens);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0], "LEFT");
    }

    // -----------------------------------------------------------------------
    // Test 3: INC command
    // -----------------------------------------------------------------------

    /// "+" produces a single INC token.
    #[test]
    fn test_inc_token() {
        let tokens = tokenize_brainfuck("+");
        let n = names(&tokens);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0], "INC");
    }

    // -----------------------------------------------------------------------
    // Test 4: DEC command
    // -----------------------------------------------------------------------

    /// "-" produces a single DEC token.
    #[test]
    fn test_dec_token() {
        let tokens = tokenize_brainfuck("-");
        let n = names(&tokens);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0], "DEC");
    }

    // -----------------------------------------------------------------------
    // Test 5: OUTPUT command
    // -----------------------------------------------------------------------

    /// "." produces a single OUTPUT token.
    #[test]
    fn test_output_token() {
        let tokens = tokenize_brainfuck(".");
        let n = names(&tokens);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0], "OUTPUT");
    }

    // -----------------------------------------------------------------------
    // Test 6: INPUT command
    // -----------------------------------------------------------------------

    /// "," produces a single INPUT token.
    #[test]
    fn test_input_token() {
        let tokens = tokenize_brainfuck(",");
        let n = names(&tokens);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0], "INPUT");
    }

    // -----------------------------------------------------------------------
    // Test 7: LOOP_START command
    // -----------------------------------------------------------------------

    /// "[" produces a single LOOP_START token.
    #[test]
    fn test_loop_start_token() {
        let tokens = tokenize_brainfuck("[");
        let n = names(&tokens);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0], "LOOP_START");
    }

    // -----------------------------------------------------------------------
    // Test 8: LOOP_END command
    // -----------------------------------------------------------------------

    /// "]" produces a single LOOP_END token.
    #[test]
    fn test_loop_end_token() {
        let tokens = tokenize_brainfuck("]");
        let n = names(&tokens);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0], "LOOP_END");
    }

    // -----------------------------------------------------------------------
    // Test 9: All eight commands together
    // -----------------------------------------------------------------------

    /// "+-><.,[]" produces all 8 command tokens in order.
    /// This verifies the grammar covers all command characters.
    #[test]
    fn test_all_eight_commands() {
        let tokens = tokenize_brainfuck("><+-.,[]{");
        // The '{' is a comment (not a command), so it is silently skipped.
        // We expect 8 command tokens.
        let n = names(&tokens);
        assert_eq!(n, vec!["RIGHT", "LEFT", "INC", "DEC", "OUTPUT", "INPUT", "LOOP_START", "LOOP_END"]);
    }

    // -----------------------------------------------------------------------
    // Test 10: Comment skipping
    // -----------------------------------------------------------------------

    /// Characters that are not Brainfuck commands are silently discarded.
    /// In Brainfuck, any non-command character is a comment. This is
    /// how Brainfuck programs are annotated — by embedding natural language.
    #[test]
    fn test_comment_skipping() {
        // This source has a comment between each command.
        // The comment characters should all be silently consumed.
        let source = "+ increment the pointer - decrement it";
        let tokens = tokenize_brainfuck(source);
        let n = names(&tokens);
        // Only + and - are commands; all other chars are comments.
        assert_eq!(n, vec!["INC", "DEC"]);
    }

    // -----------------------------------------------------------------------
    // Test 11: Comment with newlines
    // -----------------------------------------------------------------------

    /// Multi-line Brainfuck source with block comments.
    /// Newlines within comments should not affect the token stream.
    #[test]
    fn test_multiline_comment_skipping() {
        let source = "++\n  increment twice\n>>\n  move right twice";
        let tokens = tokenize_brainfuck(source);
        let n = names(&tokens);
        assert_eq!(n, vec!["INC", "INC", "RIGHT", "RIGHT"]);
    }

    // -----------------------------------------------------------------------
    // Test 12: Line/col tracking
    // -----------------------------------------------------------------------

    /// The first token should be at line 1, col 1.
    /// Whitespace before a token advances the column counter.
    #[test]
    fn test_line_col_tracking() {
        let tokens = tokenize_brainfuck("+");
        let first = tokens.iter().find(|t| t.type_ != TokenType::Eof).unwrap();
        // "+" is the very first character, so line=1, col=1.
        assert_eq!(first.line, 1);
        assert_eq!(first.column, 1);
    }

    // -----------------------------------------------------------------------
    // Test 13: Empty source → just EOF
    // -----------------------------------------------------------------------

    /// An empty source string produces only the EOF sentinel token.
    /// This is the base case: no input, no commands, one EOF.
    #[test]
    fn test_empty_source() {
        let tokens = tokenize_brainfuck("");
        // Should be exactly one token: EOF.
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 14: Comments-only source
    // -----------------------------------------------------------------------

    /// A string containing only comment characters (no commands) produces
    /// just EOF — the lexer consumes all of it via skip patterns.
    #[test]
    fn test_comments_only() {
        let tokens = tokenize_brainfuck("hello this is all a comment");
        let n = names(&tokens);
        assert_eq!(n.len(), 0, "Comments-only source should have no command tokens");
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 15: Canonical program "++[>+<-]"
    // -----------------------------------------------------------------------

    /// The classic "add 2 to cell 1" program. This exercises:
    /// - Multiple INC tokens in sequence
    /// - LOOP_START and LOOP_END bracketing
    /// - RIGHT, INC, LEFT, DEC inside the loop body
    ///
    /// Expected token sequence:
    ///   INC INC LOOP_START RIGHT INC LEFT DEC LOOP_END EOF
    #[test]
    fn test_canonical_program() {
        let tokens = tokenize_brainfuck("++[>+<-]");
        let n = names(&tokens);
        assert_eq!(
            n,
            vec!["INC", "INC", "LOOP_START", "RIGHT", "INC", "LEFT", "DEC", "LOOP_END"]
        );
        // All command characters should have correct values.
        let v = values(&tokens);
        assert_eq!(v, vec!["+", "+", "[", ">", "+", "<", "-", "]"]);
    }

    // -----------------------------------------------------------------------
    // Test 16: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_brainfuck_lexer` factory function should return a
    /// `GrammarLexer` that can successfully tokenize source text.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_brainfuck_lexer("+");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");
        // Should produce: INC, EOF
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 17: Annotated Brainfuck — natural language in program
    // -----------------------------------------------------------------------

    /// Real Brainfuck programs often contain inline prose documentation.
    /// For example: "increment the cell [loop while nonzero: go right] end"
    /// The entire prose is comment text, leaving only +, [, >, ].
    #[test]
    fn test_annotated_program() {
        let source = "++ two increments [loop body >+<-] done";
        let tokens = tokenize_brainfuck(source);
        let n = names(&tokens);
        assert_eq!(n, vec!["INC", "INC", "LOOP_START", "RIGHT", "INC", "LEFT", "DEC", "LOOP_END"]);
    }
}
