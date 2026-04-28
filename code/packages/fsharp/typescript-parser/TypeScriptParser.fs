namespace CodingAdventures.TypeScriptParser.FSharp

/// TypeScript parser - parses TypeScript source text using the grammar-driven parser infrastructure.
type TypeScriptParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "typescript-parser"