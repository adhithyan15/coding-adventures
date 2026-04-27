namespace CodingAdventures.JsonParser.FSharp

/// JSON parser - parses JSON source text using the grammar-driven parser infrastructure.
type JsonParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "json-parser"