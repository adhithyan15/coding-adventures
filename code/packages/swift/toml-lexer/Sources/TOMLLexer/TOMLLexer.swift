import Foundation
import GrammarTools
import Lexer

/// TOML Lexer -- tokenizes TOML source text.
///
/// This module is a **thin wrapper** around the generic `GrammarLexer`
/// from the `Lexer` package. It loads the `toml.tokens` grammar file
/// and delegates all tokenization work to the generic engine.
public struct TOMLLexer: Sendable {
    public static let version = "0.1.0"

    /// Load and parse the TOML token grammar.
    public static func loadGrammar() throws -> TokenGrammar {
        let thisFile = #filePath
        var url = URL(fileURLWithPath: thisFile)
        // Walk up: TOMLLexer.swift -> Sources/TOMLLexer -> Sources -> toml-lexer -> swift -> packages -> code
        for _ in 0..<6 {
            url = url.deletingLastPathComponent()
        }
        let tokensURL = url
            .appendingPathComponent("grammars")
            .appendingPathComponent("toml.tokens")

        let content = try String(contentsOf: tokensURL, encoding: .utf8)
        return try parseTokenGrammar(source: content)
    }

    /// Tokenize TOML text and return an array of tokens.
    ///
    /// The function reads the `toml.tokens` grammar file, parses it into a
    /// `TokenGrammar` object, then passes the source text to the generic
    /// `GrammarLexer`.
    ///
    /// @param source - The TOML text to tokenize.
    /// @returns An array of Token objects. The last token is always EOF.
    public static func tokenize(_ source: String) throws -> [Token] {
        let grammar = try loadGrammar()
        let lexer = GrammarLexer(source: source, grammar: grammar)
        return try lexer.tokenize()
    }
}
