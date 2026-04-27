using CodingAdventures.Lexer;

namespace CodingAdventures.AlgolLexer.Tests;

public sealed class AlgolLexerTests
{
    [Fact]
    public void TokenizesSample()
    {
        var tokens = CodingAdventures.AlgolLexer.AlgolLexer.TokenizeAlgol("begin integer x; x := 42 end");

        Assert.Equal(TokenType.Keyword, tokens[0].Type);
        Assert.Equal("begin", tokens[0].Value);
        Assert.Equal("NAME", tokens[2].EffectiveTypeName);
        Assert.Equal("x", tokens[2].Value);
    }

    [Fact]
    public void DefaultVersionMatchesExplicitVersion()
    {
        Assert.Equal(
            CodingAdventures.AlgolLexer.AlgolLexer.TokenizeAlgol("begin integer x; x := 42 end"),
            CodingAdventures.AlgolLexer.AlgolLexer.TokenizeAlgol("begin integer x; x := 42 end", CodingAdventures.AlgolLexer.AlgolLexer.DefaultVersion));
    }

    [Fact]
    public void RejectsUnknownVersion()
    {
        var error = Assert.Throws<ArgumentException>(() => CodingAdventures.AlgolLexer.AlgolLexer.TokenizeAlgol("begin integer x; x := 42 end", "algol68"));
        Assert.Contains("Unknown ALGOL version", error.Message);
    }
}
