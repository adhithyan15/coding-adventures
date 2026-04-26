namespace CodingAdventures.JsonLexer.Tests

open CodingAdventures.JsonLexer.FSharp
open Xunit

module JsonLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("json-lexer", JsonLexer().Ping())