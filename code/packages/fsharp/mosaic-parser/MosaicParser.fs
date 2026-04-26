namespace CodingAdventures.MosaicParser.FSharp

/// Mosaic parser - parses Mosaic source text using the grammar-driven parser infrastructure.
type MosaicParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "mosaic-parser"