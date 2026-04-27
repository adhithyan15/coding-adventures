namespace CodingAdventures.StarlarkLexer.FSharp

/// Starlark lexer - tokenizes Starlark source text using the grammar-driven lexer infrastructure.
type StarlarkLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "starlark-lexer"