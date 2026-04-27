import Foundation
import GrammarTools
import Lexer

/// XML Lexer -- tokenizes XML using pattern groups and callback hooks.
public struct XMLLexer: Sendable {
    public static let version = "0.1.0"

    /// Callback that switches pattern groups for XML tokenization.
    public static func xmlOnToken(token: Token, ctx: LexerContext) {
        let tokenType = token.type

        switch tokenType {
        // --- Tag boundaries ---
        case "OPEN_TAG_START", "CLOSE_TAG_START":
            ctx.pushGroup("tag")
            
        case "TAG_CLOSE", "SELF_CLOSE":
            ctx.popGroup()

        // --- Comment boundaries ---
        case "COMMENT_START":
            ctx.pushGroup("comment")
            ctx.setSkipEnabled(false)
            
        case "COMMENT_END":
            ctx.popGroup()
            ctx.setSkipEnabled(true)

        // --- CDATA boundaries ---
        case "CDATA_START":
            ctx.pushGroup("cdata")
            ctx.setSkipEnabled(false)
            
        case "CDATA_END":
            ctx.popGroup()
            ctx.setSkipEnabled(true)

        // --- Processing instruction boundaries ---
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

    /// Load and parse the XML token grammar.
    public static func loadGrammar() throws -> TokenGrammar {
        let thisFile = #filePath
        var url = URL(fileURLWithPath: thisFile)
        // Walk up: XMLLexer.swift -> Sources/XMLLexer -> Sources -> xml-lexer -> swift -> packages -> code
        for _ in 0..<6 {
            url = url.deletingLastPathComponent()
        }
        let tokensURL = url
            .appendingPathComponent("grammars")
            .appendingPathComponent("xml.tokens")

        let content = try String(contentsOf: tokensURL, encoding: .utf8)
        return try parseTokenGrammar(source: content)
    }

    /// Create a `GrammarLexer` configured for XML text.
    public static func createXMLLexer(_ source: String) throws -> GrammarLexer {
        let grammar = try loadGrammar()
        let lexer = GrammarLexer(source: source, grammar: grammar)
        lexer.setOnToken(xmlOnToken)
        return lexer
    }

    /// Tokenize XML text and return an array of tokens.
    public static func tokenize(_ source: String) throws -> [Token] {
        return try createXMLLexer(source).tokenize()
    }
}
