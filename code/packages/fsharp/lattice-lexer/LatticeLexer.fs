namespace CodingAdventures.LatticeLexer.FSharp

/// Lattice lexer - tokenizes Lattice source text using the grammar-driven lexer infrastructure.
type LatticeLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "lattice-lexer"