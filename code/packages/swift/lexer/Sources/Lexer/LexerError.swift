// ============================================================================
// LexerError.swift — Error type for lexer failures.
// ============================================================================
//
// When the lexer encounters something it cannot handle -- an unexpected
// character, a reserved keyword used as an identifier, or inconsistent
// indentation -- it throws a `LexerError`.
//
// The error carries position information (line and column) so that
// downstream tools (compilers, IDEs, linters) can point the user to
// the exact location of the problem in their source code.
//
// Example error flow:
//
//     Source:  "x = @"
//                  ^
//     Error:   LexerError("Unexpected character: \"@\"", line: 1, column: 5)
//
// The position points to the offending character, making it easy for
// the user to find and fix the issue.
//
// ============================================================================

import Foundation

/// An error thrown during lexical analysis (tokenization).
///
/// Carries a human-readable message and the position in the source code
/// where the error was detected. Both `line` and `column` are 1-based
/// to match conventional text editor numbering.
///
/// Common causes:
/// - Unexpected character that doesn't match any token pattern
/// - Reserved keyword used as an identifier
/// - Tab character in indentation (when spaces-only mode is active)
/// - Inconsistent dedent level in indentation mode
///
public struct LexerError: Error, Sendable, Equatable {
    /// Human-readable description of what went wrong.
    ///
    /// Examples:
    ///   - "Unexpected character: \"@\""
    ///   - "Reserved keyword 'class' cannot be used as an identifier"
    ///   - "Tab character in indentation (use spaces only)"
    ///   - "Inconsistent dedent"
    ///
    public let message: String

    /// The 1-based line number where the error was detected.
    public let line: Int

    /// The 1-based column number where the error was detected.
    public let column: Int

    /// Create a new LexerError with message and position.
    ///
    /// - Parameters:
    ///   - message: Human-readable error description.
    ///   - line: 1-based line number of the error.
    ///   - column: 1-based column number of the error.
    ///
    public init(_ message: String, line: Int, column: Int) {
        self.message = message
        self.line = line
        self.column = column
    }
}

// Provide a nice description when the error is printed.
extension LexerError: CustomStringConvertible {
    public var description: String {
        "LexerError at line \(line), column \(column): \(message)"
    }
}
