namespace CodingAdventures.CSharpParser.Tests

open CodingAdventures.CSharpParser.FSharp
open Xunit

module CSharpParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("csharp-parser", CSharpParser().Ping())