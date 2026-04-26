namespace CodingAdventures.TypeScriptLexer.FSharp

/// TypeScript lexer - tokenizes TypeScript source text using the grammar-driven lexer infrastructure.
type TypeScriptLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "typescript-lexer"