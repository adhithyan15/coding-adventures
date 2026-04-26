namespace CodingAdventures.LatticeParser.Tests

open CodingAdventures.LatticeParser.FSharp
open Xunit

module LatticeParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("lattice-parser", LatticeParser().Ping())