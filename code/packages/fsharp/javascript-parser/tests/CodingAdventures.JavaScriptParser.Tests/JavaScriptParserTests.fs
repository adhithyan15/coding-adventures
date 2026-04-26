namespace CodingAdventures.JavaScriptParser.Tests

open CodingAdventures.JavaScriptParser.FSharp
open Xunit

module JavaScriptParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("javascript-parser", JavaScriptParser().Ping())