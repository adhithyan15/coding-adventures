import GrammarTools
import Lexer

/// The token grammar for JSON text, embedding the rules from json.tokens.
public let jsonTokenGrammar = TokenGrammar(
    definitions: [
        // "([^"\\]|\\["\/bfnrt]|\u[0-9a-fA-F]{4})*"
        TokenDefinition(name: "STRING", pattern: "\"([^\"\\\\]|\\\\[\"\\\\/bfnrt]|\\\\u[0-9a-fA-F]{4})*\"", isRegex: true, lineNumber: 25, alias: nil),
        TokenDefinition(name: "NUMBER", pattern: "-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?", isRegex: true, lineNumber: 31, alias: nil),
        TokenDefinition(name: "TRUE", pattern: "true", isRegex: false, lineNumber: 35, alias: nil),
        TokenDefinition(name: "FALSE", pattern: "false", isRegex: false, lineNumber: 36, alias: nil),
        TokenDefinition(name: "NULL", pattern: "null", isRegex: false, lineNumber: 37, alias: nil),
        TokenDefinition(name: "LBRACE", pattern: "{", isRegex: false, lineNumber: 43, alias: nil),
        TokenDefinition(name: "RBRACE", pattern: "}", isRegex: false, lineNumber: 44, alias: nil),
        TokenDefinition(name: "LBRACKET", pattern: "[", isRegex: false, lineNumber: 45, alias: nil),
        TokenDefinition(name: "RBRACKET", pattern: "]", isRegex: false, lineNumber: 46, alias: nil),
        TokenDefinition(name: "COLON", pattern: ":", isRegex: false, lineNumber: 47, alias: nil),
        TokenDefinition(name: "COMMA", pattern: ",", isRegex: false, lineNumber: 48, alias: nil)
    ],
    keywords: [],
    mode: nil,
    escapeMode: nil,
    skipDefinitions: [
        TokenDefinition(name: "WHITESPACE", pattern: "[ \\t\\r\\n]+", isRegex: true, lineNumber: 59, alias: nil)
    ],
    reservedKeywords: [],
    contextKeywords: [],
    groups: nil,
    caseSensitive: true,
    version: 1,
    caseInsensitive: false
)

/// Create a `GrammarLexer` configured for JSON text.
///
/// - Parameter source: The JSON text to tokenize.
/// - Returns: A `GrammarLexer` instance ready to produce JSON tokens.
public func createJsonLexer(_ source: String) -> GrammarLexer {
    return GrammarLexer(source: source, grammar: jsonTokenGrammar)
}

/// Tokenize JSON text and return an array of tokens.
///
/// This is the main entry point to Lex JSON content.
///
/// - Parameter source: The JSON text to tokenize.
/// - Returns: An array of `Token` instances, concluding with `EOF`.
/// - Throws: `LexerError` if the input is malformed.
public func tokenizeJson(_ source: String) throws -> [Token] {
    let lexer = createJsonLexer(source)
    return try lexer.tokenize()
}
