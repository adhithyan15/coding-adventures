namespace CodingAdventures.JavaParser.Tests

open CodingAdventures.JavaParser.FSharp
open Xunit

module JavaParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("java-parser", JavaParser().Ping())