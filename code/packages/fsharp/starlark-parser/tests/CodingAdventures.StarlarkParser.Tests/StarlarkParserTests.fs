namespace CodingAdventures.StarlarkParser.Tests

open CodingAdventures.StarlarkParser.FSharp
open Xunit

module StarlarkParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("starlark-parser", StarlarkParser().Ping())