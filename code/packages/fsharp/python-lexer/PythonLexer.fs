namespace CodingAdventures.PythonLexer.FSharp

/// Python lexer - tokenizes Python source text using the grammar-driven lexer infrastructure.
type PythonLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "python-lexer"