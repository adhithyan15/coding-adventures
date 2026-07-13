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
    public static func loadGrammar() -> TokenGrammar {
        // Embedded at compile time in the generated `_Grammar.swift`
        // (from code/grammars/toml/...); no run-time file read.
        EmbeddedGrammar.toml
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
        let grammar = loadGrammar()
        let lexer = GrammarLexer(source: source, grammar: grammar)
        return try lexer.tokenize()
    }
}
