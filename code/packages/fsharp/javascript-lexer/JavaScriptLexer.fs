namespace CodingAdventures.JavaScriptLexer.FSharp

/// JavaScript lexer - tokenizes JavaScript source text using the grammar-driven lexer infrastructure.
type JavaScriptLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "javascript-lexer"