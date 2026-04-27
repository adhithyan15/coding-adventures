namespace CodingAdventures.TypeScriptLexer.Tests

open CodingAdventures.TypeScriptLexer.FSharp
open Xunit

module TypeScriptLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("typescript-lexer", TypeScriptLexer().Ping())