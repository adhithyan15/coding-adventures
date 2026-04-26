namespace CodingAdventures.TypeScriptParser.Tests

open CodingAdventures.TypeScriptParser.FSharp
open Xunit

module TypeScriptParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("typescript-parser", TypeScriptParser().Ping())