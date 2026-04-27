namespace CodingAdventures.MosaicLexer.FSharp

/// Mosaic lexer - tokenizes Mosaic source text using the grammar-driven lexer infrastructure.
type MosaicLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "mosaic-lexer"