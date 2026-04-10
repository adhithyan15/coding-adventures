// ============================================================================
// DartmouthBasicLexer.swift -- Dartmouth BASIC (1964) Lexer
// ============================================================================
//
// A thin wrapper around the grammar-driven GrammarLexer from the Lexer
// package, configured by the shared dartmouth_basic.tokens grammar file.
//
// ============================================================================
// WHAT IS DARTMOUTH BASIC?
// ============================================================================
//
// Dartmouth BASIC was invented in 1964 by mathematicians John G. Kemeny and
// Thomas E. Kurtz at Dartmouth College. Their goal: give non-specialist
// students access to the college's mainframe. Before BASIC, using a computer
// meant punching cards and waiting hours for batch results. BASIC gave users
// a teletype terminal and an interactive experience — you typed a command,
// pressed Return, and got an answer.
//
// The language was deliberately simple:
//
//   - Every statement lives on a numbered line. Line numbers serve two roles:
//     (a) they control execution order (ascending), and (b) they are targets
//     for GOTO and GOSUB jumps.
//
//   - The alphabet is uppercase only — teletypes in 1964 had no lowercase.
//     This makes the language case-insensitive, which the lexer normalises
//     by uppercasing all input before matching.
//
//   - Variables are single letters (A–Z) or letter+digit (A0–Z9). All 286
//     possible variables are pre-initialised to 0; no declaration is needed.
//
//   - Numbers are all floating-point internally (no integer vs. float split).
//
// ============================================================================
// THE TOKEN STREAM
// ============================================================================
//
// A Dartmouth BASIC source file looks like this:
//
//   10 LET X = 5
//   20 PRINT X
//   30 END
//
// After lexing, the token stream is (one line shown per row):
//
//   LINE_NUM("10")  KEYWORD("LET")   NAME("x")    EQ("=")  NUMBER("5")  NEWLINE
//   LINE_NUM("20")  KEYWORD("PRINT") NAME("x")             NEWLINE
//   LINE_NUM("30")  KEYWORD("END")                          NEWLINE
//   EOF
//
// The NEWLINE token is significant: it is the statement terminator. Unlike
// most languages, whitespace (horizontal) is skipped but newlines are kept.
//
// ============================================================================
// TWO POST-TOKENIZATION PASSES
// ============================================================================
//
// The GrammarLexer handles the mechanical work of pattern-matching. After it
// finishes, two language-specific passes clean up the token stream:
//
// PASS 1 — relabelLineNumbers
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~
//   In BASIC, every line starts with a number. That number is the line label,
//   not an arithmetic value. The grammar cannot distinguish them — both look
//   like digit sequences. So the raw lexer emits NUMBER for all of them.
//
//   This pass walks the token list. It maintains a boolean "at line start".
//   After each NEWLINE (or at the very start), the next NUMBER it encounters
//   is relabelled as LINE_NUM.
//
//   Example:
//     Raw:    NUMBER("10")  KEYWORD("LET")  NAME("x")  EQ  NUMBER("5")  NEWLINE
//     After:  LINE_NUM("10") KEYWORD("LET") NAME("x")  EQ  NUMBER("5")  NEWLINE
//                ↑ relabelled                                ↑ stays NUMBER
//
// PASS 2 — suppressRemContent
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~
//   REM (remark) is BASIC's comment syntax. After the REM keyword, everything
//   up to and including the NEWLINE is a comment. The grammar file's comment
//   about this:
//
//     "The lexer suppresses all tokens between REM and NEWLINE."
//
//   This pass removes any tokens that appear between a KEYWORD("REM") and the
//   following NEWLINE. It does NOT remove the REM keyword itself or the NEWLINE
//   — those stay so the parser can produce a `rem_stmt` node cleanly.
//
//   Example:
//     Raw:    LINE_NUM("10")  KEYWORD("REM")  NAME("hello")  NAME("world")  NEWLINE
//     After:  LINE_NUM("10")  KEYWORD("REM")                                NEWLINE
//                                              ↑ removed
//
// ============================================================================
// USAGE
// ============================================================================
//
//     let tokens = try DartmouthBasicLexer.tokenize("10 LET X = 5\n")
//     // tokens[0].type == "LINE_NUM", tokens[0].value == "10"
//     // tokens[1].type == "KEYWORD",  tokens[1].value == "LET"
//     // tokens[2].type == "NAME",     tokens[2].value == "x"  (lowercased)
//     // tokens[3].type == "EQ",       tokens[3].value == "="
//     // tokens[4].type == "NUMBER",   tokens[4].value == "5"
//     // tokens[5].type == "NEWLINE",  tokens[5].value == "\\n"
//     // tokens[6].type == "EOF"
//
// ============================================================================

