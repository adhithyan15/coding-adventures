namespace CodingAdventures.StarlarkParser.FSharp

/// Starlark parser - parses Starlark source text using the grammar-driven parser infrastructure.
type StarlarkParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "starlark-parser"