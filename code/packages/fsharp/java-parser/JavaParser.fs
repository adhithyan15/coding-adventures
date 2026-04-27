namespace CodingAdventures.JavaParser.FSharp

/// Java parser - parses Java source text using the grammar-driven parser infrastructure.
type JavaParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "java-parser"