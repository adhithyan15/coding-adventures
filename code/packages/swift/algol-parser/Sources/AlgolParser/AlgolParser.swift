import Foundation
import GrammarTools
import Lexer
import Parser
import AlgolLexer

public struct AlgolParser: Sendable {
    public static let version = "0.1.0"
    private static let validVersions: Set<String> = ["algol60"]

    public static func parse(_ source: String, version: String = "algol60") throws -> ASTNode {
        let tokens = try AlgolLexer.tokenize(source, version: version)
        return try parseTokens(tokens, version: version)
    }

    public static func parseTokens(_ tokens: [Token], version: String = "algol60") throws -> ASTNode {
        let grammar = try loadGrammar(version: version)
        let parser = GrammarParser(tokens: tokens, grammar: grammar)
        return try parser.parse()
    }

    public static func loadGrammar(version: String = "algol60") throws -> ParserGrammar {
        let normalizedVersion = try normalize(version)
        let thisFile = #filePath
        var url = URL(fileURLWithPath: thisFile)
        for _ in 0..<6 {
            url = url.deletingLastPathComponent()
        }
        let grammarURL = url
            .appendingPathComponent("grammars")
            .appendingPathComponent("algol")
            .appendingPathComponent("\(normalizedVersion).grammar")

        let content = try String(contentsOf: grammarURL, encoding: .utf8)
        return try parseParserGrammar(source: content)
    }

    private static func normalize(_ version: String) throws -> String {
        if validVersions.contains(version) {
            return version
        }
        throw NSError(
            domain: "AlgolParser",
            code: 1,
            userInfo: [NSLocalizedDescriptionKey: "Unknown ALGOL version \(version). Valid versions: algol60"]
        )
    }
}
