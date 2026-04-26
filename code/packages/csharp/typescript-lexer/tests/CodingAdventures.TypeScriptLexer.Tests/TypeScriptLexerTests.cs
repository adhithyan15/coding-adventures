namespace CodingAdventures.TypeScriptLexer.Tests;

public sealed class TypeScriptLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("typescript-lexer", new CodingAdventures.TypeScriptLexer.TypeScriptLexer().Ping());
    }
}