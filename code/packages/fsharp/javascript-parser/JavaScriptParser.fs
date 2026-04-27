namespace CodingAdventures.JavaScriptParser.FSharp

/// JavaScript parser - parses JavaScript source text using the grammar-driven parser infrastructure.
type JavaScriptParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "javascript-parser"