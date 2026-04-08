import Foundation
import GrammarTools
import Lexer

/// TOML Lexer -- tokenizes TOML source code.
public struct TOMLLexer: Sendable {
    public static let version = "0.1.0"

    public static func tokenize(_ source: String) throws -> [Token] {
        let grammar = try loadGrammar()
        let lexer = GrammarLexer(source: source, grammar: grammar)
        return try lexer.tokenize()
    }

    public static func loadGrammar() throws -> TokenGrammar {
        let thisFile = #filePath
        var url = URL(fileURLWithPath: thisFile)
        for _ in 0..<6 {
            url = url.deletingLastPathComponent()
        }
        let tokensURL = url
            .appendingPathComponent("grammars")
            .appendingPathComponent("toml.tokens")

        let content = try String(contentsOf: tokensURL, encoding: .utf8)
        return try parseTokenGrammar(source: content)
    }
}