import Foundation
import GrammarTools
import Lexer

/// Dartmouth BASIC (1964) lexer — tokenizes BASIC source code.
///
/// This struct provides a static `tokenize(_:)` method that:
///   1. Loads `dartmouth_basic.tokens` from the monorepo `grammars/` directory
///   2. Constructs a `GrammarLexer` to produce the raw token stream
///   3. Applies two post-processing passes (relabelLineNumbers + suppressRemContent)
///   4. Returns the cleaned token stream
///
/// All methods are static and `Sendable` — the struct itself has no stored
/// state, making it safe to use from concurrent contexts.
///
public struct DartmouthBasicLexer: Sendable {

    /// The version of this lexer package.
    ///
    /// Follows semantic versioning: MAJOR.MINOR.PATCH.
    /// MAJOR: breaking API changes; MINOR: new features; PATCH: bug fixes.
    ///
    public static let version = "0.1.0"

    // =========================================================================
    // MARK: - Public API
    // =========================================================================

    /// Tokenize a Dartmouth BASIC source string.
    ///
    /// The input may use any combination of upper and lowercase letters — the
    /// grammar's `case_sensitive: false` directive tells the GrammarLexer to
    /// normalise input to lowercase before matching. Keywords are then emitted
    /// with their uppercase value (LET, PRINT, etc.) by the post-process pass.
    ///
    /// - Parameter source: The BASIC source code to tokenize.
    /// - Returns: An array of `Token` values, ending with an EOF token.
    ///            Includes NEWLINE tokens (significant in BASIC).
    /// - Throws: `LexerError` on unexpected characters.
    ///
    public static func tokenize(_ source: String) throws -> [Token] {
        let grammar = try loadGrammar()
        let lexer = GrammarLexer(source: source, grammar: grammar)
        var tokens = try lexer.tokenize()

        // Pass 1: Relabel the first NUMBER on each source line as LINE_NUM.
        // Without this pass, "10 LET X = 5" would have two NUMBER tokens
        // (10 and 5) instead of a LINE_NUM and a NUMBER.
        tokens = relabelLineNumbers(tokens)

        // Pass 2: Remove comment content after REM.
        // Without this pass, "10 REM HELLO WORLD" would produce tokens for
        // "HELLO" and "WORLD", confusing the parser.
        tokens = suppressRemContent(tokens)

        return tokens
    }

    /// Load and parse the Dartmouth BASIC token grammar.
    ///
    /// The grammar file path is computed relative to this source file:
    ///
    ///   DartmouthBasicLexer.swift          ← this file
    ///     Sources/DartmouthBasicLexer/     (1) strip filename
    ///     dartmouth-basic-lexer/           (2)
    ///     swift/                           (3)
    ///     packages/                        (4)
    ///     code/                            (5)  → grammars/dartmouth_basic.tokens
    ///
    /// The `#filePath` directive resolves to the compile-time path of this
    /// source file inside the monorepo. Walking up 5 levels plus the filename
    /// level reaches the `code/` directory, then we descend into `grammars/`.
    ///
    /// - Returns: A `TokenGrammar` parsed from `dartmouth_basic.tokens`.
    /// - Throws: If the file cannot be read or the grammar cannot be parsed.
    ///
    public static func loadGrammar() throws -> TokenGrammar {
        let thisFile = #filePath
        var url = URL(fileURLWithPath: thisFile)
        // Walk up 6 path components:
        //   DartmouthBasicLexer.swift  → Sources/DartmouthBasicLexer
        //   Sources/DartmouthBasicLexer → Sources
        //   Sources → dartmouth-basic-lexer
        //   dartmouth-basic-lexer → swift
        //   swift → packages
        //   packages → code
        for _ in 0..<6 {
            url = url.deletingLastPathComponent()
        }
        let tokensURL = url
            .appendingPathComponent("grammars")
            .appendingPathComponent("dartmouth_basic.tokens")

        let content = try String(contentsOf: tokensURL, encoding: .utf8)
        return try parseTokenGrammar(source: content)
    }

    // =========================================================================
    // MARK: - Post-processing Pass 1: Relabel Line Numbers
    // =========================================================================

