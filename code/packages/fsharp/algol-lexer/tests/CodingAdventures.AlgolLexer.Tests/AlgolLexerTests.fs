namespace CodingAdventures.AlgolLexer.Tests

open CodingAdventures.Lexer.FSharp
open CodingAdventures.AlgolLexer.FSharp
open Xunit

module AlgolLexerTests =
    [<Fact>]
    let tokenizesSample () =
        let tokens = AlgolLexer.TokenizeAlgol("begin integer x; x := 42 end")

        Assert.Equal(TokenType.Keyword, tokens.[0].Type)
        Assert.Equal("begin", tokens.[0].Value)
        Assert.Equal("NAME", tokens.[2].EffectiveTypeName)
        Assert.Equal("x", tokens.[2].Value)

    [<Fact>]
    let defaultVersionMatchesExplicitVersion () =
        let defaultTokens = AlgolLexer.TokenizeAlgol("begin integer x; x := 42 end")
        let explicitTokens = AlgolLexer.TokenizeAlgol("begin integer x; x := 42 end", AlgolLexer.DefaultVersion)
        Assert.Equal(Seq.length defaultTokens, Seq.length explicitTokens)
        Assert.Equal(defaultTokens.[0].Value, explicitTokens.[0].Value)

    [<Fact>]
    let rejectsUnknownVersion () =
        let error = Assert.Throws<System.ArgumentException>(fun () -> AlgolLexer.TokenizeAlgol("begin integer x; x := 42 end", "algol68") |> ignore)
        Assert.Contains("Unknown ALGOL version", error.Message)
