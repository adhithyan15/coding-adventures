// ============================================================================
// EcmascriptES5Lexer.swift -- ECMAScript 5 (2009) Lexer
// ============================================================================
//
// ECMAScript 5 (ECMA-262, 5th Edition, December 2009) adds the `debugger`
// keyword over ES3 and retains all ES3 features. The real innovations were
// strict mode semantics, native JSON support, and property descriptors.
// ============================================================================

import Foundation
import GrammarTools
import Lexer

/// ECMAScript 5 (2009) lexer -- tokenizes ES5 source code.
public struct EcmascriptES5Lexer: Sendable {

    public static let version = "0.1.0"

    /// Tokenize an ECMAScript 5 source string.
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

    /// Load and parse the ES5 token grammar.
    public static func loadGrammar() throws -> TokenGrammar {
        let thisFile = #filePath
        var url = URL(fileURLWithPath: thisFile)
        for _ in 0..<6 {
            url = url.deletingLastPathComponent()
        }
        let tokensURL = url
            .appendingPathComponent("grammars")
            .appendingPathComponent("ecmascript")
            .appendingPathComponent("es5.tokens")

        let content = try String(contentsOf: tokensURL, encoding: .utf8)
        return try parseTokenGrammar(source: content)
    }
}
