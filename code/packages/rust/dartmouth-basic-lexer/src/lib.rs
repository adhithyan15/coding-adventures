//! # Dartmouth BASIC Lexer — tokenizing the original 1964 BASIC source text.
//!
//! [Dartmouth BASIC](https://en.wikipedia.org/wiki/Dartmouth_BASIC) was the
//! first BASIC programming language, designed in 1964 by John G. Kemeny and
//! Thomas E. Kurtz at Dartmouth College. It ran on a GE-225 mainframe and
//! was accessed through teletypes — mechanical printing terminals that could
//! only produce uppercase characters. This historical detail is why BASIC is
//! case-insensitive: the hardware simply did not have lowercase.
//!
//! BASIC stood for "Beginner's All-purpose Symbolic Instruction Code". The
//! design goals were deliberately beginner-friendly:
//!
//! - **Line numbers** — every statement must begin with a number like `10` or
//!   `200`. This number is both an address (where to jump with GOTO) and an
//!   ordering key (lines execute in numeric order, regardless of input order).
//!
//! - **Simple syntax** — no declarations, no type annotations. Variables are
//!   just names: single letters (`A`, `B`) or letter+digit (`A1`, `X9`). All
//!   variables start at zero.
//!
//! - **Forgiving** — every variable pre-initialized to 0, no "undeclared
//!   variable" errors, simple error messages.
//!
//! # Architecture
//!
//! This crate is a thin wrapper around the generic [`GrammarLexer`] from the
//! `lexer` crate. The tokenization pipeline has three layers:
//!
//! ```text
//! dartmouth_basic.tokens  (grammar file — declares every token pattern)
//!        |
//!        v
//! grammar-tools           (parses .tokens file → TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer     (tokenizes source using TokenGrammar + hooks)
//!        |
//!        v
//! dartmouth-basic-lexer   (this crate — adds post-tokenize hooks)
//! ```
//!
//! The grammar file lives at `code/grammars/dartmouth_basic.tokens` so all
//! language implementations (Elixir, Python, Ruby, Rust, …) share one source
//! of truth for the token definitions.
//!
//! # Post-tokenize hooks
//!
//! Two transformations are applied to the raw token stream after the grammar
//! engine has run:
//!
//! ## Hook 1: `relabel_line_numbers`
//!
//! The grammar defines `LINE_NUM = /[0-9]+/` before `NUMBER = /[0-9]*…/`.
//! Because first-match-wins, ALL bare integers lex as LINE_NUM. But only
//! the *first* number on each line is actually a line label — subsequent
//! integers are expression values.
//!
//! ```text
//! Before hook:  LINE_NUM("10") KEYWORD("GOTO") LINE_NUM("100") NEWLINE
//! After hook:   LINE_NUM("10") KEYWORD("GOTO") NUMBER("100")   NEWLINE
//! ```
//!
//! The hook walks the token list and relabels LINE_NUM→NUMBER for any token
//! that is NOT the first token on its line.
//!
//! ## Hook 2: `suppress_rem_content`
//!
//! `REM` introduces a remark (comment) that runs to the end of the line.
//! Everything between `REM` and the next `NEWLINE` is discarded:
//!
//! ```text
//! Source:    10 REM THIS COMPUTES THE ANSWER
//! Before:    LINE_NUM("10") KEYWORD("REM") NAME("THIS") … NEWLINE
//! After:     LINE_NUM("10") KEYWORD("REM") NEWLINE
//! ```
//!
//! # Public API
//!
//! - [`create_dartmouth_basic_lexer`] — returns a configured `GrammarLexer`
//!   with both hooks registered.
//! - [`tokenize_dartmouth_basic`] — convenience one-shot function returning
//!   `Vec<Token>`.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::{Token, TokenType};
mod _grammar;

// ===========================================================================
// Post-tokenize hooks
// ===========================================================================

/// Relabel bare-integer tokens that are NOT at line start from LINE_NUM to NUMBER.
///
/// # Why this is necessary
///
/// In the `dartmouth_basic.tokens` grammar, `LINE_NUM = /[0-9]+/` is listed
/// *before* `NUMBER = /[0-9]*\.?[0-9]+.../`. Because the grammar engine uses
/// first-match-wins semantics, every bare integer in the source (e.g., the
/// `100` in `GOTO 100`) will initially be emitted as a `LINE_NUM` token, not
/// a `NUMBER` token.
///
/// But LINE_NUM has a specific meaning: it is the **line label** that appears
/// at the very beginning of a physical BASIC line. The `100` in `GOTO 100`
/// is not a line label — it is an integer operand. The parser needs to know
/// the difference:
///
/// ```text
/// 10 GOTO 100
/// ^          ^
/// |          `-- NUMBER: an argument to GOTO (any integer expression)
/// `-- LINE_NUM: the address of this line itself (special syntax)
/// ```
///
/// This hook fixes the raw output by walking the token list and:
/// - Keeping the **first** LINE_NUM on each line as LINE_NUM (it IS the label)
/// - Relabeling all **subsequent** LINE_NUM tokens on the same line to NUMBER
///
/// # Implementation walkthrough
///
/// We track a single boolean flag `at_line_start`. It starts `true` (the very
/// first token in the file is in "line start" position). After seeing any
/// non-newline token, it becomes `false`. When we see a NEWLINE, it resets to
/// `true`.
///
/// ```text
/// Token stream (before hook):
///   LINE_NUM("10")  ← at_line_start=true  → keep as LINE_NUM, set flag=false
///   KEYWORD("LET")  ← at_line_start=false → unchanged
///   NAME("X")       ← at_line_start=false → unchanged
///   NEWLINE         ← reset flag to true
///   LINE_NUM("20")  ← at_line_start=true  → keep as LINE_NUM, set flag=false
///   KEYWORD("GOTO") ← at_line_start=false → unchanged
///   LINE_NUM("10")  ← at_line_start=false → RELABEL to NUMBER
///   NEWLINE         ← reset flag to true
/// ```
fn relabel_line_numbers(tokens: Vec<Token>) -> Vec<Token> {
    // Preallocate the result vector with the same capacity — we never add
    // or remove tokens, only modify their type_name fields.
    let mut result = Vec::with_capacity(tokens.len());

    // `at_line_start` tracks whether the next token we see occupies the
    // "line number position" at the beginning of a BASIC line.
    let mut at_line_start = true;

    for mut tok in tokens {
        // The grammar engine does NOT produce LINE_NUM — it produces NUMBER
        // for all bare integers (because NUMBER also matches pure integers and
        // the grammar engine picks NUMBER as the canonical match). We therefore
        // need to *promote* NUMBER→LINE_NUM at line start, rather than
        // *demote* LINE_NUM→NUMBER away from line start.
        //
        // Concrete examples (raw output before this hook):
        //   "10 GOTO 100\n"  →  NUMBER("10") KEYWORD("GOTO") NUMBER("100") NEWLINE
        // After this hook:
        //   LINE_NUM("10") KEYWORD("GOTO") NUMBER("100") NEWLINE
        //
        // We use type_=Name with type_name=Some("LINE_NUM") because LINE_NUM is
        // a grammar-defined custom type (not one of the TokenType enum variants).
        // effective_type_name() returns "LINE_NUM" for such tokens.
        if at_line_start && tok.effective_type_name() == "NUMBER" {
            // This is the first token on its line AND it is an integer.
            // In BASIC, this is the line label (e.g., the `10` in `10 PRINT X`).
            // Promote it to LINE_NUM.
            tok.type_ = TokenType::Name;
            tok.type_name = Some("LINE_NUM".to_string());
            // Clear the flag — remaining tokens on this line are in expression
            // position, not line-number position.
            at_line_start = false;
        } else if at_line_start {
            // Non-integer at line start (e.g., a blank line that has NEWLINE first).
            // Clear the flag — nothing more to do for line-number position.
            at_line_start = false;
        }

        // NEWLINE signals the end of a BASIC statement. The next token will
        // begin a fresh line, so we reset `at_line_start`.
        if tok.type_ == TokenType::Newline {
            at_line_start = true;
        }

        result.push(tok);
    }

    result
}

