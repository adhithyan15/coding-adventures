namespace CodingAdventures.RubyParser.FSharp

/// Ruby parser - parses Ruby source text using the grammar-driven parser infrastructure.
type RubyParser() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "ruby-parser"