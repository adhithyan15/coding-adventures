using CodingAdventures.Lexer;

namespace CodingAdventures.VerilogLexer.Tests;

public sealed class VerilogLexerTests
{
    [Fact]
    public void TokenizesSample()
    {
        var tokens = CodingAdventures.VerilogLexer.VerilogLexer.TokenizeVerilog("module top; endmodule");

        Assert.Equal(TokenType.Keyword, tokens[0].Type);
        Assert.Equal("module", tokens[0].Value);
        Assert.Equal("NAME", tokens[1].EffectiveTypeName);
        Assert.Equal("top", tokens[1].Value);
    }

    [Fact]
    public void DefaultVersionMatchesExplicitVersion()
    {
        Assert.Equal(
            CodingAdventures.VerilogLexer.VerilogLexer.TokenizeVerilog("module top; endmodule"),
            CodingAdventures.VerilogLexer.VerilogLexer.TokenizeVerilog("module top; endmodule", CodingAdventures.VerilogLexer.VerilogLexer.DefaultVersion));
    }

    [Fact]
    public void RejectsUnknownVersion()
    {
        var error = Assert.Throws<ArgumentException>(() => CodingAdventures.VerilogLexer.VerilogLexer.TokenizeVerilog("module top; endmodule", "2099"));
        Assert.Contains("Unknown Verilog version", error.Message);
    }
}
