namespace CodingAdventures.JavaLexer.FSharp

/// Java lexer - tokenizes Java source text using the grammar-driven lexer infrastructure.
type JavaLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "java-lexer"