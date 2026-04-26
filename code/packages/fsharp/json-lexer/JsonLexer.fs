namespace CodingAdventures.JsonLexer.FSharp

/// JSON lexer - tokenizes JSON source text using the grammar-driven lexer infrastructure.
type JsonLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "json-lexer"