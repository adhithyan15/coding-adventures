namespace CodingAdventures.PythonParser.FSharp

/// Python parser - parses Python source text using the grammar-driven parser infrastructure.
type PythonParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "python-parser"