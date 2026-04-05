// ============================================================================
// EcmascriptES3Lexer.swift -- ECMAScript 3 (1999) Lexer
// ============================================================================
//
// ECMAScript 3 (ECMA-262, 3rd Edition, December 1999) made JavaScript a
// real, complete language. It adds over ES1:
//   - === and !== (strict equality)
//   - try/catch/finally/throw (error handling)
//   - Regular expression literals (/pattern/flags)
//   - `instanceof` operator
//   - 28 keywords total
// ============================================================================

import Foundation
import GrammarTools
import Lexer

/// ECMAScript 3 (1999) lexer -- tokenizes ES3 source code.
public struct EcmascriptES3Lexer: Sendable {

    public static let version = "0.1.0"

    /// Tokenize an ECMAScript 3 source string.
    public static func tokenize(_ source: String) throws -> [Token] {
        let grammar = try loadGrammar()
        let lexer = GrammarLexer(source: source, grammar: grammar)
        let raw = try lexer.tokenize()

        // Post-process: promote KEYWORD tokens to have uppercased keyword
        // as the type (e.g., "var" -> type "VAR").
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

    /// Load and parse the ES3 token grammar.
    public static func loadGrammar() throws -> TokenGrammar {
        let thisFile = #filePath
        var url = URL(fileURLWithPath: thisFile)
        for _ in 0..<6 {
            url = url.deletingLastPathComponent()
        }
        let tokensURL = url
            .appendingPathComponent("grammars")
            .appendingPathComponent("ecmascript")
            .appendingPathComponent("es3.tokens")

        let content = try String(contentsOf: tokensURL, encoding: .utf8)
        return try parseTokenGrammar(source: content)
    }
}
