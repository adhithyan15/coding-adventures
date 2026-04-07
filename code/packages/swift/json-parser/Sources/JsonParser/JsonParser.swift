import GrammarTools
import Lexer
import Parser
import JsonLexer

/// The JSON parser grammar source embed, mimicking `json.grammar`.
public let jsonGrammarSource = """
value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
object = LBRACE [ pair { COMMA pair } ] RBRACE ;
pair   = STRING COLON value ;
array  = LBRACKET [ value { COMMA value } ] RBRACKET ;
"""

/// The parsed grammar for JSON, constructed via `parseParserGrammar`.
public let jsonParserGrammar: ParserGrammar = {
    do {
        return try parseParserGrammar(source: jsonGrammarSource)
    } catch {
        fatalError("Failed to parse json.grammar embed: \(error)")
    }
}()

/// Create a `GrammarParser` configured for JSON text.
///
/// This function tokenizes the source text using `json-lexer` and creates a
/// `GrammarParser` set up with `jsonParserGrammar`.
///
/// - Parameter source: The JSON text to parse.
/// - Returns: A `GrammarParser` instance ready to produce an AST.
public func createJsonParser(_ source: String) throws -> GrammarParser {
    let tokens = try tokenizeJson(source)
    return GrammarParser(tokens: tokens, grammar: jsonParserGrammar)
}

/// Parse JSON text and return an AST.
///
/// This is the main entry point for the JSON parser. Pass in JSON text
/// and get back an `ASTNode` representing the complete parse tree.
///
/// - Parameter source: The JSON text to parse.
/// - Returns: An `ASTNode` representing the parse tree. The root node's
///            `ruleName` is `"value"`.
/// - Throws: `LexerError` if tokenization fails, or `GrammarParseError` if
///           parsing fails according to JSON grammar rules.
public func parseJson(_ source: String) throws -> ASTNode {
    let parser = try createJsonParser(source)
    return try parser.parse()
}
