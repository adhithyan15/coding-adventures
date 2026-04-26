namespace CodingAdventures.PythonLexer.Tests

open CodingAdventures.PythonLexer.FSharp
open Xunit

module PythonLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("python-lexer", PythonLexer().Ping())