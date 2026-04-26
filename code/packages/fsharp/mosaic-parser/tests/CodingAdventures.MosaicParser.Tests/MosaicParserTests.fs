namespace CodingAdventures.MosaicParser.Tests

open CodingAdventures.MosaicParser.FSharp
open Xunit

module MosaicParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("mosaic-parser", MosaicParser().Ping())