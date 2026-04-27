namespace CodingAdventures.LatticeLexer.Tests

open CodingAdventures.LatticeLexer.FSharp
open Xunit

module LatticeLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("lattice-lexer", LatticeLexer().Ping())