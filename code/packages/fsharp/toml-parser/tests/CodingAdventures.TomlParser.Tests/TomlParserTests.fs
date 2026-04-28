namespace CodingAdventures.TomlParser.Tests

open CodingAdventures.TomlParser.FSharp
open Xunit

module TomlParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("toml-parser", TomlParser().Ping())