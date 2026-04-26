using CodingAdventures.Lexer;

namespace CodingAdventures.VhdlLexer.Tests;

public sealed class VhdlLexerTests
{
    [Fact]
    public void TokenizesSample()
    {
        var tokens = CodingAdventures.VhdlLexer.VhdlLexer.TokenizeVhdl("ENTITY TOP IS END ENTITY TOP;");

        Assert.Equal(TokenType.Keyword, tokens[0].Type);
        Assert.Equal("entity", tokens[0].Value);
        Assert.Equal("NAME", tokens[1].EffectiveTypeName);
        Assert.Equal("top", tokens[1].Value);
    }

    [Fact]
    public void DefaultVersionMatchesExplicitVersion()
    {
        Assert.Equal(
            CodingAdventures.VhdlLexer.VhdlLexer.TokenizeVhdl("entity top is end entity top;"),
            CodingAdventures.VhdlLexer.VhdlLexer.TokenizeVhdl("entity top is end entity top;", CodingAdventures.VhdlLexer.VhdlLexer.DefaultVersion));
    }

    [Fact]
    public void RejectsUnknownVersion()
    {
        var error = Assert.Throws<ArgumentException>(() => CodingAdventures.VhdlLexer.VhdlLexer.TokenizeVhdl("entity top is end entity top;", "2099"));
        Assert.Contains("Unknown VHDL version", error.Message);
    }
}
