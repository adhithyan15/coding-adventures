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
        let thisFile = #filePath
        var url = URL(fileURLWithPath: thisFile)
        for _ in 0..<6 {
            url = url.deletingLastPathComponent()
        }
        let tokensURL = url
            .appendingPathComponent("grammars")
            .appendingPathComponent("algol")
            .appendingPathComponent("\(normalizedVersion).tokens")

        let content = try String(contentsOf: tokensURL, encoding: .utf8)
        return try parseTokenGrammar(source: content)
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
