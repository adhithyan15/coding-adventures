namespace CodingAdventures.CSharpLexer.FSharp

/// C# lexer - tokenizes C# source text using the grammar-driven lexer infrastructure.
type CSharpLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "csharp-lexer"