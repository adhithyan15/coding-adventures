namespace CodingAdventures.RubyLexer.Tests

open CodingAdventures.RubyLexer.FSharp
open Xunit

module RubyLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("ruby-lexer", RubyLexer().Ping())