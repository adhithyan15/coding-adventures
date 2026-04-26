namespace CodingAdventures.SqlLexer.FSharp

/// SQL lexer - tokenizes SQL source text using the grammar-driven lexer infrastructure.
type SqlLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "sql-lexer"