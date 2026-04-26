namespace CodingAdventures.CSharpLexer.Tests

open CodingAdventures.CSharpLexer.FSharp
open Xunit

module CSharpLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("csharp-lexer", CSharpLexer().Ping())