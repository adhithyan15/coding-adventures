namespace CodingAdventures.ExcelParser.Tests

open CodingAdventures.ExcelParser.FSharp
open Xunit

module ExcelParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("excel-parser", ExcelParser().Ping())