namespace CodingAdventures.SqlParser.FSharp

/// SQL parser - parses SQL source text using the grammar-driven parser infrastructure.
type SqlParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "sql-parser"