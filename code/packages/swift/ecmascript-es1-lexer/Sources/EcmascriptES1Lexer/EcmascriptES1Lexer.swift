// ============================================================================
// EcmascriptES1Lexer.swift -- ECMAScript 1 (1997) Lexer
// ============================================================================
//
// A thin wrapper around the grammar-driven GrammarLexer from the Lexer
// package, configured by the shared ecmascript/es1.tokens grammar file.
//
// ECMAScript 1 (ECMA-262, 1st Edition, June 1997) was the first standardized
// JavaScript. It defines:
//   - 23 keywords (var, function, if, else, for, while, etc.)
//   - Basic operators (no === or !==)
//   - String literals (single and double quoted)
//   - Numeric literals (decimal, float, hex, scientific)
//   - The $ character is valid in identifiers
//
// Usage:
//
//     let tokens = try EcmascriptES1Lexer.tokenize("var x = 42;")
//     // tokens[0].type == "VAR", tokens[0].value == "var"
//     // tokens[1].type == "NAME", tokens[1].value == "x"
//
// ============================================================================

import Foundation
import GrammarTools
import Lexer

/// ECMAScript 1 (1997) lexer -- tokenizes ES1 source code.
///
/// This struct provides a static `tokenize(_:)` method that reads the
/// `ecmascript/es1.tokens` grammar file, constructs a `GrammarLexer`,
/// and returns the resulting token stream.
///
/// The grammar file is loaded relative to this source file's location
/// in the monorepo, navigating up to the `code/` directory and then
/// into `grammars/ecmascript/es1.tokens`.
public struct EcmascriptES1Lexer: Sendable {

    /// The version of this package.
    public static let version = "0.1.0"

    /// Tokenize an ECMAScript 1 source string.
    ///
    /// - Parameter source: The ES1 source code to tokenize.
    /// - Returns: An array of `Token` values, ending with an EOF token.
    /// - Throws: `LexerError` on unexpected characters.
    public static func tokenize(_ source: String) throws -> [Token] {
        let grammar = try loadGrammar()
        let lexer = GrammarLexer(source: source, grammar: grammar)
        let raw = try lexer.tokenize()

        // Post-process: the GrammarLexer emits keywords with type "KEYWORD"
        // and the actual keyword text as the value. We promote these to have
        // the uppercased keyword as the type (e.g., "var" -> type "VAR")
        // to match the convention used by the Lua and Perl lexer wrappers.
        return raw.map { token in
            if token.type == "KEYWORD" {
                return Token(
                    type: token.value.uppercased(),
                    value: token.value,
                    line: token.line,
                    column: token.column,
                    flags: token.flags
                )
            }
            return token
        }
    }

    /// Load and parse the ES1 token grammar.
    ///
    /// The grammar file path is computed relative to this source file:
    ///   Sources/EcmascriptES1Lexer/  (1)
    ///   ecmascript-es1-lexer/        (2)
    ///   swift/                        (3)
    ///   packages/                     (4)
    ///   code/                         (5)  -> grammars/ecmascript/es1.tokens
    ///
    /// - Returns: A `TokenGrammar` parsed from `es1.tokens`.
    /// - Throws: If the file cannot be read or parsed.
    public static func loadGrammar() throws -> TokenGrammar {
        let thisFile = #filePath
        var url = URL(fileURLWithPath: thisFile)
        // Walk up: EcmascriptES1Lexer.swift -> Sources/EcmascriptES1Lexer -> Sources -> ecmascript-es1-lexer -> swift -> packages -> code
        for _ in 0..<6 {
            url = url.deletingLastPathComponent()
        }
        let tokensURL = url
            .appendingPathComponent("grammars")
            .appendingPathComponent("ecmascript")
            .appendingPathComponent("es1.tokens")

        let content = try String(contentsOf: tokensURL, encoding: .utf8)
        return try parseTokenGrammar(source: content)
    }
}
