namespace CodingAdventures.TomlLexer.FSharp

/// TOML lexer - tokenizes TOML source text using the grammar-driven lexer infrastructure.
type TomlLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "toml-lexer"