import Foundation
import GrammarTools
import Lexer

/// XML Lexer -- tokenizes XML source code.
public struct XMLLexer: Sendable {
    public static let version = "0.1.0"

    public static func tokenize(_ source: String) throws -> [Token] {
        let grammar = try loadGrammar()
        let lexer = GrammarLexer(source: source, grammar: grammar)
        
        lexer.setOnToken { token, ctx in
            let tokenType = token.type
            switch tokenType {
            case "OPEN_TAG_START", "CLOSE_TAG_START":
                ctx.pushGroup("tag")
            case "TAG_CLOSE", "SELF_CLOSE":
                ctx.popGroup()
            case "COMMENT_START":
                ctx.pushGroup("comment")
                ctx.setSkipEnabled(false)
            case "COMMENT_END":
                ctx.popGroup()
                ctx.setSkipEnabled(true)
            case "CDATA_START":
                ctx.pushGroup("cdata")
                ctx.setSkipEnabled(false)
            case "CDATA_END":
                ctx.popGroup()
                ctx.setSkipEnabled(true)
            case "PI_START":
                ctx.pushGroup("pi")
                ctx.setSkipEnabled(false)
            case "PI_END":
                ctx.popGroup()
                ctx.setSkipEnabled(true)
            default:
                break
            }
        }
        
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
            .appendingPathComponent("xml.tokens")

        let content = try String(contentsOf: tokensURL, encoding: .utf8)
        return try parseTokenGrammar(source: content)
    }
}
