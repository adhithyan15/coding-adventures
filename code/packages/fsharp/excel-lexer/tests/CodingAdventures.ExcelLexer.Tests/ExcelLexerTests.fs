namespace CodingAdventures.ExcelLexer.Tests

open CodingAdventures.ExcelLexer.FSharp
open Xunit

module ExcelLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("excel-lexer", ExcelLexer().Ping())