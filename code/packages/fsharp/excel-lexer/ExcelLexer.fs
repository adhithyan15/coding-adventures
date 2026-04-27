namespace CodingAdventures.ExcelLexer.FSharp

/// Excel lexer - tokenizes Excel source text using the grammar-driven lexer infrastructure.
type ExcelLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "excel-lexer"