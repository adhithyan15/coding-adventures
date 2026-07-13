import Foundation
import GrammarTools
import Lexer

public struct AlgolLexer: Sendable {
    public static let version = "0.1.0"
    private static let validVersions: Set<String> = ["algol60"]

    public static func tokenize(_ source: String, version: String = "algol60") throws -> [Token] {
        let grammar = try loadGrammar(version: version)
        let lexer = GrammarLexer(source: source, grammar: grammar)
        return try lexer.tokenize()
    }

    public static func loadGrammar(version: String = "algol60") throws -> TokenGrammar {
        let normalizedVersion = try normalize(version)
        // The grammar is embedded at compile time in the generated
        // `_Grammar.swift` (from code/grammars/algol/<version>.tokens); nothing
        // is read from disk at run time.
        guard let grammar = EmbeddedGrammar.tokenGrammars[normalizedVersion] else {
            throw NSError(
                domain: "AlgolLexer",
                code: 1,
                userInfo: [NSLocalizedDescriptionKey: "No embedded grammar for ALGOL version \(normalizedVersion)."]
            )
        }
        return grammar
    }

    private static func normalize(_ version: String) throws -> String {
        if validVersions.contains(version) {
            return version
        }
        throw NSError(
            domain: "AlgolLexer",
            code: 1,
            userInfo: [NSLocalizedDescriptionKey: "Unknown ALGOL version \(version). Valid versions: algol60"]
        )
    }
}
