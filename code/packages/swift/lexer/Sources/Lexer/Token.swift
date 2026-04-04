// ============================================================================
// Token.swift — The smallest meaningful unit of source code.
// ============================================================================
//
// Before a computer can execute code like `x = 1 + 2`, it needs to break
// that raw text into meaningful chunks. Those chunks are called **tokens**.
//
// Think of it like reading a sentence in English. When you see:
//
//     The cat sat on the mat.
//
// Your brain automatically groups the letters into words: "The", "cat",
// "sat", "on", "the", "mat", and the period ".". You don't think about
// individual letters -- you think about *words* and *punctuation*. A lexer
// does the same thing for source code.
//
// Given the input `x = 1 + 2`, the lexer produces:
//
//     NAME("x")  EQUALS("=")  NUMBER("1")  PLUS("+")  NUMBER("2")  EOF
//
// Each of these is a **Token** -- a small labeled piece of text. The label
// (like NAME or NUMBER) is called the **token type**, and the text itself
// (like "x" or "1") is called the **token value**.
//
// In Swift we represent tokens as value-type structs conforming to Sendable
// and Equatable. Unlike the TypeScript version which uses plain interfaces,
// Swift structs give us value semantics -- copying a token creates an
// independent copy, preventing accidental mutation bugs.
//
// ============================================================================

import Foundation

// ---------------------------------------------------------------------------
// Token Flag Constants
// ---------------------------------------------------------------------------

/// Bitmask flags for token metadata.
///
/// Flags carry information that is neither type nor value but affects
/// how downstream consumers (parsers, formatters, linters) interpret
/// a token. For example, JavaScript's automatic semicolon insertion
/// rule depends on whether a newline appeared before certain tokens.
///
/// Flags are optional -- when `flags` is 0, all flags are off.
/// Use bitwise AND to test: `token.flags & TOKEN_PRECEDED_BY_NEWLINE != 0`

/// Set when a line break appeared between this token and the previous one.
///
/// Languages with automatic semicolon insertion (JavaScript, Go) use
/// this to decide whether an implicit semicolon should be inserted.
/// The lexer itself does not insert semicolons -- that is a language-
/// specific concern handled in language packages via post-tokenize
/// hooks or parser pre-parse hooks.
///
/// Truth table for this flag:
///
///     Previous token line | Current token line | Flag set?
///     -------------------|--------------------|----------
///               1        |         1          |    No
///               1        |         2          |    Yes
///               3        |         7          |    Yes
///               5        |         5          |    No
///
public let TOKEN_PRECEDED_BY_NEWLINE: Int = 1

/// Set for context-sensitive keywords -- words that are keywords in some
/// syntactic positions but identifiers in others.
///
/// For example, JavaScript's `async`, `yield`, `await`, `get`, `set`
/// are sometimes keywords (in function declarations, property accessors)
/// and sometimes plain identifiers (`let get = 5`). The lexer emits
/// these as NAME tokens with this flag set, leaving the final
/// keyword-vs-identifier decision to the language-specific parser.
///
/// Example:
///
///     // In "async function foo()", async is a keyword:
///     Token(type: "NAME", value: "async", flags: TOKEN_CONTEXT_KEYWORD)
///
///     // The parser sees the flag and decides it's a keyword here
///     // based on what follows (the word "function").
///
public let TOKEN_CONTEXT_KEYWORD: Int = 2


// ---------------------------------------------------------------------------
// Token Struct
// ---------------------------------------------------------------------------

/// A single token -- the smallest meaningful unit of source code.
///
/// A token pairs a **type** (what kind of thing it is) with a **value**
/// (the actual text from the source code), plus position information for
/// error reporting.
///
/// Think of a token like a labeled sticky note attached to a piece of text:
///
///     +----------+
///     | NAME     |  <- type (what kind of token)
///     | "x"      |  <- value (the actual text)
///     | line 1   |  <- where it appeared
///     | col 1    |
///     +----------+
///
/// Why use a struct instead of a class? Tokens are simple data -- they
/// don't need inheritance or reference semantics. A value-type struct is
/// the lightest-weight representation in Swift, gives us Equatable for
/// free (via synthesis), and is trivially thread-safe (Sendable).
///
/// Properties:
///   - type: The kind of token (e.g., "NAME", "NUMBER", "PLUS").
///   - value: The actual text from the source that this token represents.
///   - line: The 1-based line number where this token starts.
///   - column: The 1-based column number where this token starts.
///   - flags: Bitmask of token metadata flags (default 0 = no flags set).
///
public struct Token: Sendable, Equatable {
    /// The kind of token (e.g., "NAME", "NUMBER", "PLUS", "EOF").
    public let type: String

    /// The actual text from the source code that this token represents.
    /// For the EOF token, this is an empty string.
    public let value: String

    /// The 1-based line number where this token starts in the source.
    public let line: Int

    /// The 1-based column number where this token starts in the source.
    public let column: Int

    /// Bitmask of token metadata flags. Defaults to 0 (no flags).
    ///
    /// Use bitwise AND to test individual flags:
    ///
    ///     if token.flags & TOKEN_PRECEDED_BY_NEWLINE != 0 {
    ///         // A newline appeared before this token
    ///     }
    ///
    public let flags: Int

    /// Create a new Token with all fields specified.
    ///
    /// - Parameters:
    ///   - type: The token type string (e.g., "NAME", "NUMBER").
    ///   - value: The matched source text.
    ///   - line: 1-based line number.
    ///   - column: 1-based column number.
    ///   - flags: Bitmask of metadata flags (default 0).
    ///
    public init(type: String, value: String, line: Int, column: Int, flags: Int = 0) {
        self.type = type
        self.value = value
        self.line = line
        self.column = column
        self.flags = flags
    }
}
