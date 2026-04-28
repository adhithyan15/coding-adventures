namespace CodingAdventures.ExcelParser.FSharp

/// Excel parser - parses Excel source text using the grammar-driven parser infrastructure.
type ExcelParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "excel-parser"