namespace CodingAdventures.JavaLexer.Tests

open CodingAdventures.JavaLexer.FSharp
open Xunit

module JavaLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("java-lexer", JavaLexer().Ping())