    /// Promote the first NUMBER token on each logical line to LINE_NUM.
    ///
    /// In Dartmouth BASIC, every program line begins with an integer line
    /// number. The grammar cannot distinguish this number from arithmetic
    /// literals — both match the same digit sequence pattern. This pass
    /// resolves the ambiguity using positional context:
    ///
    ///   - The very first NUMBER in the token stream (before any NEWLINE)
    ///     is a line number.
    ///   - After each NEWLINE, the next NUMBER is a line number.
    ///   - All other NUMBERs are arithmetic literals.
    ///
    /// Algorithm:
    ///   1. Start with `atLineStart = true` (the beginning of input counts).
    ///   2. Walk tokens left-to-right.
    ///   3. When we see NUMBER and `atLineStart` is true:
    ///      - Copy the token with type changed to "LINE_NUM"
    ///      - Set `atLineStart = false` (we've consumed the line label)
    ///   4. When we see NEWLINE: set `atLineStart = true`.
    ///   5. All other tokens pass through unchanged.
    ///
    /// - Parameter tokens: The raw token stream from GrammarLexer.
    /// - Returns: A new token array with line-number NUMBERs relabelled.
    ///
    static func relabelLineNumbers(_ tokens: [Token]) -> [Token] {
        var result: [Token] = []
        result.reserveCapacity(tokens.count)

        // `atLineStart` tracks whether we're looking for a line label.
        // It starts true because the very beginning of the file is a "line start".
        var atLineStart = true

        for token in tokens {
            switch token.type {
            case "NEWLINE":
                // A NEWLINE resets us to line-start position.
                // The next NUMBER we see will be the next line's label.
                atLineStart = true
                result.append(token)

            case "NUMBER" where atLineStart:
                // This is the first NUMBER after a NEWLINE (or at program start).
                // It's the line label — relabel it LINE_NUM.
                result.append(Token(
                    type: "LINE_NUM",
                    value: token.value,
                    line: token.line,
                    column: token.column,
                    flags: token.flags
                ))
                atLineStart = false

            default:
                // Any other token: pass through, mark that we're no longer
                // at line start (so subsequent NUMBERs stay as NUMBER).
                if token.type != "NEWLINE" && token.type != "EOF" {
                    atLineStart = false
                }
                result.append(token)
            }
        }

        return result
    }

    // =========================================================================
    // MARK: - Post-processing Pass 2: Suppress REM Content
    // =========================================================================

    /// Remove all tokens between a REM keyword and the following NEWLINE.
    ///
    /// REM (remark) is Dartmouth BASIC's comment syntax. The grammar file
    /// notes: "The lexer suppresses all tokens between REM and NEWLINE."
    ///
    /// By the time this pass runs, the KEYWORD tokens hold uppercase values
    /// (the GrammarLexer normalises case-insensitive keywords to uppercase).
    /// So we check for `token.value == "REM"`.
    ///
    /// What we keep and what we delete:
    ///
    ///   LINE_NUM  KEYWORD("REM")  NAME("comment")  NAME("text")  NEWLINE
    ///   keep      keep            DELETE           DELETE        keep
    ///
    /// The parser rule `rem_stmt = "REM" ;` only needs KEYWORD("REM") to
    /// succeed. The NEWLINE is kept as the line terminator.
    ///
    /// Algorithm:
    ///   1. Walk tokens left-to-right.
    ///   2. When we see KEYWORD("REM"), set `inRem = true` and append the REM.
    ///   3. While `inRem`:
    ///      - NEWLINE: append it, set `inRem = false`, break
    ///      - EOF: append it, stop entirely
    ///      - anything else: discard
    ///
    /// - Parameter tokens: The token stream (after relabelLineNumbers).
    /// - Returns: A new token array with REM content removed.
    ///
    static func suppressRemContent(_ tokens: [Token]) -> [Token] {
        var result: [Token] = []
        result.reserveCapacity(tokens.count)

        var inRem = false

        for token in tokens {
            if inRem {
                // We're inside a REM comment — only NEWLINE and EOF escape.
                if token.type == "NEWLINE" {
                    result.append(token)
                    inRem = false
                } else if token.type == "EOF" {
                    result.append(token)
                    break
                }
                // All other tokens (comment words) are silently discarded.
            } else {
                result.append(token)

                // Check if this token begins a REM comment.
                // The GrammarLexer emits case-insensitive keywords with
                // uppercase values, so we check `token.value == "REM"`.
                if token.type == "KEYWORD" && token.value == "REM" {
                    inRem = true
                }
            }
        }

        return result
    }
}