/// Suppress all tokens between a `REM` keyword and the end of its line.
///
/// # What REM does
///
/// `REM` introduces a remark — a programmer comment. Everything from `REM` to
/// the next newline (inclusive of `REM` but exclusive of the newline) is a
/// human-readable annotation that the BASIC interpreter ignores entirely.
///
/// ```text
/// 10 REM SET X TO THE STARTING VALUE
/// 20 LET X = 1
/// ```
///
/// The first line would naively lex as:
/// ```text
/// LINE_NUM("10") KEYWORD("REM") NAME("SET") NAME("X") KEYWORD("TO") …
/// ```
///
/// After suppression it becomes:
/// ```text
/// LINE_NUM("10") KEYWORD("REM") NEWLINE
/// ```
///
/// This is what the parser expects: `REM` followed immediately by a newline.
///
/// # Algorithm
///
/// We track a `suppressing` flag. When we encounter a KEYWORD token with value
/// `"REM"`, we set `suppressing = true`. While suppressing, we drop all
/// subsequent tokens. When we see a NEWLINE, we stop suppressing (the newline
/// itself is kept — the parser needs it as a statement terminator).
///
/// Note the ordering: we FIRST check whether to suppress the current token,
/// THEN check whether the current token should turn suppression on or off.
/// This ensures:
/// - The `REM` token itself is always kept (we suppress *after* `REM`).
/// - The NEWLINE token is always kept (we stop suppressing when we see it).
///
/// ```text
/// Token stream:
///   LINE_NUM("10")    suppressing=false → KEEP
///   KEYWORD("REM")    suppressing=false → KEEP, then set suppressing=true
///   NAME("SET")       suppressing=true  → DROP
///   NAME("X")         suppressing=true  → DROP
///   KEYWORD("TO")     suppressing=true  → DROP
///   NEWLINE           suppressing=true  → KEEP (it's NEWLINE), set suppressing=false
/// ```
fn suppress_rem_content(tokens: Vec<Token>) -> Vec<Token> {
    let mut result = Vec::with_capacity(tokens.len());
    let mut suppressing = false;

    for tok in tokens {
        // Step 1: If we are currently suppressing, check if this token is a
        // NEWLINE — if so, stop suppressing and keep the newline. Otherwise drop.
        if suppressing {
            if tok.type_ == TokenType::Newline {
                // End of the REM line. Keep the NEWLINE and turn suppression off.
                suppressing = false;
                result.push(tok);
            }
            // Non-NEWLINE tokens while suppressing are silently dropped.
            continue;
        }

        // Step 2: Not suppressing — keep this token.
        result.push(tok.clone());

        // Step 3: If this is a REM keyword, start suppressing from the *next* token.
        if tok.type_ == TokenType::Keyword && tok.value == "REM" {
            suppressing = true;
        }
    }

    result
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for 1964 Dartmouth BASIC source text.
///
/// This function:
///
/// 1. **Reads** the `dartmouth_basic.tokens` grammar file from the shared
///    `code/grammars/` directory at the repository root.
/// 2. **Parses** the grammar text into a `TokenGrammar` struct using the
///    `grammar-tools` crate.
/// 3. **Constructs** a `GrammarLexer` with that grammar and the given source
///    text.
/// 4. **Registers** two post-tokenize hooks:
///    - [`relabel_line_numbers`] — fixes integer token types based on position.
///    - [`suppress_rem_content`] — removes comment text after REM.
///
/// The returned lexer is ready to call `.tokenize()` on. Use this when you
/// need the `GrammarLexer` object itself — for example, to chain it with other
/// transformations or to inspect individual tokens as they are produced.
///
/// # Panics
///
/// Panics if:
/// - The grammar file cannot be found or read. (This should never happen in a
///   correct checkout — the file is committed to the repository.)
/// - The grammar file cannot be parsed. (This would indicate a bug in the
///   grammar file itself, which the grammar-tools test suite would catch.)
///
/// # Example
///
/// ```no_run
/// use coding_adventures_dartmouth_basic_lexer::create_dartmouth_basic_lexer;
///
/// let mut lexer = create_dartmouth_basic_lexer("10 LET X = 42");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{:?} {:?}", token.type_, token.value);
/// }
/// ```
pub fn create_dartmouth_basic_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);

    // ----------------------------------------------------------------
    // Step 4: Register post-tokenize hooks.
    //
    // Hooks run in registration order AFTER the grammar engine has
    // finished producing all tokens. Each hook receives the complete
    // token list and returns a (possibly modified) list.
    //
    // Hook 1 — relabel_line_numbers:
    //   Fixes integer tokens that are not in line-number position.
    //   The grammar produces LINE_NUM for all bare integers (because
    //   LINE_NUM comes first). This hook relabels those that are NOT
    //   at the start of a line back to NUMBER.
    //
    // Hook 2 — suppress_rem_content:
    //   Removes tokens between REM and the end of its line. This
    //   implements the BASIC comment syntax.
    // ----------------------------------------------------------------
    lexer.add_post_tokenize(Box::new(relabel_line_numbers));
    lexer.add_post_tokenize(Box::new(suppress_rem_content));

    lexer
}

