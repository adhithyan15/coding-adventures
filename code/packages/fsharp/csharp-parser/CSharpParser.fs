namespace CodingAdventures.CSharpParser.FSharp

/// C# parser - parses C# source text using the grammar-driven parser infrastructure.
type CSharpParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "csharp-parser"