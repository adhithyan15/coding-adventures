namespace CodingAdventures.LatticeParser.FSharp

/// Lattice parser - parses Lattice source text using the grammar-driven parser infrastructure.
type LatticeParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "lattice-parser"