/// Tokenize 1964 Dartmouth BASIC source text into a vector of tokens.
///
/// This is the most convenient entry point. It handles grammar loading,
/// lexer construction, hook registration, and tokenization in one call.
/// The returned vector always ends with an `EOF` token.
///
/// # Token stream characteristics
///
/// - **NEWLINE tokens are included** — BASIC is line-oriented. The parser
///   uses NEWLINE as the statement terminator.
/// - **Whitespace is excluded** — spaces and tabs between tokens are consumed
///   silently and do not appear in the stream.
/// - **Keywords are uppercase** — because `@case_insensitive true` uppercases
///   the entire source before matching, `print` becomes `PRINT`.
/// - **LINE_NUM appears first on each line** — followed by the statement.
/// - **REM comments are stripped** — only the REM token itself survives.
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed, or if tokenization
/// fails internally. In a production setting you would want to propagate
/// errors via `Result`; for this educational codebase, panicking with a
/// clear message is simpler.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_dartmouth_basic_lexer::tokenize_dartmouth_basic;
///
/// let tokens = tokenize_dartmouth_basic("10 LET X = 42\n20 PRINT X\n30 END");
/// for token in &tokens {
///     println!("{} {:?}", token.effective_type_name(), token.value);
/// }
/// // Output (approximately):
/// //   LINE_NUM "10"
/// //   KEYWORD  "LET"
/// //   NAME     "X"
/// //   EQ       "="
/// //   NUMBER   "42"
/// //   NEWLINE  "\n"
/// //   LINE_NUM "20"
/// //   KEYWORD  "PRINT"
/// //   NAME     "X"
/// //   NEWLINE  "\n"
/// //   LINE_NUM "30"
/// //   KEYWORD  "END"
/// //   NEWLINE  "\n"
/// //   EOF      ""
/// ```
pub fn tokenize_dartmouth_basic(source: &str) -> Vec<Token> {
    let mut lexer = create_dartmouth_basic_lexer(source);

    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Dartmouth BASIC tokenization failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    /// Return (effective_type_name, value) pairs for all non-EOF tokens.
    ///
    /// `effective_type_name()` returns the grammar's string name for custom
    /// token types (like LINE_NUM, BUILTIN_FN, USER_FN) and the built-in
    /// name for standard types (like "KEYWORD", "NUMBER", "NEWLINE").
    ///
    /// We strip EOF from almost all tests because we care about the token
    /// content, not the sentinel at the end.
    fn pairs(tokens: &[Token]) -> Vec<(&str, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.effective_type_name(), t.value.as_str()))
            .collect()
    }

    /// Like `pairs()` but also excludes NEWLINE tokens for tests where
    /// newlines are unimportant (e.g., testing a single expression).
    fn pairs_no_newline(tokens: &[Token]) -> Vec<(&str, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof && t.type_ != TokenType::Newline)
            .map(|t| (t.effective_type_name(), t.value.as_str()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: Basic LET statement
    //
    // The simplest meaningful BASIC statement: assign a number to a variable.
    // Verifies LINE_NUM, KEYWORD, NAME, EQ, NUMBER, NEWLINE.
    // -----------------------------------------------------------------------

    /// `10 LET X = 5` is the "hello world" of BASIC — assign 5 to X on line 10.
    ///
    /// After the post-tokenize hooks:
    /// - `10` is LINE_NUM (first number on its line → line label)
    /// - `5` is NUMBER (not at line start → expression value)
    #[test]
    fn test_let_statement() {
        let tokens = tokenize_dartmouth_basic("10 LET X = 5\n");
        let p = pairs(&tokens);

        assert_eq!(p[0], ("LINE_NUM", "10"), "Expected LINE_NUM(10)");
        assert_eq!(p[1], ("KEYWORD", "LET"), "Expected KEYWORD(LET)");
        assert_eq!(p[2], ("NAME", "X"), "Expected NAME(X)");
        assert_eq!(p[3].0, "EQ", "Expected EQ(=)");
        assert_eq!(p[4], ("NUMBER", "5"), "Expected NUMBER(5)");
        assert_eq!(p[5].0, "NEWLINE", "Expected NEWLINE");
    }

    // -----------------------------------------------------------------------
    // Test 2: PRINT with comma separator
    //
    // PRINT X, Y prints X, advances to the next print zone (column multiple
    // of 14), then prints Y. The comma is syntactically meaningful.
    // -----------------------------------------------------------------------

    #[test]
    fn test_print_comma() {
        let tokens = tokenize_dartmouth_basic("20 PRINT X, Y\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[0], ("LINE_NUM", "20"));
        assert_eq!(p[1], ("KEYWORD", "PRINT"));
        assert_eq!(p[2], ("NAME", "X"));
        assert_eq!(p[3].0, "COMMA");
        assert_eq!(p[4], ("NAME", "Y"));
    }

    // -----------------------------------------------------------------------
    // Test 3: GOTO — GOTO target becomes NUMBER, not LINE_NUM
    //
    // This is the key test for the relabel_line_numbers hook.
    // `30 GOTO 10` has:
    //   - `30` at line start → LINE_NUM (the label for this line)
    //   - `10` not at line start → NUMBER (the target branch address)
    //
    // Before the hook, both `30` and `10` would be LINE_NUM. After the hook,
    // only `30` remains LINE_NUM.
    // -----------------------------------------------------------------------

    #[test]
    fn test_goto_target_is_number() {
        let tokens = tokenize_dartmouth_basic("30 GOTO 10\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[0], ("LINE_NUM", "30"), "30 is the line label");
        assert_eq!(p[1], ("KEYWORD", "GOTO"));
        assert_eq!(
            p[2],
            ("NUMBER", "10"),
            "GOTO target must be NUMBER, not LINE_NUM"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: IF … THEN with comparison
    //
    // Tests relational operators and the THEN keyword.
    // -----------------------------------------------------------------------

    #[test]
    fn test_if_then() {
        let tokens = tokenize_dartmouth_basic("40 IF X > 0 THEN 100\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[0], ("LINE_NUM", "40"));
        assert_eq!(p[1], ("KEYWORD", "IF"));
        assert_eq!(p[2], ("NAME", "X"));
        assert_eq!(p[3].0, "GT");
        assert_eq!(p[4], ("NUMBER", "0"));
        assert_eq!(p[5], ("KEYWORD", "THEN"));
        assert_eq!(p[6], ("NUMBER", "100"));
    }

    // -----------------------------------------------------------------------
    // Test 5: FOR loop
    //
    // FOR I = 1 TO 10 STEP 2 exercises several keywords.
    // -----------------------------------------------------------------------

    #[test]
    fn test_for_loop() {
        let tokens = tokenize_dartmouth_basic("50 FOR I = 1 TO 10 STEP 2\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[0], ("LINE_NUM", "50"));
        assert_eq!(p[1], ("KEYWORD", "FOR"));
        assert_eq!(p[2], ("NAME", "I"));
        assert_eq!(p[3].0, "EQ");
        assert_eq!(p[4], ("NUMBER", "1"));
        assert_eq!(p[5], ("KEYWORD", "TO"));
        assert_eq!(p[6], ("NUMBER", "10"));
        assert_eq!(p[7], ("KEYWORD", "STEP"));
        assert_eq!(p[8], ("NUMBER", "2"));
    }

    // -----------------------------------------------------------------------
    // Test 6: DEF with user function
    //
    // `DEF FNA(X) = X * X` defines a user function FNA.
    // USER_FN tokens are "FN" followed by exactly one letter.
    // -----------------------------------------------------------------------

    #[test]
    fn test_def_user_function() {
        let tokens = tokenize_dartmouth_basic("60 DEF FNA(X) = X * X\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[0], ("LINE_NUM", "60"));
        assert_eq!(p[1], ("KEYWORD", "DEF"));
        assert_eq!(p[2], ("USER_FN", "FNA"));
        assert_eq!(p[3].0, "LPAREN");
        assert_eq!(p[4], ("NAME", "X"));
        assert_eq!(p[5].0, "RPAREN");
        assert_eq!(p[6].0, "EQ");
        assert_eq!(p[7], ("NAME", "X"));
        assert_eq!(p[8].0, "STAR");
        assert_eq!(p[9], ("NAME", "X"));
    }

    // -----------------------------------------------------------------------
    // Test 7: Built-in functions SIN and COS
    //
    // `LET Y = SIN(X) + COS(X)` uses two of the 11 built-in functions.
    // BUILTIN_FN tokens must appear before NAME tokens in the grammar
    // so that `SIN` is not mistaken for a 3-letter identifier.
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_functions_sin_cos() {
        let tokens = tokenize_dartmouth_basic("70 LET Y = SIN(X) + COS(X)\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[0], ("LINE_NUM", "70"));
        assert_eq!(p[1], ("KEYWORD", "LET"));
        assert_eq!(p[2], ("NAME", "Y"));
        assert_eq!(p[3].0, "EQ");
        assert_eq!(p[4], ("BUILTIN_FN", "SIN"));
        assert_eq!(p[5].0, "LPAREN");
        assert_eq!(p[6], ("NAME", "X"));
        assert_eq!(p[7].0, "RPAREN");
        assert_eq!(p[8].0, "PLUS");
        assert_eq!(p[9], ("BUILTIN_FN", "COS"));
        assert_eq!(p[10].0, "LPAREN");
        assert_eq!(p[11], ("NAME", "X"));
        assert_eq!(p[12].0, "RPAREN");
    }

    // -----------------------------------------------------------------------
    // Test 8: Case insensitivity — lowercase input
    //
    // `10 print x` should produce the same tokens as `10 PRINT X`.
    // The grammar uses `case_sensitive: false`, which lowercases the entire
    // source before matching. This means:
    //   - KEYWORD values are uppercased by the keyword-promotion logic (PRINT)
    //   - NAME values are lowercased (x, not X) because the source was lowercased
    //   - LINE_NUM values remain as-is (digits are the same in either case)
    //
    // The test verifies that token TYPES match, and that KEYWORD values are
    // uppercased. Variable names are stored lowercased in this Rust implementation
    // because case_sensitive: false lowercases the source before lexing.
    // -----------------------------------------------------------------------

    #[test]
    fn test_case_insensitive_print() {
        let lower = tokenize_dartmouth_basic("10 print x\n");
        let upper = tokenize_dartmouth_basic("10 PRINT X\n");
        let p_lower = pairs_no_newline(&lower);
        let p_upper = pairs_no_newline(&upper);

        assert_eq!(p_lower.len(), p_upper.len(), "Same number of tokens");
        // Token types must match (LINE_NUM, KEYWORD, NAME)
        for (l, u) in p_lower.iter().zip(p_upper.iter()) {
            assert_eq!(l.0, u.0, "Token type should match: {:?} vs {:?}", l, u);
        }
        // Keywords are promoted to uppercase regardless of source case
        assert_eq!(
            p_lower[1],
            ("KEYWORD", "PRINT"),
            "PRINT should be uppercase keyword"
        );
    }

    /// `20 Let A = 1` — mixed-case keyword.
    #[test]
    fn test_case_insensitive_let() {
        let tokens = tokenize_dartmouth_basic("20 Let A = 1\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[1], ("KEYWORD", "LET"), "LET should be uppercased");
        assert_eq!(p[2], ("NAME", "A"));
    }

    /// `30 goto 20` — lowercase keyword and number.
    #[test]
    fn test_case_insensitive_goto() {
        let tokens = tokenize_dartmouth_basic("30 goto 20\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[1], ("KEYWORD", "GOTO"), "GOTO should be uppercased");
        assert_eq!(p[2], ("NUMBER", "20"), "Branch target should be NUMBER");
    }

    // -----------------------------------------------------------------------
    // Test 9: Two-character operators — must not split into single chars
    //
    // The grammar lists LE, GE, NE before LT, GT, EQ to ensure the longer
    // match wins. Without this ordering:
    //   `<=` would lex as LT("=") then EQ("=") — wrong!
    //   `>=` would lex as GT(">") then EQ("=") — wrong!
    //   `<>` would lex as LT("<") then GT(">") — wrong!
    // -----------------------------------------------------------------------

    #[test]
    fn test_operator_le_not_lt_eq() {
        let tokens = tokenize_dartmouth_basic("10 IF X <= Y THEN 50\n");
        let p = pairs_no_newline(&tokens);

        // Find the operator token — should be LE, not two separate tokens
        let op_idx = 3;
        assert_eq!(
            p[op_idx],
            ("LE", "<="),
            "Expected LE token, got {:?}",
            p[op_idx]
        );
        // Verify total token count: LINE_NUM IF X LE Y THEN NUMBER = 7
        assert_eq!(p.len(), 7, "Expected 7 tokens: {:?}", p);
    }

    #[test]
    fn test_operator_ge_not_gt_eq() {
        let tokens = tokenize_dartmouth_basic("10 IF X >= Y THEN 50\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[3], ("GE", ">="), "Expected GE token, got {:?}", p[3]);
        assert_eq!(p.len(), 7);
    }

    #[test]
    fn test_operator_ne_not_lt_gt() {
        let tokens = tokenize_dartmouth_basic("10 IF X <> Y THEN 50\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[3], ("NE", "<>"), "Expected NE token, got {:?}", p[3]);
        assert_eq!(p.len(), 7);
    }

    // -----------------------------------------------------------------------
    // Test 10: Number literal formats
    //
    // Dartmouth BASIC supports several numeric formats:
    //   42      integer (stored as float internally)
    //   3.14    decimal
    //   .5      leading-dot decimal (no integer part)
    //   1.5E3   scientific notation (= 1500.0)
    //   1.5E-3  negative exponent (= 0.0015)
    //   1E10    integer+exponent (no decimal point)
    // -----------------------------------------------------------------------

    #[test]
    fn test_number_integer() {
        let tokens = tokenize_dartmouth_basic("10 LET X = 42\n");
        let p = pairs_no_newline(&tokens);
        assert_eq!(p[4], ("NUMBER", "42"));
    }

    #[test]
    fn test_number_decimal() {
        let tokens = tokenize_dartmouth_basic("10 LET X = 3.14\n");
        let p = pairs_no_newline(&tokens);
        assert_eq!(p[4], ("NUMBER", "3.14"));
    }

    #[test]
    fn test_number_leading_dot() {
        let tokens = tokenize_dartmouth_basic("10 LET X = .5\n");
        let p = pairs_no_newline(&tokens);
        assert_eq!(
            p[4],
            ("NUMBER", ".5"),
            "Leading-dot number should tokenize as NUMBER"
        );
    }

    #[test]
    fn test_number_scientific() {
        let tokens = tokenize_dartmouth_basic("10 LET X = 1.5E3\n");
        let p = pairs_no_newline(&tokens);
        assert_eq!(p[4], ("NUMBER", "1.5E3"));
    }

    #[test]
    fn test_number_negative_exponent() {
        let tokens = tokenize_dartmouth_basic("10 LET X = 1.5E-3\n");
        let p = pairs_no_newline(&tokens);
        assert_eq!(
            p[4],
            ("NUMBER", "1.5E-3"),
            "Negative exponent should tokenize as single NUMBER token"
        );
    }

    #[test]
    fn test_number_integer_exponent() {
        let tokens = tokenize_dartmouth_basic("10 LET X = 1E10\n");
        let p = pairs_no_newline(&tokens);
        assert_eq!(p[4], ("NUMBER", "1E10"));
    }

    // -----------------------------------------------------------------------
    // Test 11: String literals
    //
    // Strings in Dartmouth BASIC are delimited by double quotes.
    // The 1964 spec has no escape sequences — a double quote cannot appear
    // inside a string. The grammar aliases STRING_BODY → STRING.
    // -----------------------------------------------------------------------

    #[test]
    fn test_string_literal() {
        let tokens = tokenize_dartmouth_basic("10 PRINT \"HELLO WORLD\"\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[0], ("LINE_NUM", "10"));
        assert_eq!(p[1], ("KEYWORD", "PRINT"));
        assert_eq!(p[2].0, "STRING", "Expected STRING token type");
        assert!(
            p[2].1.contains("HELLO WORLD"),
            "String value should contain HELLO WORLD, got {:?}",
            p[2].1
        );
    }

    #[test]
    fn test_empty_string_literal() {
        let tokens = tokenize_dartmouth_basic("10 PRINT \"\"\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[2].0, "STRING", "Empty string should be STRING token");
    }

    // -----------------------------------------------------------------------
    // Test 12: REM comment handling
    //
    // Hook 2 (suppress_rem_content) removes everything between REM and
    // the end of the line. The REM keyword itself is kept, and the NEWLINE
    // is kept. Comment text tokens are dropped.
    // -----------------------------------------------------------------------

    /// A REM with following text — text should be suppressed.
    #[test]
    fn test_rem_comment_suppressed() {
        let tokens = tokenize_dartmouth_basic("10 REM THIS IS A COMMENT\n");
        let p = pairs(&tokens);

        // We should see: LINE_NUM("10"), KEYWORD("REM"), NEWLINE
        // The words THIS, IS, A, COMMENT should all be gone.
        assert_eq!(
            p.len(),
            3,
            "REM line should have 3 tokens (LINE_NUM, REM, NEWLINE), got: {:?}",
            p
        );
        assert_eq!(p[0], ("LINE_NUM", "10"));
        assert_eq!(p[1], ("KEYWORD", "REM"));
        assert_eq!(p[2].0, "NEWLINE");
    }

    /// A REM followed by another line — second line should be unaffected.
    #[test]
    fn test_rem_then_let() {
        let tokens = tokenize_dartmouth_basic("10 REM\n20 LET X = 1\n");
        let p = pairs(&tokens);

        // Line 10: LINE_NUM("10"), KEYWORD("REM"), NEWLINE
        // Line 20: LINE_NUM("20"), KEYWORD("LET"), NAME("X"), EQ, NUMBER("1"), NEWLINE
        assert_eq!(p[0], ("LINE_NUM", "10"));
        assert_eq!(p[1], ("KEYWORD", "REM"));
        assert_eq!(p[2].0, "NEWLINE");
        assert_eq!(p[3], ("LINE_NUM", "20"));
        assert_eq!(p[4], ("KEYWORD", "LET"));
        assert_eq!(p[5], ("NAME", "X"));
        assert_eq!(p[7], ("NUMBER", "1"));
    }

    // -----------------------------------------------------------------------
    // Test 13: Multi-line program
    //
    // Verifies that LINE_NUM and NEWLINE tokens work correctly across
    // multiple lines, and that the relabel_line_numbers hook correctly
    // identifies line starts after each NEWLINE.
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_line_program() {
        let source = "10 LET X = 1\n20 PRINT X\n30 END\n";
        let tokens = tokenize_dartmouth_basic(source);
        let p = pairs(&tokens);

        // Line 10: LINE_NUM LET NAME EQ NUMBER NEWLINE
        assert_eq!(p[0], ("LINE_NUM", "10"));
        assert_eq!(p[1], ("KEYWORD", "LET"));
        assert_eq!(p[4], ("NUMBER", "1"));

        // Line 20: LINE_NUM PRINT NAME NEWLINE
        assert_eq!(p[6], ("LINE_NUM", "20"));
        assert_eq!(p[7], ("KEYWORD", "PRINT"));

        // Line 30: LINE_NUM END NEWLINE
        assert_eq!(p[10], ("LINE_NUM", "30"));
        assert_eq!(p[11], ("KEYWORD", "END"));
    }

    // -----------------------------------------------------------------------
    // Test 14: Variable name formats
    //
    // Dartmouth BASIC 1964 allows exactly two variable name forms:
    //   - Single letter:  A, B, …, Z
    //   - Letter + digit: A0, A1, …, Z9
    //
    // The grammar rule NAME = /[A-Z][0-9]?/ captures both.
    // -----------------------------------------------------------------------

    #[test]
    fn test_variable_single_letter() {
        let tokens = tokenize_dartmouth_basic("10 LET X = 1\n");
        let p = pairs_no_newline(&tokens);
        assert_eq!(p[2], ("NAME", "X"), "Single-letter variable");
    }

    #[test]
    fn test_variable_letter_digit() {
        let tokens = tokenize_dartmouth_basic("10 LET A1 = 2\n");
        let p = pairs_no_newline(&tokens);
        assert_eq!(p[2], ("NAME", "A1"), "Letter+digit variable");
    }

    #[test]
    fn test_variable_z9() {
        let tokens = tokenize_dartmouth_basic("10 LET Z9 = 3\n");
        let p = pairs_no_newline(&tokens);
        assert_eq!(p[2], ("NAME", "Z9"), "Z9 is a valid BASIC variable name");
    }

    // -----------------------------------------------------------------------
    // Test 15: PRINT separators — comma vs semicolon
    //
    // BASIC's PRINT has two field separators with different spacing behaviors:
    //   COMMA     — advance to the next print zone (column multiple of 14)
    //   SEMICOLON — no space; print next item immediately
    //
    // Example: PRINT X, Y    → X[spaces]Y (print zones)
    //          PRINT X; Y    → XY (no space)
    // -----------------------------------------------------------------------

    #[test]
    fn test_print_semicolon() {
        let tokens = tokenize_dartmouth_basic("10 PRINT X; Y\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[3].0, "SEMICOLON", "Semicolon separator");
    }

    #[test]
    fn test_print_comma_is_comma() {
        let tokens = tokenize_dartmouth_basic("10 PRINT X, Y\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[3].0, "COMMA", "Comma separator");
    }

    // -----------------------------------------------------------------------
    // Test 16: All arithmetic operators
    // -----------------------------------------------------------------------

    #[test]
    fn test_arithmetic_operators() {
        let tokens = tokenize_dartmouth_basic("10 LET X = A + B - C * D / E ^ F\n");
        let p = pairs_no_newline(&tokens);

        // Find operators by position: after A, B, C, D, E
        let type_names: Vec<&str> = p.iter().map(|pair| pair.0).collect();
        assert!(type_names.contains(&"PLUS"), "PLUS operator");
        assert!(type_names.contains(&"MINUS"), "MINUS operator");
        assert!(type_names.contains(&"STAR"), "STAR operator");
        assert!(type_names.contains(&"SLASH"), "SLASH operator");
        assert!(
            type_names.contains(&"CARET"),
            "CARET (^) exponentiation operator"
        );
    }

    // -----------------------------------------------------------------------
    // Test 17: All 20 keywords present
    //
    // Every keyword in the 1964 spec must tokenize as KEYWORD, not NAME.
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_keywords() {
        // The 20 keywords from the 1964 Dartmouth BASIC spec.
        // We wrap each in a minimal valid BASIC line so the lexer
        // has context to work with.
        let keywords = [
            "LET", "PRINT", "INPUT", "IF", "THEN", "GOTO", "GOSUB", "RETURN", "FOR", "TO", "STEP",
            "NEXT", "END", "STOP", "REM", "READ", "DATA", "RESTORE", "DIM", "DEF",
        ];

        for kw in &keywords {
            // Use a fresh line to avoid keyword interaction side-effects.
            // We skip REM since it would suppress subsequent tokens.
            if *kw == "REM" {
                let tokens = tokenize_dartmouth_basic(&format!("10 {}\n", kw));
                let keyword_tokens: Vec<&Token> = tokens
                    .iter()
                    .filter(|t| t.type_ == TokenType::Keyword)
                    .collect();
                assert!(
                    keyword_tokens.iter().any(|t| t.value == "REM"),
                    "REM should be a KEYWORD token"
                );
                continue;
            }

            let tokens = tokenize_dartmouth_basic(&format!("10 {} X\n", kw));
            let keyword_tokens: Vec<&Token> = tokens
                .iter()
                .filter(|t| t.type_ == TokenType::Keyword)
                .collect();

            assert!(
                keyword_tokens.iter().any(|t| t.value == *kw),
                "Expected KEYWORD token for '{kw}', got: {:?}",
                keyword_tokens.iter().map(|t| &t.value).collect::<Vec<_>>()
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 18: All 11 built-in functions
    //
    // The 1964 spec defines exactly these functions:
    //   SIN, COS, TAN, ATN  — trigonometric
    //   EXP, LOG             — exponential / logarithm
    //   ABS, SQR, INT, SGN   — utility
    //   RND                  — random number
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_builtin_functions() {
        let builtins = [
            "SIN", "COS", "TAN", "ATN", "EXP", "LOG", "ABS", "SQR", "INT", "RND", "SGN",
        ];

        for func in &builtins {
            let source = format!("10 LET X = {}(X)\n", func);
            let tokens = tokenize_dartmouth_basic(&source);
            let p = pairs_no_newline(&tokens);

            let found = p
                .iter()
                .any(|(name, val)| *name == "BUILTIN_FN" && *val == *func);
            assert!(found, "Expected BUILTIN_FN({func}) in tokens: {:?}", p);
        }
    }

    // -----------------------------------------------------------------------
    // Test 19: User-defined functions FNA through FNZ
    //
    // FN followed by exactly one uppercase letter. The grammar rule is:
    //   USER_FN = /FN[A-Z]/
    // These must be tested to ensure they are not split into NAME("F") + NAME("N")
    // + NAME("A") or matched as NAME("FN") + NAME("A").
    // -----------------------------------------------------------------------

    #[test]
    fn test_user_function_fna() {
        let tokens = tokenize_dartmouth_basic("10 LET X = FNA(Y)\n");
        let p = pairs_no_newline(&tokens);

        let found = p
            .iter()
            .any(|(name, val)| *name == "USER_FN" && *val == "FNA");
        assert!(found, "Expected USER_FN(FNA): {:?}", p);
    }

    #[test]
    fn test_user_function_fnz() {
        let tokens = tokenize_dartmouth_basic("10 LET X = FNZ(Y)\n");
        let p = pairs_no_newline(&tokens);

        let found = p
            .iter()
            .any(|(name, val)| *name == "USER_FN" && *val == "FNZ");
        assert!(found, "Expected USER_FN(FNZ): {:?}", p);
    }

    // -----------------------------------------------------------------------
    // Test 20: Unknown character handling
    //
    // The grammar's `errors: UNKNOWN = /./` section is not currently
    // implemented in the Rust GrammarLexer — unrecognized characters cause
    // the lexer to return an error rather than emit an UNKNOWN token.
    //
    // This test verifies the actual Rust behavior: tokenization returns an
    // error result when an invalid character like `@` is encountered.
    //
    // Other implementations (Elixir, Python, Ruby) DO support the errors:
    // section and emit UNKNOWN tokens for error recovery. The Rust impl
    // may be updated in the future to match.
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_recovery_unknown_char() {
        // `@` is not a valid BASIC character; in Rust the GrammarLexer
        // returns an error rather than emitting UNKNOWN tokens.
        let mut lexer = create_dartmouth_basic_lexer("10 LET @ = 1\n");
        let result = lexer.tokenize();

        // The Rust lexer returns Err for unrecognized characters.
        assert!(
            result.is_err(),
            "Expected tokenization error for unrecognized '@' character, got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Test 21: EQ is used for both assignment and comparison
    //
    // In BASIC, `=` serves two roles:
    //   LET X = 5    — assignment (LET context)
    //   IF X = 5 THEN — equality comparison (IF context)
    //
    // Both produce EQ tokens. The parser resolves ambiguity by context.
    // -----------------------------------------------------------------------

    #[test]
    fn test_eq_token_in_let() {
        let tokens = tokenize_dartmouth_basic("10 LET X = 5\n");
        let p = pairs_no_newline(&tokens);
        assert_eq!(p[3].0, "EQ", "LET uses EQ for assignment");
    }

    #[test]
    fn test_eq_token_in_if() {
        let tokens = tokenize_dartmouth_basic("10 IF X = 5 THEN 100\n");
        let p = pairs_no_newline(&tokens);
        let eq_idx = p.iter().position(|(name, _)| *name == "EQ");
        assert!(eq_idx.is_some(), "IF uses EQ for equality comparison");
    }

    // -----------------------------------------------------------------------
    // Test 22: GOSUB and RETURN
    //
    // GOSUB is BASIC's subroutine call — similar to a function call.
    // RETURN exits back to the line after the GOSUB.
    // -----------------------------------------------------------------------

    #[test]
    fn test_gosub_return() {
        let source = "10 GOSUB 500\n20 RETURN\n";
        let tokens = tokenize_dartmouth_basic(source);
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[1], ("KEYWORD", "GOSUB"));
        assert_eq!(p[2], ("NUMBER", "500"));
        assert_eq!(p[4], ("KEYWORD", "RETURN"));
    }

    // -----------------------------------------------------------------------
    // Test 23: STOP and END
    //
    // Both stop program execution; END is the normal terminal, STOP is an
    // early exit (like break).
    // -----------------------------------------------------------------------

    #[test]
    fn test_stop_and_end_keywords() {
        let tokens_stop = tokenize_dartmouth_basic("10 STOP\n");
        let p_stop = pairs_no_newline(&tokens_stop);
        assert_eq!(p_stop[1], ("KEYWORD", "STOP"));

        let tokens_end = tokenize_dartmouth_basic("10 END\n");
        let p_end = pairs_no_newline(&tokens_end);
        assert_eq!(p_end[1], ("KEYWORD", "END"));
    }

    // -----------------------------------------------------------------------
    // Test 24: DATA, READ, RESTORE
    //
    // These keywords implement BASIC's data list feature:
    //   DATA 1, 2, 3    — stores values in a data list
    //   READ X          — reads the next value from the list into X
    //   RESTORE         — resets the data list pointer to the beginning
    // -----------------------------------------------------------------------

    #[test]
    fn test_data_read_restore() {
        let source = "10 DATA 1, 2, 3\n20 READ X\n30 RESTORE\n";
        let tokens = tokenize_dartmouth_basic(source);
        let p = pairs_no_newline(&tokens);

        assert!(p
            .iter()
            .any(|(name, val)| *name == "KEYWORD" && *val == "DATA"));
        assert!(p
            .iter()
            .any(|(name, val)| *name == "KEYWORD" && *val == "READ"));
        assert!(p
            .iter()
            .any(|(name, val)| *name == "KEYWORD" && *val == "RESTORE"));
    }

    // -----------------------------------------------------------------------
    // Test 25: DIM — dimension declaration
    //
    // DIM A(10) declares a 10-element array named A.
    // -----------------------------------------------------------------------

    #[test]
    fn test_dim_keyword() {
        let tokens = tokenize_dartmouth_basic("10 DIM A(10)\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[1], ("KEYWORD", "DIM"));
        assert_eq!(p[2], ("NAME", "A"));
        assert_eq!(p[3].0, "LPAREN");
        assert_eq!(p[4], ("NUMBER", "10"));
        assert_eq!(p[5].0, "RPAREN");
    }

    // -----------------------------------------------------------------------
    // Test 26: NEXT keyword (end of FOR loop)
    //
    // FOR I = 1 TO 10 … NEXT I is the loop structure.
    // -----------------------------------------------------------------------

    #[test]
    fn test_next_keyword() {
        let tokens = tokenize_dartmouth_basic("10 NEXT I\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[1], ("KEYWORD", "NEXT"));
        assert_eq!(p[2], ("NAME", "I"));
    }

    // -----------------------------------------------------------------------
    // Test 27: INPUT keyword
    //
    // INPUT X reads a value from the user and stores it in X.
    // -----------------------------------------------------------------------

    #[test]
    fn test_input_keyword() {
        let tokens = tokenize_dartmouth_basic("10 INPUT X\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[1], ("KEYWORD", "INPUT"));
        assert_eq!(p[2], ("NAME", "X"));
    }

    // -----------------------------------------------------------------------
    // Test 28: Whitespace handling — spaces and tabs
    //
    // Horizontal whitespace (spaces and tabs) between tokens is silently
    // consumed. This means `10 LET X=5` and `10  LET  X  =  5` produce
    // identical token streams.
    // -----------------------------------------------------------------------

    #[test]
    fn test_whitespace_ignored() {
        let compact = tokenize_dartmouth_basic("10 LET X=5\n");
        let spaced = tokenize_dartmouth_basic("10  LET  X  =  5\n");
        let tabbed = tokenize_dartmouth_basic("10\tLET\tX\t=\t5\n");

        let p_compact = pairs(&compact);
        let p_spaced = pairs(&spaced);
        let p_tabbed = pairs(&tabbed);

        assert_eq!(
            p_compact.len(),
            p_spaced.len(),
            "Compact and spaced should have the same number of tokens"
        );
        assert_eq!(
            p_compact.len(),
            p_tabbed.len(),
            "Compact and tabbed should have the same number of tokens"
        );

        for (i, (c, s)) in p_compact.iter().zip(p_spaced.iter()).enumerate() {
            assert_eq!(c.0, s.0, "Token {i} type mismatch");
            assert_eq!(c.1, s.1, "Token {i} value mismatch");
        }
    }

    // -----------------------------------------------------------------------
    // Test 29: Factory function returns a working lexer
    //
    // The `create_dartmouth_basic_lexer` function should return a GrammarLexer
    // that tokenizes correctly when `.tokenize()` is called on it.
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_lexer_factory() {
        let mut lexer = create_dartmouth_basic_lexer("10 LET X = 42\n");
        let tokens = lexer
            .tokenize()
            .expect("Lexer should tokenize without error");

        // Should end with EOF
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);

        // Should contain LINE_NUM("10")
        let has_line_num = tokens
            .iter()
            .any(|t| t.effective_type_name() == "LINE_NUM" && t.value == "10");
        assert!(has_line_num, "Expected LINE_NUM(10) in output");
    }

    // -----------------------------------------------------------------------
    // Test 30: NEWLINE tokens are present (significant in BASIC)
    //
    // Unlike most languages where whitespace is skipped, BASIC keeps NEWLINE
    // tokens because they are statement terminators.
    // -----------------------------------------------------------------------

    #[test]
    fn test_newline_tokens_present() {
        let tokens = tokenize_dartmouth_basic("10 LET X = 1\n20 PRINT X\n");
        let newline_count = tokens
            .iter()
            .filter(|t| t.type_ == TokenType::Newline)
            .count();

        assert_eq!(
            newline_count, 2,
            "Expected 2 NEWLINE tokens (one per statement), found {}",
            newline_count
        );
    }

    // -----------------------------------------------------------------------
    // Test 31: EOF token always present
    //
    // Every token stream ends with exactly one EOF sentinel. This gives
    // the parser a clean termination condition without checking length.
    // -----------------------------------------------------------------------

    #[test]
    fn test_eof_always_present() {
        let tokens = tokenize_dartmouth_basic("10 END\n");
        assert_eq!(
            tokens.last().unwrap().type_,
            TokenType::Eof,
            "Last token must be EOF"
        );
    }

    // -----------------------------------------------------------------------
    // Test 32: LT and GT individual operators
    // -----------------------------------------------------------------------

    #[test]
    fn test_lt_gt_individual() {
        let tokens_lt = tokenize_dartmouth_basic("10 IF X < Y THEN 50\n");
        let p_lt = pairs_no_newline(&tokens_lt);
        assert_eq!(p_lt[3].0, "LT", "Expected LT operator");

        let tokens_gt = tokenize_dartmouth_basic("10 IF X > Y THEN 50\n");
        let p_gt = pairs_no_newline(&tokens_gt);
        assert_eq!(p_gt[3].0, "GT", "Expected GT operator");
    }

    // -----------------------------------------------------------------------
    // Test 33: CARET exponentiation operator
    //
    // `^` is exponentiation in BASIC. 2^3 = 8.
    // Right-associative: 2^3^2 = 2^(3^2) = 512.
    // -----------------------------------------------------------------------

    #[test]
    fn test_caret_exponentiation() {
        let tokens = tokenize_dartmouth_basic("10 LET X = 2 ^ 3\n");
        let p = pairs_no_newline(&tokens);

        let has_caret = p.iter().any(|(name, val)| *name == "CARET" && *val == "^");
        assert!(has_caret, "Expected CARET(^) for exponentiation: {:?}", p);
    }

    // -----------------------------------------------------------------------
    // Test 34: Line number zero is valid
    //
    // Although unusual, line 0 is syntactically valid. Verify it is labeled
    // LINE_NUM, not NUMBER.
    // -----------------------------------------------------------------------

    #[test]
    fn test_line_number_zero() {
        let tokens = tokenize_dartmouth_basic("0 END\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[0], ("LINE_NUM", "0"), "Line number 0 should be LINE_NUM");
    }

    // -----------------------------------------------------------------------
    // Test 35: Large line numbers
    //
    // BASIC programs can have line numbers like 9999 or even 65535.
    // -----------------------------------------------------------------------

    #[test]
    fn test_large_line_number() {
        let tokens = tokenize_dartmouth_basic("9999 END\n");
        let p = pairs_no_newline(&tokens);

        assert_eq!(p[0], ("LINE_NUM", "9999"), "9999 should be LINE_NUM");
    }
}
