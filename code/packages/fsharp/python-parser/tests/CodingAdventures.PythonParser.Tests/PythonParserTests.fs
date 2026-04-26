namespace CodingAdventures.PythonParser.Tests

open CodingAdventures.PythonParser.FSharp
open Xunit

module PythonParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("python-parser", PythonParser().Ping())