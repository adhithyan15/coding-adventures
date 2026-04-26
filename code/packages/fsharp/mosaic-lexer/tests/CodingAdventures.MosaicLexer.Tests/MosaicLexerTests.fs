namespace CodingAdventures.MosaicLexer.Tests

open CodingAdventures.MosaicLexer.FSharp
open Xunit

module MosaicLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("mosaic-lexer", MosaicLexer().Ping())