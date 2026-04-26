namespace CodingAdventures.RubyParser.Tests

open CodingAdventures.RubyParser.FSharp
open Xunit

module RubyParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("ruby-parser", RubyParser().Ping())