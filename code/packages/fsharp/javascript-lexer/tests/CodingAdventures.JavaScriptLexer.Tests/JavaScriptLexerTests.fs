namespace CodingAdventures.JavaScriptLexer.Tests

open CodingAdventures.JavaScriptLexer.FSharp
open Xunit

module JavaScriptLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("javascript-lexer", JavaScriptLexer().Ping())