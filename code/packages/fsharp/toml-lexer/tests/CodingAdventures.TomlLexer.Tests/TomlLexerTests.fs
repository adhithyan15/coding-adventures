namespace CodingAdventures.TomlLexer.Tests

open CodingAdventures.TomlLexer.FSharp
open Xunit

module TomlLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("toml-lexer", TomlLexer().Ping())