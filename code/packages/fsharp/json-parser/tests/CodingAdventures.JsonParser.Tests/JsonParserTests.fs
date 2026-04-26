namespace CodingAdventures.JsonParser.Tests

open CodingAdventures.JsonParser.FSharp
open Xunit

module JsonParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("json-parser", JsonParser().Ping())