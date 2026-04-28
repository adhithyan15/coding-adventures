namespace CodingAdventures.StarlarkLexer.Tests;

public sealed class StarlarkLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("starlark-lexer", new CodingAdventures.StarlarkLexer.StarlarkLexer().Ping());
    }
}