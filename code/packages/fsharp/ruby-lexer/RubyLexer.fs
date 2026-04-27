namespace CodingAdventures.RubyLexer.FSharp

/// Ruby lexer - tokenizes Ruby source text using the grammar-driven lexer infrastructure.
type RubyLexer() =
    /// Returns the package identifier used by the parity placeholder packages.
    member _.Ping() = "ruby-lexer"