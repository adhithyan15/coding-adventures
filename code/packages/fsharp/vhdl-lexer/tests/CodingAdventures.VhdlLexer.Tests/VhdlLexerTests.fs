namespace CodingAdventures.VhdlLexer.Tests

open CodingAdventures.Lexer.FSharp
open CodingAdventures.VhdlLexer.FSharp
open Xunit

module VhdlLexerTests =
    [<Fact>]
    let tokenizesSample () =
        let tokens = VhdlLexer.TokenizeVhdl("ENTITY TOP IS END ENTITY TOP;")

        Assert.Equal(TokenType.Keyword, tokens.[0].Type)
        Assert.Equal("entity", tokens.[0].Value)
        Assert.Equal("NAME", tokens.[1].EffectiveTypeName)
        Assert.Equal("top", tokens.[1].Value)

    [<Fact>]
    let defaultVersionMatchesExplicitVersion () =
        let defaultTokens = VhdlLexer.TokenizeVhdl("entity top is end entity top;")
        let explicitTokens = VhdlLexer.TokenizeVhdl("entity top is end entity top;", VhdlLexer.DefaultVersion)
        Assert.Equal(Seq.length defaultTokens, Seq.length explicitTokens)
        Assert.Equal(defaultTokens.[0].Value, explicitTokens.[0].Value)

    [<Fact>]
    let rejectsUnknownVersion () =
        let error = Assert.Throws<System.ArgumentException>(fun () -> VhdlLexer.TokenizeVhdl("entity top is end entity top;", "2099") |> ignore)
        Assert.Contains("Unknown VHDL version", error.Message)
