namespace CodingAdventures.TomlParser.FSharp

/// TOML parser - parses TOML source text using the grammar-driven parser infrastructure.
type TomlParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "toml-parser"