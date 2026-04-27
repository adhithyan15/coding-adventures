namespace CodingAdventures.StarlarkLexer.Tests

open CodingAdventures.StarlarkLexer.FSharp
open Xunit

module StarlarkLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("starlark-lexer", StarlarkLexer().Ping())