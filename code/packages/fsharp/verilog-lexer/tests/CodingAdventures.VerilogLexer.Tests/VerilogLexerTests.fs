namespace CodingAdventures.VerilogLexer.Tests

open CodingAdventures.Lexer.FSharp
open CodingAdventures.VerilogLexer.FSharp
open Xunit

module VerilogLexerTests =
    [<Fact>]
    let tokenizesSample () =
        let tokens = VerilogLexer.TokenizeVerilog("module top; endmodule")

        Assert.Equal(TokenType.Keyword, tokens.[0].Type)
        Assert.Equal("module", tokens.[0].Value)
        Assert.Equal("NAME", tokens.[1].EffectiveTypeName)
        Assert.Equal("top", tokens.[1].Value)

    [<Fact>]
    let defaultVersionMatchesExplicitVersion () =
        let defaultTokens = VerilogLexer.TokenizeVerilog("module top; endmodule")
        let explicitTokens = VerilogLexer.TokenizeVerilog("module top; endmodule", VerilogLexer.DefaultVersion)
        Assert.Equal(Seq.length defaultTokens, Seq.length explicitTokens)
        Assert.Equal(defaultTokens.[0].Value, explicitTokens.[0].Value)

    [<Fact>]
    let rejectsUnknownVersion () =
        let error = Assert.Throws<System.ArgumentException>(fun () -> VerilogLexer.TokenizeVerilog("module top; endmodule", "2099") |> ignore)
        Assert.Contains("Unknown Verilog version", error.Message)
